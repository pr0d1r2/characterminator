//! The `fix` and `fix --check` verbs (T50): rewrite what the map covers.
//!
//! The engine is `src/fix`; this reads the files, hands each one the set
//! it is judged against, and writes back only when asked. The split is
//! V7's: `fix --check` GATES and writes nothing, bare `fix` rewrites and
//! is therefore something a caller invokes deliberately.
//!
//! Nothing here decides what a character becomes. That is the map's, and
//! the map is data (`src/fix:V26`) plus whatever `.ctrm-map` declares
//! over it (`src/rules:V45`).

use super::check::Checker;
use crate::fix::{self as engine, Map};
use crate::render::{self, Change, Format, Skipped};
use crate::rules::Origin;
use crate::scan::Unreadable;
use crate::tokens;
use std::path::Path;

/// The map file discovered at the run root (`src/rules:V45`).
const MAP: &str = ".ctrm-map";

/// What a run of `fix` produced.
pub struct Report {
    pub text: String,
    pub code: u8,
}

/// One file's rewrites, owned so the borrowed render rows can point at
/// them.
struct Row {
    path: String,
    rewrite: engine::Rewrite,
}

/// A file that could not be read as text, named rather than dropped.
struct Skip {
    path: String,
    reason: Unreadable,
}

/// What the walk accumulated.
#[derive(Default)]
struct Found {
    rows: Vec<Row>,
    skips: Vec<Skip>,
    /// Characters that are outside their set and that no map entry
    /// covers. V4: kept as they are, reported, and the run exits 1 --
    /// a silent drop is the one thing `fix` may never do.
    unmapped: usize,
}

/// Run `fix` over a repository.
///
/// `write` false is `--check`: the same walk, the same report, and no
/// file touched. The difference between the two verbs is whether the
/// write happened, not what is found.
///
/// # Errors
///
/// A configuration that cannot be read, a file that cannot be written,
/// or a rewrite the engine refused to trust (`src/fix:V5`, `src/fix:V6`).
pub fn run(
    root: &Path,
    paths: &[String],
    format: Format,
    write: bool,
) -> Result<Report, String> {
    let pass = Pass {
        checker: Checker::load(root)?,
        map: map_of(root)?,
        write,
    };
    let files = tokens::select(root, paths)
        .map_err(|bad| format!("{}: {}", bad.path.display(), bad.reason))?;
    let mut found = Found::default();
    for full in &files {
        let shown = super::check::shown_path(root, full);
        pass.visit(full, shown, &mut found)?;
    }
    Ok(report_of(&found, format))
}

/// The map in force: the builtin (`src/fix:V26`), then `.ctrm-map` over
/// it (`src/rules:V19`, `src/rules:V45`).
fn map_of(root: &Path) -> Result<Map, String> {
    let map = Map::parse(engine::BUILTIN, &|line| Origin::Builtin { line })
        .map_err(|bad| bad.to_string())?;
    let Ok(text) = std::fs::read_to_string(root.join(MAP)) else {
        return Ok(map);
    };
    map.layer(&text, &|line| Origin::File {
        path: MAP.into(),
        line,
    })
    .map_err(|bad| bad.to_string())
}

/// What one run holds for every file it visits, so a per-file call takes
/// the file and nothing else.
struct Pass {
    checker: Checker,
    map: Map,
    write: bool,
}

impl Pass {
    /// One file: judge it, rewrite it, write it back when asked.
    fn visit(
        &self,
        full: &Path,
        shown: String,
        found: &mut Found,
    ) -> Result<(), String> {
        let (set, _) = self.checker.effective(&shown)?;
        let Some(text) = text_of(full, &shown, found)? else {
            return Ok(());
        };
        let fixed = engine::fix(&text, &self.map, &|point| set.contains(point))
            .map_err(|bad| format!("{shown}: {bad}"))?;
        if self.write && fixed.output != text {
            std::fs::write(full, &fixed.output)
                .map_err(|e| format!("{}: {e}", full.display()))?;
        }
        absorb(found, shown, fixed.report);
        Ok(())
    }
}

/// A file's text, or `None` when it was skipped and named.
///
/// Named rather than dropped, the way `check` names it (`src/scan:V8`):
/// a file quietly missing from a rewrite run is a file nobody knows was
/// not rewritten.
fn text_of(
    full: &Path,
    shown: &str,
    found: &mut Found,
) -> Result<Option<String>, String> {
    let bytes =
        std::fs::read(full).map_err(|e| format!("{}: {e}", full.display()))?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(bad) => {
            found.skips.push(Skip {
                path: shown.to_owned(),
                reason: Unreadable::NotUtf8 {
                    byte: bad.utf8_error().valid_up_to(),
                },
            });
            Ok(None)
        }
    }
}

fn absorb(found: &mut Found, path: String, report: engine::Report) {
    found.unmapped = found.unmapped.saturating_add(report.unmapped.len());
    for rewrite in report.rewrites {
        found.rows.push(Row {
            path: path.clone(),
            rewrite,
        });
    }
}

/// The report, and the code that goes with it.
///
/// Exit 1 covers BOTH halves of the verb's contract: drift under
/// `--check` (the root spec's interface section) and a disallowed
/// character no map entry covers (`src/fix:V4`). A bare `fix` that
/// rewrote everything it could still exits 1 when something was left,
/// because the file is not yet clean and a zero would say it was.
fn report_of(found: &Found, format: Format) -> Report {
    let changes: Vec<Change<'_>> = found.rows.iter().map(change).collect();
    let skipped: Vec<Skipped<'_>> = found.skips.iter().map(skip).collect();
    let drifted = !found.rows.is_empty() || found.unmapped > 0;
    Report {
        text: render::fix(format, &changes, &skipped),
        code: u8::from(drifted),
    }
}

fn change(row: &Row) -> Change<'_> {
    Change {
        path: &row.path,
        rewrite: row.rewrite.clone(),
    }
}

fn skip(held: &Skip) -> Skipped<'_> {
    Skipped {
        path: &held.path,
        reason: held.reason,
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::render::Format;
    use std::path::{Path, PathBuf};

    /// A tree under `target/`, untracked by construction. The files are
    /// NAMED to `run`, so the git-tracked default never applies and the
    /// fixture does not have to be in the index.
    fn fixture(name: &str, files: &[(&str, &str)]) -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(name);
        std::fs::create_dir_all(&root).ok()?;
        for (file, text) in files {
            std::fs::write(root.join(file), text).ok()?;
        }
        Some(root)
    }

    fn ran(root: &Path, write: bool) -> (String, u8) {
        let asked = ["notes.md".to_owned()];
        match run(root, &asked, Format::Human, write) {
            Ok(report) => (report.text, report.code),
            Err(why) => (why, 9),
        }
    }

    fn read(root: &Path) -> String {
        std::fs::read_to_string(root.join("notes.md")).unwrap_or_default()
    }

    /// The builtin map, through the verb: an em dash and curly quotes
    /// become ASCII, and the file on disk changes.
    #[test]
    fn fix_rewrites_what_the_builtin_map_covers() {
        let files = [("notes.md", "a \u{2014} \u{201C}b\u{201D}\n")];
        let Some(root) = fixture("ctrm-fix-fixture", &files) else {
            return;
        };
        let (text, code) = ran(&root, true);
        assert_eq!(read(&root), "a -- \"b\"\n", "{text}");
        // Rewrites happened, so the file was not clean when the run
        // started: exit 1 says so even though it is clean now.
        assert_eq!(code, 1, "{text}");
    }

    /// `--check` reports the SAME thing and writes nothing, which is the
    /// half of V7 that makes it usable in a gate.
    #[test]
    fn fix_check_reports_the_drift_without_writing() {
        let before = "a \u{2014} b\n";
        let files = [("notes.md", before)];
        let Some(root) = fixture("ctrm-fixcheck-fixture", &files) else {
            return;
        };
        let (text, code) = ran(&root, false);
        assert_eq!(read(&root), before, "{text}");
        assert_eq!(code, 1, "{text}");
        assert!(text.contains("U+2014"), "{text}");
    }

    /// V6: a character the file is ALLOWED to hold is not touched, even
    /// when the map has an entry for it. The grant wins.
    #[test]
    fn an_allowed_character_is_left_alone() {
        let files = [
            (".ctrm", "*.md ascii+typography\n"),
            ("notes.md", "a \u{2014} b\n"),
        ];
        let Some(root) = fixture("ctrm-fix-allowed-fixture", &files) else {
            return;
        };
        let (text, code) = ran(&root, true);
        assert_eq!(read(&root), "a \u{2014} b\n", "{text}");
        assert_eq!(code, 0, "{text}");
    }

    /// V4: a disallowed character with no mapping is KEPT and reported,
    /// and the run exits 1 rather than claiming success.
    #[test]
    fn a_character_with_no_mapping_is_kept_and_still_fails() {
        // IDENTICAL TO: outside `ascii`, and no builtin entry covers it.
        let files = [("notes.md", "a \u{2261} b\n")];
        let Some(root) = fixture("ctrm-fix-unmapped-fixture", &files) else {
            return;
        };
        let (text, code) = ran(&root, true);
        assert_eq!(read(&root), "a \u{2261} b\n", "{text}");
        assert_eq!(code, 1, "{text}");
    }

    /// `.ctrm-map` is discovered beside `.ctrm` and wins over the builtin
    /// for the same source (`src/rules:V19`, `src/rules:V45`).
    #[test]
    fn a_declared_map_entry_beats_the_builtin() {
        let files =
            [(".ctrm-map", "U+2014 -\n"), ("notes.md", "a \u{2014} b\n")];
        let Some(root) = fixture("ctrm-fix-map-fixture", &files) else {
            return;
        };
        let (text, _) = ran(&root, true);
        assert_eq!(read(&root), "a - b\n", "{text}");
    }
}
