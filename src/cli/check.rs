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
    CHAR_LINTS, Finding, Group, Hazards, LINE_LINTS, Level, Levels, Lint,
    TEXT_LINTS, Target, char_lints, exit_code, line_hits, one_claim, text_hits,
    unicode_space,
};
use crate::render::{self, Format, Skipped, Violation};
use crate::rules::{self, Resolution, Rule};
use crate::scan::{Hit, Unreadable, decode, scan_str};
use crate::tokens;
use std::path::{Component, Path, PathBuf};

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
    /// The offsets of this text's joiners and tags that sit inside an
    /// RGI emoji sequence, and so are no hazard (`src/lint:V63`).
    exempt: Vec<usize>,
}

impl<'a> Judge<'a> {
    /// A judge for one set, before any text is read.
    const fn new(
        set: &'a CharSet,
        hazards: &'a Hazards,
        outside: Lint,
    ) -> Self {
        Self {
            set,
            hazards,
            outside,
            exempt: Vec::new(),
        }
    }
}

impl Judge<'_> {
    /// This judge, with the sequence exemption read off `text`.
    fn over(&self, text: &str) -> Self {
        Judge {
            exempt: self.hazards.exempt(text),
            ..*self
        }
    }

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
        let hazard =
            self.hazards.lint_at(hit, &self.exempt).filter(|_| !excused);
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
        let rules = config.rules()?;
        let catalog = config.catalog()?;
        Ok(Self {
            rules,
            hazards: Hazards::builtin()?.vouched_by(&catalog),
            catalog,
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
        Judge::new(set, &self.hazards, self.outside)
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
    /// guard and the gate cannot disagree about the same bytes
    /// (`src/cli/guard:V35`).
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

/// The character findings, then the line- and text-shaped ones, with ONE
/// claim per character across all three (`src/lint:V55`): the walks each
/// know only their own lints, so the lint node's claim order settles a
/// byte two of them point at.
fn findings_in(text: &str, judge: &Judge<'_>, levels: &Levels) -> Vec<Finding> {
    let judge = &judge.over(text);
    let deep = asked(&CHAR_LINTS, levels);
    let hits = scan_str(text, |c| judge.passes(c, deep));
    let mut found = reportable(hits, judge, levels);
    found.extend(context_findings(text, levels));
    one_claim(found)
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
/// not ask for pedantic. A character one of them shares with another
/// finding is settled by `one_claim` in the caller: `ascii` does not
/// grant the carriage return (`src/charset` keeps it in the separate `cr`
/// set), so under the default a CR LF is ALREADY `outside-set`, and
/// `crlf` speaks only where the set grants `cr`; a trailing space that is
/// also the file's last character is `trailing-whitespace` alone.
fn context_findings(text: &str, levels: &Levels) -> Vec<Finding> {
    let mut found = Vec::new();
    if asked(&LINE_LINTS, levels) {
        found.extend(heard(line_hits(text), levels));
    }
    if asked(&TEXT_LINTS, levels) {
        found.extend(heard(text_hits(text), levels));
    }
    found
}

/// The context hits a run reports: not at `allow`.
fn heard(hits: Vec<(Lint, Hit)>, levels: &Levels) -> Vec<Finding> {
    hits.into_iter()
        .map(|(lint, hit)| levels.finding(hit, lint))
        .filter(|finding| finding.level != Level::Allow)
        .collect()
}

/// The path as a reader typed it: relative to the root, so it matches the
/// patterns in `.ctrm` and reads like the file they meant.
///
/// Both sides are folded LEXICALLY first (V71): `sub/../sub/c.md` names
/// `sub/c.md`, and a rule anchored at `sub/c.md` has to see that spelling
/// or it silently judges the file by another rule. Lexical, not
/// canonical: a symlink is not resolved, so the path stays the one typed.
pub(super) fn shown_path(root: &Path, full: &Path) -> String {
    let (root, full) = (lexical(root), lexical(full));
    full.strip_prefix(&root)
        .unwrap_or(&full)
        .to_string_lossy()
        .into_owned()
}

/// `path` with every `.` dropped and every `..` folded into the name
/// before it. A `..` with no name before it is kept: it leaves the tree.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        let named =
            matches!(out.components().next_back(), Some(Component::Normal(_)));
        match part {
            Component::CurDir => {}
            Component::ParentDir if named => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
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
        code: exit_code(&findings).max(unreadable_code(&found.skips)),
    })
}

/// Invalid UTF-8 is an ERROR, exit 1 (`src/scan:V8`): the file claims to
/// be text and is not, so no verdict about its characters was reached
/// (B22). A binary skip is not: the file never claimed to be text, and
/// failing on every tracked PNG would make the gate unusable. One code for
/// every format, since json and SARIF report the same run.
fn unreadable_code(skips: &[Skip]) -> u8 {
    let broken =
        |skip: &Skip| matches!(skip.reason, Unreadable::NotUtf8 { .. });
    u8::from(skips.iter().any(broken))
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
#[path = "check_test.rs"]
mod tests;
