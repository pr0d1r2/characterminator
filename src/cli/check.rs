//! The `check` verb (T8): report every character outside its file's set.
//!
//! Composition only. Every answer comes from the node that owns it: which
//! rule governs a path (`src/rules`), what that rule's sets contain
//! (`src/charset`), where a character sits (`src/scan`), how loudly it is
//! said (`src/lint`), and how the report reads (`src/render`). V7 makes
//! this one of the two verbs that GATE.

use crate::charset::{CharSet, SetCatalog, builtin};
use crate::lint::{Finding, Level, Levels, Lint, Target, exit_code};
use crate::render::{self, Format, Skipped, Violation};
use crate::rules::{self, Resolution, Rule, Sources};
use crate::scan::{Hit, Unreadable, scan_bytes};
use crate::tokens;
use std::path::Path;

/// The rules file discovered at the repo root: the dotfile tier of the
/// precedence chain (`src/rules:V19`).
const CONFIG: &str = ".ctrm";

/// The lint a character outside its set is reported under. Named for what
/// is true of the character, not for its group (`src/lint` registry).
const OUTSIDE: &str = "outside-set";

/// One file's contribution to the report.
enum Looked {
    /// What was found, already levelled.
    Findings(Vec<Finding>),
    /// The file could not be read AS TEXT, and is named rather than
    /// dropped (`src/scan:V8`).
    Unread(Unreadable),
}

/// A violation with the strings it is reported against, owned so the
/// borrowed `render` rows can point at them.
struct Row {
    path: String,
    set: String,
    finding: Finding,
}

/// A file that was skipped, owned for the same reason.
struct Skip {
    path: String,
    reason: Unreadable,
}

/// A finished report: what to print, and what to exit with.
pub struct Report {
    pub text: String,
    pub code: u8,
}

/// The rules and the sets a run resolves against, read once.
pub struct Checker {
    rules: Vec<Rule>,
    catalog: SetCatalog,
}

impl Checker {
    /// Read `.ctrm` if it is there.
    ///
    /// Its ABSENCE is not an error: V1 gives an unmatched path `ascii`, so
    /// a repo that has never been configured is still checkable, and the
    /// answer it gets is the strict one.
    ///
    /// The catalog is the builtin one, so a rule may name a preset that
    /// ships as data (`src/charset:V22`) and not only the intrinsic set.
    ///
    /// # Errors
    ///
    /// A `.ctrm` that cannot be read, or a compiled-in preset that cannot
    /// be parsed -- the second is a defect in this crate rather than in
    /// the tree being checked, and it says so.
    pub fn load(root: &Path) -> Result<Self, String> {
        let rules = match std::fs::read_to_string(root.join(CONFIG)) {
            Ok(text) => parse(text)?,
            Err(_) => Vec::new(),
        };
        Ok(Self {
            rules,
            catalog: builtin::catalog().map_err(|e| e.to_string())?,
        })
    }

    /// What one path may contain, and how loudly a stray character there
    /// is reported.
    ///
    /// The union is named after the sets it came FROM, because that name
    /// lands in the json contract: `set: "effective"` would tell a reader
    /// nothing, while `ascii+caveman` says what the file was judged
    /// against and which rule to look for.
    fn law(&self, shown: &str) -> Result<(CharSet, Levels), String> {
        let found = rules::resolve(shown, &self.rules, &rules::matches);
        let set = self
            .catalog
            .resolve_union(&found.sets.join("+"), &found.sets)
            .map_err(|e| e.to_string())?;
        Ok((set, levels_for(&found)?))
    }
}

fn parse(text: String) -> Result<Vec<Rule>, String> {
    Sources::new()
        .dotfile(CONFIG, text)
        .rules()
        .map_err(|e| e.to_string())
}

/// The level directives of the winning rules, as the lint node's own
/// vocabulary. An unknown target is an ERROR: a misspelled lint name that
/// silently did nothing would read exactly like a rule that worked.
fn levels_for(found: &Resolution) -> Result<Levels, String> {
    let mut levels = Levels::new();
    if let Some(level) = found.default_level {
        levels.set_charset(level);
    }
    for choice in &found.levels {
        let target = Target::named(&choice.target)
            .ok_or_else(|| unknown(&choice.target))?;
        levels.set(target, choice.level);
    }
    Ok(levels)
}

fn unknown(target: &str) -> String {
    format!("unknown lint or group: {target}")
}

/// The findings in one file's bytes.
///
/// Pure, so every rule it encodes is testable without a filesystem: the
/// I/O lives in the caller and this decides what the bytes MEAN.
fn inspect(bytes: &[u8], set: &CharSet, levels: &Levels, lint: Lint) -> Looked {
    match scan_bytes(bytes, |c| set.contains(c)) {
        Err(reason) => Looked::Unread(reason),
        Ok(hits) => Looked::Findings(reportable(hits, levels, lint)),
    }
}

/// A finding at `allow` is not reported: the level system decides what is
/// worth saying, and `allow` is how a project says "not this one".
fn reportable(hits: Vec<Hit>, levels: &Levels, lint: Lint) -> Vec<Finding> {
    hits.into_iter()
        .map(|hit| levels.finding(hit, lint))
        .filter(|finding| finding.level != Level::Allow)
        .collect()
}

/// The path as a reader typed it: relative to the root, so it matches the
/// patterns in `.ctrm` and reads like the file they meant.
fn shown_path(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .unwrap_or(full)
        .to_string_lossy()
        .into_owned()
}

fn gather(
    checker: &Checker,
    root: &Path,
    paths: &[String],
    lint: Lint,
) -> Result<Found, String> {
    let files = tokens::select(root, paths)
        .map_err(|e| format!("{}: {}", e.path.display(), e.reason))?;
    let mut found = Found::default();
    for full in &files {
        let shown = shown_path(root, full);
        let (set, levels) = checker.law(&shown)?;
        let bytes = read(full)?;
        found.absorb(shown, &set, inspect(&bytes, &set, &levels, lint));
    }
    Ok(found)
}

fn read(full: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(full).map_err(|e| format!("{}: {e}", full.display()))
}

/// What the walk accumulated.
#[derive(Default)]
struct Found {
    rows: Vec<Row>,
    skips: Vec<Skip>,
}

impl Found {
    fn absorb(&mut self, path: String, set: &CharSet, looked: Looked) {
        match looked {
            Looked::Unread(reason) => self.skips.push(Skip { path, reason }),
            Looked::Findings(findings) => {
                for finding in findings {
                    self.rows.push(Row {
                        path: path.clone(),
                        set: set.name.clone(),
                        finding,
                    });
                }
            }
        }
    }
}

/// Run `check` over a repository.
///
/// `paths` empty means the git-tracked fileset; naming paths reaches
/// untracked files too (`src/tokens:V9`).
pub fn run(
    root: &Path,
    paths: &[String],
    format: Format,
) -> Result<Report, String> {
    let checker = Checker::load(root)?;
    let lint = Lint::named(OUTSIDE)
        .ok_or_else(|| String::from("no `outside-set` lint registered"))?;
    let found = gather(&checker, root, paths, lint)?;
    let violations: Vec<Violation<'_>> = found.rows.iter().map(row).collect();
    let skipped: Vec<Skipped<'_>> = found.skips.iter().map(skip).collect();
    let findings: Vec<Finding> =
        found.rows.iter().map(|r| r.finding.clone()).collect();
    Ok(Report {
        text: render::check(format, &violations, &skipped),
        code: exit_code(&findings),
    })
}

fn row(source: &Row) -> Violation<'_> {
    Violation {
        path: &source.path,
        finding: source.finding.clone(),
        set: &source.set,
    }
}

fn skip(source: &Skip) -> Skipped<'_> {
    Skipped {
        path: &source.path,
        reason: source.reason,
    }
}

#[cfg(test)]
mod tests {
    use super::{Checker, Looked, OUTSIDE, inspect, levels_for, shown_path};
    use crate::charset::{CharSet, builtin};
    use crate::lint::{Level, Levels, Lint};
    use crate::rules::{self, Rule};
    use std::path::{Path, PathBuf};

    fn lint() -> Lint {
        match Lint::named(OUTSIDE) {
            Some(found) => found,
            None => Lint::named("outside-set").unwrap_or_else(|| {
                unreachable!("the registry always carries this lint")
            }),
        }
    }

    fn found(bytes: &[u8]) -> Looked {
        inspect(bytes, &builtin::ascii(), &Levels::new(), lint())
    }

    #[test]
    fn ascii_text_reports_nothing() {
        match found(b"plain ascii, nothing to say\n") {
            Looked::Findings(findings) => assert!(findings.is_empty()),
            Looked::Unread(_) => unreachable!("this text is readable"),
        }
    }

    #[test]
    fn a_character_outside_the_set_is_found_where_it_sits() {
        // An em dash on the second line, after four characters.
        let text = "ok\nab c\u{2014}d\n";
        match found(text.as_bytes()) {
            Looked::Findings(findings) => {
                assert_eq!(findings.len(), 1);
                let first = findings.first().cloned();
                let one = first.unwrap_or_else(|| unreachable!("one finding"));
                assert_eq!(one.hit.character, '\u{2014}');
                assert_eq!(one.hit.position.line, 2);
                assert_eq!(one.hit.position.column, 5);
                // The level lives on the FINDING, not on the hit: the
                // scanner reports what is where, the lint node says how
                // loudly, and `charset` denies by default.
                assert_eq!(one.level, Level::Deny);
            }
            Looked::Unread(_) => unreachable!("this text is readable"),
        }
    }

    #[test]
    fn a_binary_file_is_skipped_rather_than_reported() {
        match found(b"text\0more") {
            Looked::Unread(_) => (),
            Looked::Findings(_) => unreachable!("a NUL means binary"),
        }
    }

    #[test]
    fn an_unmatched_path_is_governed_by_ascii() {
        // V1: no rule means `ascii`, so the strict default needs no file.
        let resolution =
            rules::resolve("a.md", &[] as &[Rule], &rules::matches);
        assert_eq!(resolution.sets, vec![String::from("ascii")]);
    }

    #[test]
    fn a_level_directive_naming_nothing_is_an_error() {
        // A misspelled lint that silently did nothing would read exactly
        // like a rule that worked.
        let rules = match rules::parse_rule(
            "*.md !nosuchlint=warn",
            rules::Origin::Builtin { line: 1 },
        ) {
            Ok(rule) => vec![rule],
            Err(_) => unreachable!("the line parses; the TARGET is unknown"),
        };
        let resolution = rules::resolve("a.md", &rules, &rules::matches);
        assert!(levels_for(&resolution).is_err());
    }

    #[test]
    fn a_path_is_shown_relative_to_the_root() {
        let root = Path::new("/repo");
        assert_eq!(shown_path(root, Path::new("/repo/src/a.rs")), "src/a.rs");
        assert_eq!(
            shown_path(root, Path::new("/elsewhere/a.rs")),
            "/elsewhere/a.rs"
        );
    }

    /// A tree whose `.ctrm` grants a preset, or `None` if it cannot be
    /// written: a test should not fail for the disk's reasons.
    ///
    /// It lives under `target/`, which `.gitignore` excludes, so it is
    /// untracked by construction rather than by hoping.
    fn preset_fixture() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("ctrm-preset-fixture");
        std::fs::create_dir_all(&root).ok()?;
        std::fs::write(root.join(".ctrm"), "*.md ascii+caveman\n").ok()?;
        Some(root)
    }

    /// The set one path is judged against, in a tree of our own.
    fn granted(root: &Path, shown: &str) -> Result<CharSet, String> {
        Checker::load(root)?.law(shown).map(|(set, _)| set)
    }

    /// A rule may name a preset that ships as DATA, not only the
    /// intrinsic set: `src/charset:V22` compiles the preset file in, and
    /// this is the path that makes it reachable from a `.ctrm` line.
    #[test]
    fn a_rule_may_grant_a_preset_that_ships_as_data() {
        let Some(root) = preset_fixture() else {
            return;
        };
        let found = granted(&root, "notes.md");
        let why = found.as_ref().err().cloned().unwrap_or_default();
        let set = found.unwrap_or_else(|_| unreachable!("{why}"));
        assert_eq!(set.name, "ascii+caveman");
        // RIGHTWARDS ARROW, granted by the preset, and EM DASH, which is
        // the map's business rather than a grant (`src/fix:V26`).
        assert!(set.contains('\u{2192}'));
        assert!(!set.contains('\u{2014}'));
    }
}
