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
use super::config::Config;
use crate::fix::{self as engine, Map};
use crate::lint::{Finding, Level, Lint};
use crate::render::{self, Change, Format, Skipped, Violation};
use crate::scan::{Hit, Unreadable, decode};
use crate::tokens;
use std::path::{Path, PathBuf};

/// The lint an unmapped character is named under: `check`'s, verbatim.
const OUTSIDE: &str = "outside-set";

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
    /// covers. V4: kept as they are, REPORTED, and the run exits 1 -- a
    /// silent drop is the one thing `fix` may never do, and an exit 1
    /// that names nothing drops the reason instead (B23).
    unmapped: Vec<Kept>,
    /// What a bare `fix` will write, held until EVERY file has been judged
    /// (V72): a refusal or a read error on the tenth file must not leave
    /// the first nine rewritten behind a run that reports only the error.
    pending: Vec<(PathBuf, String)>,
}

/// A character `fix` left in place, owned for the borrowed render row:
/// the row `check` would print for it, so the two verbs name it alike.
struct Kept {
    path: String,
    set: String,
    finding: Finding,
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
    config: &Config,
    paths: &[String],
    format: Format,
    write: bool,
) -> Result<Report, String> {
    let root = &config.root;
    let pass = Pass::new(config, write)?;
    let files = tokens::select(root, paths)
        .map_err(|bad| format!("{}: {}", bad.path.display(), bad.reason))?;
    let mut found = Found::default();
    for full in &files {
        let shown = super::check::shown_path(root, full);
        pass.visit(full, shown, &mut found)?;
    }
    written(&found.pending)?;
    Ok(report_of(&found, format, pass.write))
}

/// The second phase (V72): every file was judged and none refused, so
/// the rewrites go to disk.
fn written(pending: &[(PathBuf, String)]) -> Result<(), String> {
    for (full, text) in pending {
        std::fs::write(full, text)
            .map_err(|e| format!("{}: {e}", full.display()))?;
    }
    Ok(())
}

/// What one run holds for every file it visits, so a per-file call takes
/// the file and nothing else.
struct Pass {
    checker: Checker,
    map: Map,
    /// The lint an unmapped character is reported under, as in `check`.
    outside: Lint,
    write: bool,
}

impl Pass {
    fn new(config: &Config, write: bool) -> Result<Self, String> {
        Ok(Self {
            checker: Checker::configured(config)?,
            map: config.map()?,
            outside: Lint::named(OUTSIDE)
                .ok_or_else(|| format!("no `{OUTSIDE}` lint"))?,
            write,
        })
    }

    /// One file: judge it, rewrite it, queue the write when asked.
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
            found.pending.push((full.to_owned(), fixed.output));
        }
        let kept = self.kept(&shown, &set.name, &fixed.report.unmapped);
        found.unmapped.extend(kept);
        absorb(found, shown, fixed.report);
        Ok(())
    }

    /// Every unmapped character, as the `outside-set` row `check` gives
    /// it, at the default `deny`: `fix` judges sets, not levels.
    fn kept(&self, path: &str, set: &str, hits: &[Hit]) -> Vec<Kept> {
        let row = |hit: &Hit| Kept {
            path: path.to_owned(),
            set: set.to_owned(),
            finding: Finding {
                hit: *hit,
                lint: self.outside,
                level: Level::Deny,
            },
        };
        hits.iter().map(row).collect()
    }
}

/// A file's text, or `None` when it was skipped and named.
///
/// Classified by `check`'s own decode (`src/scan:V8`): a NUL anywhere is
/// binary, then UTF-8 is required. Named rather than dropped, the way
/// `check` names it: a file quietly missing from a rewrite run is a file
/// nobody knows was not rewritten, and a blob `check` skips but `fix`
/// rewrites is a file corrupted by the verb meant to clean it (B21).
fn text_of(
    full: &Path,
    shown: &str,
    found: &mut Found,
) -> Result<Option<String>, String> {
    let bytes =
        std::fs::read(full).map_err(|e| format!("{}: {e}", full.display()))?;
    match decode(&bytes) {
        Ok(text) => Ok(Some(text.to_owned())),
        Err(reason) => {
            found.skips.push(Skip {
                path: shown.to_owned(),
                reason,
            });
            Ok(None)
        }
    }
}

fn absorb(found: &mut Found, path: String, report: engine::Report) {
    for rewrite in report.rewrites {
        found.rows.push(Row {
            path: path.clone(),
            rewrite,
        });
    }
}

/// The report, and the code that goes with it.
///
/// `--check` GATES (V7): exit 1 on drift (the root spec's interface
/// section) or on a character no map entry covers (`src/fix:V4`). A bare
/// `fix` does not gate on what it just repaired: exit 1 only when an
/// unmapped character is LEFT, because then the tree is still not clean.
/// A run that cleaned everything exits 0. The `ctrm-fix` pre-commit hook
/// still refuses the commit: pre-commit fails any hook that modified files.
fn report_of(found: &Found, format: Format, write: bool) -> Report {
    let changes: Vec<Change<'_>> = found.rows.iter().map(change).collect();
    let skipped: Vec<Skipped<'_>> = found.skips.iter().map(skip).collect();
    let kept: Vec<Violation<'_>> = found.unmapped.iter().map(kept).collect();
    let drifted = !write && !found.rows.is_empty();
    let failed = drifted || !kept.is_empty();
    Report {
        text: render::fix(format, &changes, &kept, &skipped),
        code: u8::from(failed),
    }
}

fn change(row: &Row) -> Change<'_> {
    Change {
        path: &row.path,
        rewrite: row.rewrite.clone(),
    }
}

fn kept(held: &Kept) -> Violation<'_> {
    Violation {
        path: &held.path,
        finding: held.finding.clone(),
        set: &held.set,
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
    use super::{Config, run};
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
        match run(&Config::discovered(root), &asked, Format::Human, write) {
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
        // V7: a bare `fix` that left nothing behind exits 0. Only `check`
        // and `fix --check` gate, and the file IS clean now.
        assert_eq!(code, 0, "{text}");
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
        // B23: and NAMED, in `check`'s row grammar, so exit 1 says why.
        assert_eq!(text, "notes.md:1:3 U+2261 ascii");
    }

    /// B21: a file `check` skips as binary is skipped by `fix` too, and
    /// named. Before, only UTF-8 was asked, so a NUL-laden blob that
    /// happened to decode had its bytes rewritten.
    #[test]
    fn a_binary_file_is_skipped_and_named_not_rewritten() {
        let blob = "\0\0\u{2014}\u{FEFF}data\0";
        let files = [("notes.md", blob)];
        let Some(root) = fixture("ctrm-fix-binary-fixture", &files) else {
            return;
        };
        let (text, code) = ran(&root, true);
        assert_eq!(read(&root), blob, "{text}");
        assert_eq!(text, "notes.md: skipped, binary");
        assert_eq!(code, 0, "{text}");
    }

    /// V72: a run that fails on a later file writes NOTHING, not the files
    /// it judged before the failure (B25). The failure here is a file the
    /// run cannot read; as root it can, and then there is no failure.
    #[cfg(unix)]
    #[test]
    fn a_failed_run_writes_no_file_at_all() {
        let before = "a \u{2014} b\n";
        let files = [("notes.md", before), ("zz.md", "x\n")];
        let Some(root) = fixture("ctrm-fix-two-phase", &files) else {
            return;
        };
        let asked = ["notes.md".to_owned(), "zz.md".to_owned()];
        let config = Config::discovered(&root);
        mode(&root.join("zz.md"), 0o000);
        let failed = run(&config, &asked, Format::Human, true).is_err();
        mode(&root.join("zz.md"), 0o644);
        if failed {
            assert_eq!(read(&root), before);
        }
    }

    #[cfg(unix)]
    fn mode(path: &Path, bits: u32) {
        use std::os::unix::fs::PermissionsExt;
        let wanted = std::fs::Permissions::from_mode(bits);
        let _ = std::fs::set_permissions(path, wanted);
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

    /// `src/fix:V51` through the verb: the notation is left alone until a
    /// `.ctrm-map` line opts in, and then it becomes words that do not
    /// fuse with the letter beside them.
    #[test]
    fn the_words_map_rewrites_only_once_asked_for() {
        let before = "x \u{22A5}owns y\n";
        let files = [("notes.md", before)];
        let Some(root) = fixture("ctrm-fix-words-fixture", &files) else {
            return;
        };
        let _ = std::fs::remove_file(root.join(".ctrm-map"));
        let (text, _) = ran(&root, true);
        assert_eq!(read(&root), before, "{text}");
        let opted = std::fs::write(root.join(".ctrm-map"), "use words\n");
        assert!(opted.is_ok());
        let (text, _) = ran(&root, true);
        assert_eq!(read(&root), "x not owns y\n", "{text}");
    }
}
