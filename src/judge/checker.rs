//! The judging engine every verb shares: which set and levels govern a
//! path, and what one file's bytes are found to hold against them.
//!
//! `check`, `fix`, `stats`, `explain` and the guard all judge through the
//! one [`Checker`], so no two of them can disagree about the same bytes
//! (`src/judge:V99`). The verbs own what they DO with a finding; this
//! file owns what a finding IS.

use super::Config;
use crate::charset::{CharSet, SetCatalog};
use crate::lint::{
    CHAR_LINTS, Finding, Hazards, LINE_LINTS, Level, Levels, Lint, TEXT_LINTS,
    Target, char_lints, line_hits, one_claim, text_hits, unicode_space,
};
use crate::rules::{self, Resolution, Rule};
use crate::scan::{Hit, Unreadable, decode, scan_str};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The lint a character outside its set is reported under. Named for what
/// is true of the character, not for its group (`src/lint` registry).
const OUTSIDE: &str = "outside-set";

/// One file's contribution to the report.
pub(crate) enum Looked {
    /// What was found, already levelled.
    Findings(Vec<Finding>),
    /// The file could not be read AS TEXT, and is named rather than
    /// dropped (`src/scan:V8`).
    Unread(Unreadable),
}

/// The rules and the sets a run resolves against, read once, and the
/// lints `check` reports under.
pub(crate) struct Checker {
    rules: Vec<Rule>,
    catalog: SetCatalog,
    /// The hazard classes, from the COMPILED-IN data (`src/lint:V34`) and
    /// never from `catalog`: nothing a run configures can reach them.
    hazards: Hazards,
    outside: Lint,
    /// `--strict`: warn counts as deny (`src/lint:V36`).
    strict: bool,
    /// Each union [`Checker::granted`] resolved, by family, then sets.
    unions: RefCell<Unions>,
}

/// The kept unions: family, then the set names, to the set.
type Unions = HashMap<String, HashMap<Vec<String>, Rc<CharSet>>>;

/// What one file's characters are judged against: its set, and the
/// hazards that fire whatever the set says.
pub(crate) struct Judge<'a> {
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
    ///
    /// LAZY in the pedantic tail (`src/render:R17`): the caller takes the first
    /// lint that speaks, and under the default that is `outside-set`, so the
    /// Unicode tables are asked only when nothing stronger claimed the
    /// character.
    fn lints_for(&self, hit: Hit) -> impl Iterator<Item = Lint> {
        let excused = self.hazards.excuses(&self.set.name, hit.character);
        let hazard =
            self.hazards.lint_at(hit, &self.exempt).filter(|_| !excused);
        let outside =
            (!self.set.contains(hit.character)).then_some(self.outside);
        let pedantic = std::iter::once_with(move || char_lints(hit.character));
        [hazard, outside]
            .into_iter()
            .flatten()
            .chain(pedantic.flatten().flatten())
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
    /// A line that cannot be parsed, named at its origin.
    pub(crate) fn configured(config: &Config) -> Result<Self, String> {
        let rules = config.rules()?;
        let catalog = config.catalog()?;
        Ok(Self {
            rules,
            hazards: Hazards::builtin().vouched_by(&catalog),
            catalog,
            outside: Lint::named(OUTSIDE)
                .ok_or_else(|| String::from("no `outside-set` lint"))?,
            strict: config.strict,
            unions: RefCell::default(),
        })
    }

    /// The checker `sets` lists from: every locale set in, not only the
    /// ones the rules name (`src/charset:V61`).
    ///
    /// # Errors
    ///
    /// As [`Checker::configured`].
    pub(crate) fn listing(config: &Config) -> Result<Self, String> {
        let catalog = config.listing()?;
        Ok(Self {
            catalog,
            ..Self::configured(config)?
        })
    }

    /// What one file's characters are judged against.
    pub(crate) fn judge<'a>(&'a self, set: &'a CharSet) -> Judge<'a> {
        Judge::new(set, &self.hazards, self.outside)
    }

    /// [`Checker::shared_law`], with the set copied out, as a test reads it.
    #[cfg(test)]
    pub(crate) fn law(&self, shown: &str) -> Result<(CharSet, Levels), String> {
        let (set, levels) = self.shared_law(shown)?;
        Ok((CharSet::clone(&*set), levels))
    }

    /// What one path may contain, and how loudly a stray character there is
    /// reported. The set is SHARED: its union is resolved once per run for each
    /// sets-and-family a rule grants, not once per file (`src/render:R17`).
    pub(crate) fn shared_law(
        &self,
        shown: &str,
    ) -> Result<(Rc<CharSet>, Levels), String> {
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
    pub(crate) fn findings(
        &self,
        shown: &str,
        bytes: &[u8],
    ) -> Result<Option<Vec<Finding>>, String> {
        let (set, levels) = self.shared_law(shown)?;
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
    pub(crate) fn effective(
        &self,
        shown: &str,
    ) -> Result<(CharSet, Option<Rule>), String> {
        let found = rules::resolve(shown, &self.rules, &rules::matches);
        let set = CharSet::clone(&*self.granted(&found)?);
        Ok((set, found.winner.cloned()))
    }

    /// Every declared set, resolved at one fidelity -- `sets`' answer.
    ///
    /// # Errors
    ///
    /// A declared set that cannot be resolved: a cycle, or a member
    /// naming a set nothing declares.
    pub(crate) fn declared(
        &self,
        family: &str,
    ) -> Result<Vec<CharSet>, String> {
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
    ///
    /// Resolved once per sets-and-family and kept (`src/render:R17`): every
    /// file one rule governs asks the same question, and under a locale set the
    /// union is the costliest thing a small file asks for. Looked up by
    /// borrowed key, so a file whose answer is kept allocates nothing.
    fn granted(&self, found: &Resolution<'_>) -> Result<Rc<CharSet>, String> {
        if let Some(set) = self.kept(found) {
            return Ok(set);
        }
        let set = Rc::new(self.union(found)?);
        let mut kept = self.unions.borrow_mut();
        let by_sets = kept.entry(found.family.clone()).or_default();
        by_sets.insert(found.sets.clone(), Rc::clone(&set));
        Ok(set)
    }

    /// The union already resolved for this sets-and-family, if any.
    fn kept(&self, found: &Resolution<'_>) -> Option<Rc<CharSet>> {
        let kept = self.unions.borrow();
        let by_sets = kept.get(found.family.as_str())?;
        by_sets.get(found.sets.as_slice()).map(Rc::clone)
    }

    fn union(&self, found: &Resolution<'_>) -> Result<CharSet, String> {
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
pub(crate) fn inspect(
    bytes: &[u8],
    judge: &Judge<'_>,
    levels: &Levels,
) -> Looked {
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

#[cfg(test)]
#[path = "checker_test.rs"]
mod tests;
