//! The `check` verb (T8): report every character outside its file's set.
//!
//! Composition only. Every answer comes from the node that owns it: which
//! rule governs a path (`src/rules`), what that rule's sets contain
//! (`src/charset`), where a character sits (`src/scan`), how loudly it is
//! said (`src/lint`), and how the report reads (`src/render`). V7 makes
//! this one of the two verbs that GATE.

use crate::charset::{self, CharSet, SetCatalog, SetDefinition, builtin};
use crate::lint::{
    Finding, Group, Hazards, Level, Levels, Lint, Target, exit_code,
};
use crate::render::{self, Format, Skipped, Violation};
use crate::rules::{self, Resolution, Rule, Sources};
use crate::scan::{Hit, Unreadable, scan_bytes};
use crate::tokens;
use std::path::Path;

/// The rules file discovered at the repo root: the dotfile tier of the
/// precedence chain (`src/rules:V19`).
const CONFIG: &str = ".ctrm";

/// The sets file discovered beside it (`src/rules:V45`).
const SETS: &str = ".ctrm-sets";

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

/// The rules and the sets a run resolves against, read once, and the
/// lints `check` reports under.
pub struct Checker {
    rules: Vec<Rule>,
    catalog: SetCatalog,
    /// The hazard classes, from the COMPILED-IN data (`src/lint:V34`) and
    /// never from `catalog`: nothing a run configures can reach them.
    hazards: Hazards,
    outside: Lint,
}

/// What one file's characters are judged against: its set, and the
/// hazards that fire whatever the set says.
struct Judge<'a> {
    set: &'a CharSet,
    hazards: &'a Hazards,
    outside: Lint,
}

impl Judge<'_> {
    /// Whether the scan may walk past a character without stopping.
    ///
    /// A HAZARD STOPS IT EVEN WHEN THE SET GRANTS IT. That is V34's whole
    /// point: `any` grants every code point, and the exclusion lives in
    /// the level, so the set alone cannot be what decides.
    fn passes(&self, character: char) -> bool {
        self.set.contains(character) && !self.hazards.contains(character)
    }

    /// The lint one hit fires, if any. A hazard wins over `outside-set`,
    /// so a character that is both is reported ONCE, at the louder level.
    /// `None` is the byte order mark at byte 0 in a file whose set grants
    /// it: no hazard (V34), and not outside the set either.
    fn lint_for(&self, hit: Hit) -> Option<Lint> {
        self.hazards.lint_for(hit).or_else(|| {
            (!self.set.contains(hit.character)).then_some(self.outside)
        })
    }
}

impl Checker {
    /// Read `.ctrm` if it is there.
    ///
    /// Its ABSENCE is not an error: V1 gives an unmatched path `ascii`, so
    /// a repo that has never been configured is still checkable, and the
    /// answer it gets is the strict one.
    ///
    /// `.ctrm-sets` is read the same way, so a repo may declare a set of
    /// its own and a rule may name it (`src/rules:V45`).
    ///
    /// # Errors
    ///
    /// A discovered file that cannot be parsed, or a compiled-in preset
    /// that cannot be -- the second is a defect in this crate rather than
    /// in the tree being checked, and it says so.
    pub fn load(root: &Path) -> Result<Self, String> {
        let rules = match std::fs::read_to_string(root.join(CONFIG)) {
            Ok(text) => parse(text)?,
            Err(_) => Vec::new(),
        };
        Ok(Self {
            rules,
            catalog: catalog(root)?,
            hazards: Hazards::builtin()?,
            outside: Lint::named(OUTSIDE)
                .ok_or_else(|| String::from("no `outside-set` lint"))?,
        })
    }

    /// What one file's characters are judged against.
    fn judge<'a>(&'a self, set: &'a CharSet) -> Judge<'a> {
        Judge {
            set,
            hazards: &self.hazards,
            outside: self.outside,
        }
    }

    /// What one path may contain, and how loudly a stray character there
    /// is reported.
    fn law(&self, shown: &str) -> Result<(CharSet, Levels), String> {
        let found = rules::resolve(shown, &self.rules, &rules::matches);
        Ok((self.granted(&found)?, levels_for(&found)?))
    }

    /// What one path may contain, and the rule that decided it -- which
    /// is `explain`'s whole question (`src/rules:V2`).
    ///
    /// The winner is CLONED rather than borrowed: it is one small struct,
    /// and handing back a reference would tie every caller's lifetime to
    /// this checker for no gain.
    ///
    /// # Errors
    ///
    /// A rule naming a set nothing declares, or one that cycles.
    pub fn effective(
        &self,
        shown: &str,
    ) -> Result<(CharSet, Option<Rule>), String> {
        let found = rules::resolve(shown, &self.rules, &rules::matches);
        Ok((self.granted(&found)?, found.winner.cloned()))
    }

    /// Every declared set, resolved at one fidelity -- `sets`' answer.
    ///
    /// # Errors
    ///
    /// A declared set that cannot be resolved: a cycle, or a member
    /// naming a set nothing declares.
    pub fn declared(&self, family: &str) -> Result<Vec<CharSet>, String> {
        self.catalog
            .names()
            .map(|name| self.catalog.resolve(name, family))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|bad| bad.to_string())
    }

    /// The union a resolution grants, named after the sets it came FROM.
    ///
    /// That name lands in the json contract: `set: "effective"` would
    /// tell a reader nothing, while `ascii+caveman` says what the file
    /// was judged against and which rule to look for.
    fn granted(&self, found: &Resolution<'_>) -> Result<CharSet, String> {
        self.catalog
            .resolve_union(&found.sets.join("+"), &found.sets, &found.family)
            .map_err(|bad| bad.to_string())
    }
}

fn parse(text: String) -> Result<Vec<Rule>, String> {
    Sources::new()
        .dotfile(CONFIG, text)
        .rules()
        .map_err(|e| e.to_string())
}

/// The sets a run resolves against: the compiled-in presets, then what
/// `.ctrm-sets` declares over them (`src/rules:V19`, `src/rules:V45`).
///
/// A repo whose files hold characters no preset covers declares a set of
/// its own rather than reaching for `any`, which is the difference
/// between a grant somebody wrote down and a check switched off.
fn catalog(root: &Path) -> Result<SetCatalog, String> {
    let mut sources = Sources::new().builtin(builtin::SETS);
    if let Ok(text) = std::fs::read_to_string(root.join(SETS)) {
        sources = sources.dotfile(SETS, text);
    }
    let declared = sources.assemble(set_line).map_err(|e| e.to_string())?;
    let mut catalog = builtin::intrinsic_catalog();
    for definition in declared {
        catalog.insert(definition);
    }
    Ok(catalog)
}

/// One `.ctrm-sets` line, at the origin the chain gave it.
///
/// The grammar's parser belongs to `src/charset` and the precedence chain
/// to `src/rules`, and neither calls the other: the parser travels as an
/// argument, so this adapter is the one place their two error types meet.
fn set_line(
    line: &str,
    origin: rules::Origin,
) -> Result<SetDefinition, rules::ParseError> {
    match charset::parse_line(line) {
        Ok(Some(declared)) => Ok(declared),
        // The chain skips blank and comment lines before calling this, so
        // a line declaring nothing cannot arrive here.
        Ok(None) => Err(rules::error(origin, "declares no set")),
        Err(bad) => Err(rules::error(origin, bad.to_string())),
    }
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
fn inspect(bytes: &[u8], judge: &Judge<'_>, levels: &Levels) -> Looked {
    match scan_bytes(bytes, |c| judge.passes(c)) {
        Err(reason) => Looked::Unread(reason),
        Ok(hits) => Looked::Findings(reportable(hits, judge, levels)),
    }
}

/// A finding at `allow` is not reported: the level system decides what is
/// worth saying, and `allow` is how a project says "not this one". A
/// hazard never reaches `allow`, because no rule lowers a forbid (V36).
fn reportable(
    hits: Vec<Hit>,
    judge: &Judge<'_>,
    levels: &Levels,
) -> Vec<Finding> {
    hits.into_iter()
        .filter_map(|hit| judge.lint_for(hit).map(|l| levels.finding(hit, l)))
        .filter(|finding| finding.level != Level::Allow)
        .collect()
}

/// The path as a reader typed it: relative to the root, so it matches the
/// patterns in `.ctrm` and reads like the file they meant.
pub(super) fn shown_path(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .unwrap_or(full)
        .to_string_lossy()
        .into_owned()
}

fn gather(
    checker: &Checker,
    root: &Path,
    paths: &[String],
) -> Result<Found, String> {
    let files = tokens::select(root, paths)
        .map_err(|e| format!("{}: {}", e.path.display(), e.reason))?;
    let mut found = Found::default();
    for full in &files {
        let shown = shown_path(root, full);
        let (set, levels) = checker.law(&shown)?;
        let looked = inspect(&read(full)?, &checker.judge(&set), &levels);
        found.absorb(shown, &set, looked);
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
                        set: judged_against(&finding, set),
                        finding,
                    });
                }
            }
        }
    }
}

/// The set a finding names in the report.
///
/// A hazard names `hazard`, not the file's set: the set did not decide it
/// (V34), and `notes.md:1:1 U+202E any` would read as though the override
/// fell outside `any`, which is nonsense. Which hazard it was is the
/// lint's name, carried in the json.
fn judged_against(finding: &Finding, set: &CharSet) -> String {
    if finding.lint.group == Group::Hazard {
        String::from(Group::Hazard.name())
    } else {
        set.name.clone()
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
    let found = gather(&checker, root, paths)?;
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
    use super::{
        Checker, Judge, Looked, OUTSIDE, inspect, levels_for, run, shown_path,
    };
    use crate::charset::{CharSet, builtin};
    use crate::lint::{Finding, Group, Hazards, Level, Levels, Lint};
    use crate::render::Format;
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

    fn hazards() -> Hazards {
        Hazards::builtin().unwrap_or_else(|why| unreachable!("{why}"))
    }

    /// What `check` finds in `bytes` when the file is granted `set`.
    fn found_in(bytes: &[u8], set: &CharSet) -> Looked {
        let hazards = hazards();
        let judge = Judge {
            set,
            hazards: &hazards,
            outside: lint(),
        };
        inspect(bytes, &judge, &Levels::new())
    }

    fn found(bytes: &[u8]) -> Looked {
        found_in(bytes, &builtin::ascii())
    }

    /// The findings, as `(lint name, level, byte)`, for a text that must
    /// be readable.
    fn findings(
        text: &str,
        set: &CharSet,
    ) -> Vec<(&'static str, Level, usize)> {
        match found_in(text.as_bytes(), set) {
            Looked::Findings(all) => all.iter().map(summary).collect(),
            Looked::Unread(_) => unreachable!("this text is readable"),
        }
    }

    fn summary(finding: &Finding) -> (&'static str, Level, usize) {
        let at = finding.hit.position.byte;
        (finding.lint.name, finding.level, at)
    }

    /// `any`, as the builtin catalog resolves it.
    fn any() -> CharSet {
        builtin::catalog()
            .ok()
            .and_then(|sets| sets.resolve("any", "text").ok())
            .unwrap_or_else(|| unreachable!("`any` ships as a preset"))
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

    /// A tree carrying the dotfiles named, or `None` if it cannot be
    /// written: a test should not fail for the disk's reasons.
    ///
    /// It lives under `target/`, which `.gitignore` excludes, so it is
    /// untracked by construction rather than by hoping, and each caller
    /// names its own directory so two tests never share one tree.
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

    fn preset_fixture() -> Option<PathBuf> {
        fixture("ctrm-preset-fixture", &[(".ctrm", "*.md ascii+caveman\n")])
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

    /// `.ctrm-sets` is DISCOVERED beside `.ctrm` (`src/rules:V45`), so a
    /// repo whose files hold characters no preset covers declares a set
    /// rather than reaching for `any`.
    #[test]
    fn a_declared_set_is_discovered_beside_the_rules() {
        // IDENTICAL TO, which no builtin preset grants.
        let files = [
            (".ctrm", "*.md ascii+house\n"),
            (".ctrm-sets", "house U+2261\n"),
        ];
        let Some(root) = fixture("ctrm-declared-fixture", &files) else {
            return;
        };
        let found = granted(&root, "notes.md");
        let why = found.as_ref().err().cloned().unwrap_or_default();
        let set = found.unwrap_or_else(|_| unreachable!("{why}"));
        assert!(set.contains('\u{2261}'));
    }

    /// A declared set REPLACES the builtin of the same name (V19), which
    /// is what "later wins" means for a set, and is how a repo narrows a
    /// preset it finds too generous.
    #[test]
    fn a_declared_set_replaces_the_builtin_it_renames() {
        let files = [
            (".ctrm", "*.md ascii+legal\n"),
            // The builtin `legal` also grants REGISTERED and TRADE MARK.
            (".ctrm-sets", "legal U+00A9\n"),
        ];
        let Some(root) = fixture("ctrm-override-fixture", &files) else {
            return;
        };
        let set = granted(&root, "notes.md").unwrap_or_else(|why| {
            unreachable!("{why}");
        });
        assert!(set.contains('\u{00A9}'));
        assert!(!set.contains('\u{00AE}'));
    }

    /// A `.ctrm-sets` line that cannot be read names the FILE and LINE,
    /// not just the complaint: the origin travels with the error
    /// (`src/rules:V20`).
    #[test]
    fn a_bad_declared_line_is_reported_at_its_origin() {
        let files = [(".ctrm-sets", "\n\nbad U+ZZZZ\n")];
        let Some(root) = fixture("ctrm-badset-fixture", &files) else {
            return;
        };
        let why = Checker::load(&root).err().unwrap_or_default();
        assert!(why.contains(".ctrm-sets:3"), "{why}");
        assert!(why.contains("U+ZZZZ"), "{why}");
    }
    /// The whole of V41 through the real path: two rules name the SAME
    /// preset, and the fidelity each one chose decides what it grants.
    #[test]
    fn one_preset_grants_differently_under_two_fidelities() {
        let rules = "*.md ascii+marks\ndocs/*.md ascii+marks @emoji\n";
        let Some(root) = fixture("ctrm-fidelity-fixture", &[(".ctrm", rules)])
        else {
            return;
        };
        let plain = granted(&root, "notes.md").unwrap_or_else(|why| {
            unreachable!("{why}");
        });
        let rich = granted(&root, "docs/notes.md").unwrap_or_else(|why| {
            unreachable!("{why}");
        });
        // CHECK MARK at `text`, WHITE HEAVY CHECK MARK at `emoji`.
        assert!(plain.contains('\u{2713}') && !plain.contains('\u{2705}'));
        assert!(rich.contains('\u{2705}') && !rich.contains('\u{2713}'));
        // WARNING SIGN carries no label, so both spellings hold it.
        assert!(plain.contains('\u{26A0}') && rich.contains('\u{26A0}'));
    }

    /// Trojan Source (CVE-2021-42574) in a file granted EVERYTHING: a
    /// RIGHT-TO-LEFT OVERRIDE and an isolate pair around a condition. The
    /// set grants them; the level is what fires (V34).
    #[test]
    fn a_trojan_source_override_fires_under_any() {
        let text = "x = \"\u{202E} }\u{2066}if ok\u{2069} {\";\n";
        let fired = findings(text, &any());
        assert_eq!(fired.len(), 3);
        for (name, level, _) in fired {
            assert_eq!((name, level), ("bidi-control", Level::Forbid));
        }
    }

    /// ASCII smuggling: TAG LATIN CAPITAL LETTER A and B, invisible to a
    /// reader and legible to a model, in a file granted everything.
    #[test]
    fn tag_letters_fire_under_any() {
        let fired = findings("hi\u{E0041}\u{E0042}\n", &any());
        let tag = ("tag-character", Level::Forbid);
        assert_eq!(fired, vec![(tag.0, tag.1, 2), (tag.0, tag.1, 6)]);
    }

    /// The other three classes, one each, all under `any`.
    #[test]
    fn a_control_a_stray_bom_and_an_invisible_fire_under_any() {
        let fired = findings("a\u{001B}b\u{FEFF}c\u{200B}\n", &any());
        let names: Vec<&str> = fired.iter().map(|(n, _, _)| *n).collect();
        assert_eq!(names, ["control-character", "stray-bom", "invisible"]);
        assert!(fired.iter().all(|(_, level, _)| *level == Level::Forbid));
    }

    /// At byte 0 the BOM is a signature and no hazard: under `any` it is
    /// nothing at all, and under `ascii` it is the ordinary charset
    /// finding, at the ordinary level.
    #[test]
    fn a_bom_at_the_start_is_judged_by_the_set_alone() {
        assert_eq!(findings("\u{FEFF}hello\n", &any()), vec![]);
        let ascii = findings("\u{FEFF}hello\n", &builtin::ascii());
        assert_eq!(ascii, vec![("outside-set", Level::Deny, 0)]);
    }

    /// A character that is both a hazard and outside the set is reported
    /// ONCE, as the hazard, at the louder level.
    #[test]
    fn a_hazard_outside_the_set_is_reported_once_as_the_hazard() {
        let fired = findings("a\u{200B}\n", &builtin::ascii());
        assert_eq!(fired, vec![("invisible", Level::Forbid, 1)]);
    }

    /// The whole path, through a real `.ctrm`: a rule that grants `any`
    /// and tries to allow the group, the lint, and the charset findings,
    /// plus a `.ctrm-sets` that redeclares the class as harmless. None of
    /// it lowers the forbid (V36), and the run fails.
    #[test]
    fn no_configuration_talks_a_hazard_down() {
        let files = [
            (".ctrm", "* any !hazard=allow !invisible=allow !allow\n"),
            (".ctrm-sets", "hazard-invisible U+0041\n"),
            ("smuggled.txt", "fine\u{200B}\n"),
        ];
        let Some(root) = fixture("ctrm-hazard-fixture", &files) else {
            return;
        };
        let paths = [String::from("smuggled.txt")];
        let report = run(&root, &paths, Format::Json);
        let report = report.unwrap_or_else(|why| unreachable!("{why}"));
        assert_eq!(report.code, 1, "{}", report.text);
        assert!(report.text.contains("\"invisible\""), "{}", report.text);
        assert!(report.text.contains("\"forbid\""), "{}", report.text);
    }

    /// The human row names the `hazard` set, not the file's: the set did
    /// not decide a hazard, and `any` in that column would read as though
    /// the override fell outside it.
    #[test]
    fn a_hazard_row_names_the_hazard_set() {
        let files = [(".ctrm", "* any\n"), ("trojan.rs", "a\u{202E}b\n")];
        let Some(root) = fixture("ctrm-hazard-row-fixture", &files) else {
            return;
        };
        let paths = [String::from("trojan.rs")];
        let report = run(&root, &paths, Format::Human);
        let text = report.map(|r| r.text).unwrap_or_else(|why| why);
        assert_eq!(text, "trojan.rs:1:2 U+202E hazard");
        assert_eq!(Group::Hazard.name(), "hazard");
    }
}
