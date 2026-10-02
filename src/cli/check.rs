//! The `check` verb (T8): report every character outside its file's set.
//!
//! Composition only. Every answer comes from the node that owns it: which
//! rule governs a path (`src/rules`), what that rule's sets contain
//! (`src/charset`), where a character sits (`src/scan`), how loudly it is
//! said (`src/lint`), and how the report reads (`src/render`). V7 makes
//! this one of the two verbs that GATE.

use super::config::Config;
use crate::charset::{CharSet, SetCatalog};
use crate::lint::{
    CHAR_LINTS, CRLF, Finding, Group, Hazards, LINE_LINTS, Level, Levels, Lint,
    TEXT_LINTS, Target, char_lints, exit_code, line_hits, text_hits,
    unicode_space,
};
use crate::render::{self, Format, Skipped, Violation};
use crate::rules::{self, Resolution, Rule};
use crate::scan::{Hit, Unreadable, decode, scan_str};
use crate::tokens;
use std::path::Path;

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
    /// `--strict`: warn counts as deny (`src/lint:V36`).
    strict: bool,
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
    ///
    /// A pedantic character stops it too, whatever its level: whether
    /// that lint speaks is the level's question, asked once in `loudest`.
    /// `deep` is whether any table-reading one is asked for; without it
    /// only the cheap space list is consulted, so a run that did not ask
    /// pays no Unicode lookup per character (`src/lint:V58`).
    fn passes(&self, character: char, deep: bool) -> bool {
        self.set.contains(character)
            && !self.hazard(character)
            && unicode_space(character).is_none()
            && !(deep && char_lints(character).iter().any(Option::is_some))
    }

    /// Whether a character is a hazard IN THIS FILE: a joiner the file's
    /// own script preset grants is spelling, not a hazard (`src/lint:V57`).
    fn hazard(&self, character: char) -> bool {
        self.hazards.contains(character)
            && !self.hazards.excuses(&self.set.name, character)
    }

    /// The lints one hit could fire, the strongest claim first: a hazard,
    /// then `outside-set`, then the pedantic ones a character fires alone
    /// (`src/lint:V55`, `src/lint:V58`). Empty is the byte order mark at
    /// byte 0 in a file whose set grants it: no hazard (V34), and not
    /// outside the set either.
    fn lints_for(&self, hit: Hit) -> Vec<Lint> {
        let excused = self.hazards.excuses(&self.set.name, hit.character);
        let hazard = self.hazards.lint_for(hit).filter(|_| !excused);
        let outside =
            (!self.set.contains(hit.character)).then_some(self.outside);
        let pedantic = char_lints(hit.character);
        [hazard, outside]
            .into_iter()
            .chain(pedantic)
            .flatten()
            .collect()
    }
}

impl Checker {
    /// The checker for a configuration built from argv (`src/cli:T55`).
    ///
    /// A configuration with no rule at all is not an error: V1 gives an
    /// unmatched path `ascii`, so a repo that has never been configured
    /// is still checkable, and the answer it gets is the strict one.
    ///
    /// # Errors
    ///
    /// A line that cannot be parsed, named at its origin, or a compiled-in
    /// preset that cannot be -- the second is a defect in this crate
    /// rather than in the tree being checked, and it says so.
    pub fn configured(config: &Config) -> Result<Self, String> {
        Ok(Self {
            rules: config.rules()?,
            catalog: config.catalog()?,
            hazards: Hazards::builtin()?,
            outside: Lint::named(OUTSIDE)
                .ok_or_else(|| String::from("no `outside-set` lint"))?,
            strict: config.strict,
        })
    }

    /// The checker `sets` lists from: every locale set in, not only the
    /// ones the rules name (`src/charset:V61`).
    ///
    /// # Errors
    ///
    /// As [`Checker::configured`].
    pub fn listing(config: &Config) -> Result<Self, String> {
        let catalog = config.listing()?;
        Ok(Self {
            catalog,
            ..Self::configured(config)?
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
        let mut levels = levels_for(&found)?;
        levels.set_strict(self.strict);
        Ok((self.granted(&found)?, levels))
    }

    /// One file's findings, judged exactly as `check` judges it, so the
    /// guard and the gate cannot disagree about the same bytes (V35).
    /// `None` is a file that is not text: `check` names it as a skip.
    ///
    /// # Errors
    ///
    /// A rule naming a set nothing declares, or a lint nothing registers.
    pub(super) fn findings(
        &self,
        shown: &str,
        bytes: &[u8],
    ) -> Result<Option<Vec<Finding>>, String> {
        let (set, levels) = self.law(shown)?;
        match inspect(bytes, &self.judge(&set), &levels) {
            Looked::Findings(found) => Ok(Some(found)),
            Looked::Unread(_) => Ok(None),
        }
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
    match decode(bytes) {
        Err(reason) => Looked::Unread(reason),
        Ok(text) => Looked::Findings(findings_in(text, judge, levels)),
    }
}

/// The character findings, then the line-shaped ones. Two lists rather
/// than one merged walk; the report's order is `src/render`'s to impose
/// (by path, then byte), so nothing here has to interleave them.
fn findings_in(text: &str, judge: &Judge<'_>, levels: &Levels) -> Vec<Finding> {
    let deep = asked(&CHAR_LINTS, levels);
    let hits = scan_str(text, |c| judge.passes(c, deep));
    let mut found = reportable(hits, judge, levels);
    found.extend(context_findings(text, judge, levels));
    found
}

/// Whether a run asked for any of these lints: none at `allow` is the
/// common case, and it is what lets a run skip their work entirely.
fn asked(lints: &[Lint], levels: &Levels) -> bool {
    lints.iter().any(|l| levels.level_of(*l) != Level::Allow)
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
        .filter_map(|hit| loudest(hit, judge, levels))
        .collect()
}

/// ONE finding per character at most: the first lint that claims it and
/// is not at `allow`. So a no-break space outside `ascii` is reported as
/// `outside-set` and not twice, and still reported as `unicode-space` in
/// a file whose rule allowed the charset finding but asked for pedantic.
fn loudest(hit: Hit, judge: &Judge<'_>, levels: &Levels) -> Option<Finding> {
    judge
        .lints_for(hit)
        .into_iter()
        .map(|lint| levels.finding(hit, lint))
        .find(|finding| finding.level != Level::Allow)
}

/// The pedantic findings one character cannot decide alone: line-shaped
/// (`src/lint:V55`) and text-shaped (`src/lint:V58`). Neither walk runs
/// when every lint it serves is at `allow` -- which is every run that did
/// not ask for pedantic. Two text findings on one character are one: the
/// first, `not-nfc`.
fn context_findings(
    text: &str,
    judge: &Judge<'_>,
    levels: &Levels,
) -> Vec<Finding> {
    let mut found = Vec::new();
    if asked(&LINE_LINTS, levels) {
        found.extend(heard(line_hits(text), judge, levels));
    }
    if asked(&TEXT_LINTS, levels) {
        let mut words = heard(text_hits(text), judge, levels);
        words.dedup_by_key(|finding| finding.hit.position.byte);
        found.extend(words);
    }
    found
}

/// The context hits a run reports: not said already, not at `allow`.
fn heard(
    hits: Vec<(Lint, Hit)>,
    judge: &Judge<'_>,
    levels: &Levels,
) -> Vec<Finding> {
    hits.into_iter()
        .filter(|(lint, hit)| !said_already(*lint, *hit, judge, levels))
        .map(|(lint, hit)| levels.finding(hit, lint))
        .filter(|finding| finding.level != Level::Allow)
        .collect()
}

/// Whether the character pass already reported this hit's news.
///
/// `crlf`, `not-nfc` and `mixed-script` can collide; the text lints point
/// at a character for what it IS among its neighbours, so a character
/// the first pass already named keeps that one name (`src/lint:V58`).
/// `ascii` does not grant the carriage return
/// (`src/charset` keeps it in the separate `cr` set), so under the
/// default a CR LF is ALREADY `outside-set`, and a second finding on the
/// same character would say one thing twice. `crlf` therefore speaks
/// where the set grants `cr`, which is where nothing else would. The
/// other two point at a character for a reason that is not the
/// character's: a line that trails, a file left open.
fn said_already(
    lint: Lint,
    hit: Hit,
    judge: &Judge<'_>,
    levels: &Levels,
) -> bool {
    (lint == CRLF || TEXT_LINTS.contains(&lint))
        && loudest(hit, judge, levels).is_some()
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
    config: &Config,
    paths: &[String],
    format: Format,
) -> Result<Report, String> {
    let checker = Checker::configured(config)?;
    let found = gather(&checker, &config.root, paths)?;
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
        Checker, Config, Judge, Looked, OUTSIDE, inspect, levels_for, run,
        shown_path,
    };
    use crate::charset::{CharSet, builtin};
    use crate::lint::{Finding, Group, Hazards, Level, Levels, Lint, Target};
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
        Checker::configured(&Config::discovered(root))?
            .law(shown)
            .map(|(set, _)| set)
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
        let why = Checker::configured(&Config::discovered(&root))
            .err()
            .unwrap_or_default();
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
        let report = run(&Config::discovered(&root), &paths, Format::Json);
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
        let report = run(&Config::discovered(&root), &paths, Format::Human);
        let text = report.map(|r| r.text).unwrap_or_else(|why| why);
        assert_eq!(text, "trojan.rs:1:2 U+202E hazard");
        assert_eq!(Group::Hazard.name(), "hazard");
    }

    /// One finding as `(lint name, level, byte)`.
    type Fired = (&'static str, Level, usize);

    /// What `check` finds in `text` under `set`, with pedantic at `warn`
    /// and the charset findings at `charset`, sorted by byte.
    fn pedantic(text: &str, set: &CharSet, charset: Level) -> Vec<Fired> {
        let mut levels = Levels::new();
        levels.set(Target::Group(Group::Pedantic), Level::Warn);
        levels.set_charset(charset);
        let hazards = hazards();
        let judge = Judge {
            set,
            hazards: &hazards,
            outside: lint(),
        };
        let mut all = match inspect(text.as_bytes(), &judge, &levels) {
            Looked::Findings(all) => all.iter().map(summary).collect(),
            Looked::Unread(_) => Vec::new(),
        };
        all.sort_by_key(|(_, _, byte)| *byte);
        all
    }

    /// V37: the group is `allow`, so a text with every pedantic shape in
    /// it says nothing until a run asks.
    #[test]
    fn pedantic_lints_are_silent_until_asked() {
        assert_eq!(findings("a \r\nb\u{a0}c", &any()), vec![]);
    }

    /// Asked, each of the four fires at the character it points at.
    #[test]
    fn asked_for_every_pedantic_shape_fires_once() {
        let fired = pedantic("a \r\nb\u{a0}c", &any(), Level::Deny);
        let warn = Level::Warn;
        let expected = vec![
            ("trailing-whitespace", warn, 1),
            ("crlf", warn, 2),
            ("unicode-space", warn, 5),
            ("final-newline", warn, 7),
        ];
        assert_eq!(fired, expected);
    }

    /// A no-break space the set does not grant is ONE finding: the set's,
    /// which is the stronger claim. Allow that one, and pedantic speaks.
    #[test]
    fn a_space_outside_the_set_is_reported_once() {
        let ascii = builtin::ascii();
        let denied = pedantic("a\u{a0}b\n", &ascii, Level::Deny);
        assert_eq!(denied, vec![("outside-set", Level::Deny, 1)]);
        let allowed = pedantic("a\u{a0}b\n", &ascii, Level::Allow);
        assert_eq!(allowed, vec![("unicode-space", Level::Warn, 1)]);
    }

    /// The human report for `files` under `ctrm`, with every file that is
    /// not a dotfile named on the command line.
    /// `src/lint:V57`: a Persian word spelled with ZWNJ (mi-khaham, "I
    /// want") is clean where the rule names `persian`, and the same bytes
    /// under `any` still fire -- granting everything excuses nothing.
    #[test]
    fn a_script_preset_excuses_its_own_joiner_and_any_does_not() {
        let word = "\u{0645}\u{06CC}\u{200C}\u{062E}\u{0648}\u{0627}\u{0647}\u{0645}\n";
        let files = [("fa.md", word)];
        let fa = report("ctrm-joiner-fa", "* ascii+persian\n", &files);
        assert_eq!(fa, "");
        let any = report("ctrm-joiner-any", "* any\n", &files);
        assert!(any.contains("U+200C"), "{any}");
    }

    /// The excuse is the joiner the preset grants and nothing else: a bidi
    /// control in a Persian file still fires, and `persian` does not
    /// excuse the ZERO WIDTH JOINER that `hindi` does.
    #[test]
    fn a_script_preset_excuses_nothing_but_its_joiners() {
        let bidi = [("fa.md", "\u{0645}\u{202E}\u{0645}\n")];
        let fired = report("ctrm-joiner-bidi", "* ascii+persian\n", &bidi);
        assert!(fired.contains("U+202E"), "{fired}");
        let conjunct = [("hi.md", "\u{0915}\u{094D}\u{200D}\u{0937}\n")];
        let hi = report("ctrm-joiner-hi", "* ascii+hindi\n", &conjunct);
        assert_eq!(hi, "");
        let fa = report("ctrm-joiner-zwj", "* ascii+persian\n", &conjunct);
        assert!(fa.contains("U+200D"), "{fa}");
    }

    fn report(name: &str, ctrm: &str, files: &[(&str, &str)]) -> String {
        let mut tree = vec![(".ctrm", ctrm)];
        tree.extend_from_slice(files);
        let Some(root) = fixture(name, &tree) else {
            return String::from("(fixture not written)");
        };
        let paths: Vec<String> = files
            .iter()
            .filter(|(path, _)| !path.starts_with('.'))
            .map(|(path, _)| String::from(*path))
            .collect();
        let found = run(&Config::discovered(&root), &paths, Format::Human);
        found.map(|r| r.text).unwrap_or_else(|why| why)
    }

    /// V37 fixture, `crlf`: a Windows batch file, which `cmd.exe` reads
    /// with CR LF endings, in a tree that grants `cr`. The lint cannot
    /// know that; a later rule naming the path allows the lint there. The
    /// line names no set, so it moves the level and leaves the grant alone
    /// (`src/rules:V56`) -- before B5 it narrowed `.bat` back to `ascii`.
    #[test]
    fn a_batch_file_needs_crlf_and_says_so_per_path() {
        let files = [("build.bat", "@echo off\r\n"), ("notes.txt", "hi\r\n")];
        let base = "* ascii+cr !pedantic=warn\n";
        let on = report("ctrm-crlf-on", base, &files);
        let both = "build.bat:1:10 U+000D crlf\nnotes.txt:1:3 U+000D crlf";
        assert_eq!(on, both);
        let ctrm = format!("{base}*.bat !crlf=allow\n");
        let off = report("ctrm-crlf-off", &ctrm, &files);
        assert_eq!(off, "notes.txt:1:3 U+000D crlf");
    }

    /// Under plain `ascii` the CR is not granted, so a CR LF is already
    /// `outside-set`; asking for pedantic does not say it twice.
    #[test]
    fn a_cr_the_set_refuses_is_one_finding() {
        let fired = pedantic("hi\r\n", &builtin::ascii(), Level::Deny);
        assert_eq!(fired, vec![("outside-set", Level::Deny, 2)]);
        let quiet = pedantic("hi\r\n", &builtin::ascii(), Level::Allow);
        assert_eq!(quiet, vec![("crlf", Level::Warn, 2)]);
    }

    /// V37 fixture, `trailing-whitespace`: two trailing spaces are a
    /// Markdown HARD LINE BREAK, which is content, not noise.
    #[test]
    fn a_markdown_hard_break_is_trailing_whitespace_on_purpose() {
        let files = [("a.md", "one  \ntwo\n"), ("a.rs", "fn f() {} \n")];
        let base = "* ascii !pedantic=warn\n";
        let on = report("ctrm-trailing-on", base, &files);
        let rs = "a.rs:1:10 U+0020 trailing-whitespace";
        assert_eq!(on, format!("a.md:1:4 U+0020 trailing-whitespace\n{rs}"));
        let ctrm = format!("{base}*.md ascii !trailing-whitespace=allow\n");
        assert_eq!(report("ctrm-trailing-off", &ctrm, &files), rs);
    }

    /// V37 fixture, `final-newline`: a golden file holding this tool's
    /// own human output, which ends with NO newline, byte for byte. An
    /// empty file is not a finding at all.
    #[test]
    fn a_byte_exact_golden_file_ends_without_a_newline() {
        let golden = ("want.out", "a.rs:1:1 U+2014 ascii");
        let files = [golden, ("lib.rs", "fn f() {}"), ("empty.txt", "")];
        let base = "* ascii !pedantic=warn\n";
        let on = report("ctrm-final-on", base, &files);
        let rs = "lib.rs:1:9 U+007D final-newline";
        assert_eq!(on, format!("{rs}\nwant.out:1:21 U+0069 final-newline"));
        let ctrm = format!("{base}*.out ascii !final-newline=allow\n");
        assert_eq!(report("ctrm-final-off", &ctrm, &files), rs);
    }

    /// V37 fixture, `unicode-space`: French typography puts a no-break
    /// space before `:` and a narrow one before `!`. A project grants them
    /// in a set, and allows the lint where the language wants them.
    #[test]
    fn french_spacing_is_a_unicode_space_on_purpose() {
        let fr = ("fr.md", "Prix\u{a0}: 5\nOui\u{202f}!\n");
        let sets = (".ctrm-sets", "french U+00A0 U+202F\n");
        let files = [sets, fr, ("a.md", "a\u{a0}b\n")];
        let base = "* ascii+french !pedantic=warn\n";
        let one = "a.md:1:2 U+00A0 unicode-space";
        let on = report("ctrm-space-on", base, &files);
        assert!(
            on.starts_with(one) && on.contains("fr.md:2:4 U+202F"),
            "{on}"
        );
        let ctrm = format!("{base}fr.md ascii+french !unicode-space=allow\n");
        assert_eq!(report("ctrm-space-off", &ctrm, &files), one);
    }

    /// One V58 fixture: `files` under `* any !pedantic=warn` fires `on`,
    /// and adding `exempt` (a later rule naming no set) leaves `off`.
    fn exempted(
        name: &str,
        files: &[(&str, &str)],
        exempt: &str,
    ) -> [String; 2] {
        let base = "* any !pedantic=warn\n";
        let on = report(&format!("{name}-on"), base, files);
        let ctrm = format!("{base}{exempt}\n");
        [on, report(&format!("{name}-off"), &ctrm, files)]
    }

    /// V58 fixture, `not-nfc`: a listing captured on macOS, whose file
    /// system hands names back DECOMPOSED. The golden file must keep the
    /// bytes it was given; prose elsewhere is still held to NFC.
    #[test]
    fn a_macos_listing_is_decomposed_on_purpose() {
        let files =
            [("ls.out", "cafe\u{301}.txt\n"), ("a.md", "cafe\u{301}\n")];
        let [on, off] = exempted("ctrm-nfc", &files, "*.out !not-nfc=allow");
        let md = "a.md:1:4 U+0065 not-nfc";
        assert_eq!(on, format!("{md}\nls.out:1:4 U+0065 not-nfc"));
        assert_eq!(off, md);
    }

    /// V58 fixture, `nfkc-compat`: SQUARE METRES are written with a
    /// superscript two, and `m2` is not the same text to a reader.
    #[test]
    fn a_unit_superscript_is_compatibility_on_purpose() {
        let files = [("area.md", "50 m\u{b2}\n"), ("a.md", "x\u{fb01}\n")];
        let exempt = "area.md !nfkc-compat=allow";
        let [on, off] = exempted("ctrm-nfkc", &files, exempt);
        let md = "a.md:1:2 U+FB01 nfkc-compat";
        assert_eq!(on, format!("{md}\narea.md:1:5 U+00B2 nfkc-compat"));
        assert_eq!(off, md);
    }

    /// V58 fixture, `mixed-script`: a micrometre is Greek mu then Latin
    /// m, one word in two scripts by definition. The finding points at
    /// the letter that emptied the word's scripts: the `m`.
    #[test]
    fn a_micrometre_mixes_scripts_on_purpose() {
        let files = [("lab.md", "5 \u{3bc}m\n"), ("a.md", "p\u{3bb}y\n")];
        let exempt = "lab.md !mixed-script=allow";
        let [on, off] = exempted("ctrm-mixed", &files, exempt);
        let md = "a.md:1:2 U+03BB mixed-script";
        assert_eq!(on, format!("{md}\nlab.md:1:4 U+006D mixed-script"));
        assert_eq!(off, md);
    }

    /// V58 fixture, `confusable`: Russian prose. Half its alphabet is
    /// drawn like Latin, so every such letter is a lookalike, and a
    /// Russian file says so once, per path.
    #[test]
    fn russian_prose_is_confusable_on_purpose() {
        let mir = "\u{43c}\u{438}\u{440}\n";
        let files = [("ru.md", mir), ("a.md", "\u{440}\n")];
        let exempt = "ru.md !confusable=allow";
        let [on, off] = exempted("ctrm-confusable", &files, exempt);
        let md = "a.md:1:1 U+0440 confusable";
        assert_eq!(on, format!("{md}\nru.md:1:3 U+0440 confusable"));
        assert_eq!(off, md);
    }

    /// Asked for nothing, none of the four fires, and a character the
    /// set refuses is one `outside-set` finding however many lints it
    /// would also fire. The `e` the accent decomposes from is in the set, so
    /// `not-nfc` still names it.
    #[test]
    fn the_unicode_lints_are_silent_until_asked_and_claim_once() {
        let text = "cafe\u{301} p\u{430}y \u{ff21}\n";
        assert_eq!(findings(text, &any()), vec![]);
        let ascii = builtin::ascii();
        let fired = pedantic(text, &ascii, Level::Deny);
        let deny = Level::Deny;
        let expected = vec![
            ("not-nfc", Level::Warn, 3),
            ("outside-set", deny, 4),
            ("outside-set", deny, 8),
            ("outside-set", deny, 12),
        ];
        assert_eq!(fired, expected);
    }

    /// The json contract is unchanged by a pedantic finding: the same keys,
    /// `set` the set in force, `lint` saying which, asserted whole (V11).
    #[test]
    fn a_pedantic_finding_keeps_the_json_contract() {
        let ctrm = "* ascii+cr !crlf=deny\n";
        let files = [(".ctrm", ctrm), ("n.txt", "hi\r\n")];
        let Some(root) = fixture("ctrm-crlf-json", &files) else {
            return;
        };
        let paths = [String::from("n.txt")];
        let report = run(&Config::discovered(&root), &paths, Format::Json);
        let report = report.unwrap_or_else(|why| unreachable!("{why}"));
        let expected = concat!(
            r#"{"verb":"check","violations":[{"path":"n.txt","line":1,"#,
            r#""column":3,"byte":2,"codepoint":"U+000D","character":"\r","#,
            r#""set":"ascii+cr","lint":"crlf","level":"deny"}],"skipped":[]}"#
        );
        assert_eq!((report.text.as_str(), report.code), (expected, 1));
    }

    /// SARIF: the lint is the `ruleId`, and the region is the one
    /// character the finding points at.
    #[test]
    fn a_pedantic_finding_is_a_sarif_result_on_one_character() {
        let ctrm = "* ascii !final-newline=warn\n";
        let files = [(".ctrm", ctrm), ("n.txt", "ab")];
        let Some(root) = fixture("ctrm-final-sarif", &files) else {
            return;
        };
        let paths = [String::from("n.txt")];
        let found = run(&Config::discovered(&root), &paths, Format::Sarif);
        let log = found.map(|r| r.text).unwrap_or_else(|why| why);
        let result = concat!(
            r#"{"ruleId":"final-newline","level":"warning","#,
            r#""message":{"text":"U+0062 (set in force: ascii)"},"#
        );
        let region = r#""startLine":1,"startColumn":2,"endColumn":3}"#;
        assert!(log.contains(result) && log.contains(region), "{log}");
    }
}
