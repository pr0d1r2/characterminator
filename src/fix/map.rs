//! The transliteration map: what a disallowed character is rewritten to.
//!
//! One line is `<from> <to>`, where `to` may be absent and an absent `to` is
//! an explicit delete -- the ONLY way this crate ever removes a character
//! (V4). Both sides are tokens in the sense of `codepoint`, so a replacement
//! that is itself a space is written `U+0020`: the line is split on
//! whitespace, and a trailing blank nobody can see would be a poor way to
//! declare a space.
//!
//! Sources are held longest first, because a declared sequence must win over
//! a declared prefix of it (V31).
//!
//! Two more line kinds serve V51. `word <from> <to>` is an entry whose
//! replacement is a WORD, which `apply` keeps apart from a neighbouring
//! letter. `use <name>` reads a named builtin map at that line, so an
//! opt-in map is one line of the grammar every source already speaks --
//! a `.ctrm-map` line or a `--map` flag alike (`src/rules:V18`).

use crate::fix::class;
use crate::fix::codepoint::decode;
use crate::fix::family::{ROOT, Tree};
use crate::fix::{Class, Error, Family, MapEntry};
use crate::rules::Origin;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet};

/// The builtin map, in the `.ctrm-map` grammar (`src/charset:V22`).
///
/// Public as TEXT for the reason the sets file is: the builtin is the
/// lowest source of the precedence chain (`src/rules:V19`) and arrives
/// there the same way a `.ctrm-map` or a `--map` flag does. One grammar,
/// one parser, and an entry a user overrides by declaring it again.
///
/// Two files joined at compile time, as the sets are: the hand-written
/// typography and modifier map (V26, V60), then the GENERATED emoji
/// sequence map (V62), which a regeneration rewrites whole. A builtin
/// line number counts from the top of the joined text.
pub(crate) const BUILTIN: &str = concat!(
    include_str!("map.ctrm-map"),
    include_str!("emoji-seq.ctrm-map")
);

/// The opt-in `words` map (V51): notation to the English it abbreviates.
///
/// NOT layered by default, which is the whole of V26's promise: the
/// builtin rewrites only what a writer never chose, and these symbols
/// carry meaning somebody typed on purpose. A `use words` line asks.
const WORDS: &str = include_str!("words.ctrm-map");

/// Every map a `use` line may name. A table rather than a match, so the
/// test that each one parses walks the same list the parser reads.
const NAMED: &[(&str, &str)] = &[("words", WORDS)];

/// A declared source matched at the current position. `word` says the
/// replacement is a word, which `apply` keeps off a neighbouring letter.
pub(crate) struct Match {
    pub(crate) len: usize,
    pub(crate) to: String,
    pub(crate) word: bool,
}

/// What a matched source rewrites to: a literal replacement from a map
/// entry (a word or not), or a class whose answer depends on the fidelity
/// family (V28).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    To { to: String, word: bool },
    Class(usize),
}

impl Target {
    const fn word(&self) -> bool {
        matches!(self, Self::To { word: true, .. })
    }
}

/// One rewritable source, ready for longest-first matching.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    source: String,
    target: Target,
}

/// The effective map: the entries, classes and families these lines
/// declared, plus the candidate list `resolve_at` walks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Map {
    entries: Vec<MapEntry>,
    classes: Vec<Class>,
    tree: Tree,
    fidelity: Option<String>,
    candidates: Vec<Candidate>,
    /// Each first character, and the candidates opening with it, in
    /// candidate order: the scan asks at EVERY position, and two thousand
    /// emoji sequences (V62) tried in turn cost seconds on a large file.
    starts: BTreeMap<char, Vec<usize>>,
}

impl Map {
    /// The builtin map (V26) at the default fidelity: what `ctrm fix` runs
    /// with no `.ctrm-map`. INFALLIBLE because the text is compiled in; a
    /// defect in it is this crate's, and `map_test` keeps it from shipping.
    #[must_use]
    pub fn builtin() -> Self {
        Self::parse(BUILTIN, &|line| Origin::Builtin { line })
            .unwrap_or_default()
    }

    /// Parse map lines. `origin` labels a 1-based line number, so the same
    /// parser serves a file, a builtin and a `--map` flag (`src/rules:V18`).
    pub(crate) fn parse(
        source: &str,
        origin: &dyn Fn(usize) -> Origin,
    ) -> Result<Self, Error> {
        Self::default().layer(source, origin)
    }

    /// Read another source OVER this map, later winning per source and
    /// per class name (`src/rules:V19`).
    ///
    /// The precedence chain hands its sources in order, and each one is a
    /// WHOLE text rather than a line: a `family` line declares a tree a
    /// later line may use, so a map cannot be assembled entry by entry
    /// the way rules and sets can. Validation runs after every layer, so
    /// a source that breaks the tree is refused where it was added rather
    /// than blamed on whatever came last.
    ///
    /// # Errors
    ///
    /// As [`Map::parse`]: a malformed line, a cyclic family tree, or a
    /// class naming a family nothing declared.
    pub(crate) fn layer(
        mut self,
        source: &str,
        origin: &dyn Fn(usize) -> Origin,
    ) -> Result<Self, Error> {
        for (index, line) in source.lines().enumerate() {
            self.read(line, index.saturating_add(1), origin)?;
        }
        self.dedupe();
        self.entries.sort_by_key(|entry| Reverse(entry.from.len()));
        self.tree.validate()?;
        self.validate_classes()?;
        self.index();
        Ok(self)
    }

    /// The family tree these lines declared, on top of the builtin one.
    #[must_use]
    pub(crate) fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The equivalence classes these lines declared.
    #[must_use]
    pub(crate) fn classes(&self) -> &[Class] {
        &self.classes
    }

    /// The family classes resolve for. `ascii` unless the caller says
    /// otherwise, which is the strict default `src/rules:V1` asks for; the
    /// rules node supplies `text` and any `@<family>` per `src/rules:V29`.
    #[must_use]
    pub(crate) fn fidelity(&self) -> &str {
        self.fidelity.as_deref().unwrap_or(ROOT)
    }

    /// Resolve classes for `family` instead. The family has to be in the
    /// tree, so a typo is a config error rather than a silent fallback.
    pub fn with_fidelity(mut self, family: &str) -> Result<Self, Error> {
        self.tree.path(family)?;
        self.fidelity = Some(family.to_owned());
        Ok(self)
    }

    /// The effective entries, longest source first. Each carries its origin,
    /// which is how `explain` answers "why" (`src/rules:V20`).
    #[must_use]
    pub(crate) fn entries(&self) -> &[MapEntry] {
        &self.entries
    }

    /// The longest declared source -- map entry or class member -- that
    /// matches here, covers at least one disallowed character, and resolves
    /// to something. A span of allowed characters is never a violation, so
    /// it is never rewritten (V6).
    pub(crate) fn resolve_at(
        &self,
        rest: &str,
        allowed: &dyn Fn(char) -> bool,
    ) -> Result<Option<Match>, Error> {
        let first = rest.chars().next().and_then(|c| self.starts.get(&c));
        let held = first.into_iter().flatten();
        for candidate in held.filter_map(|at| self.candidates.get(*at)) {
            let Some(to) = self.try_at(candidate, rest, allowed)? else {
                continue;
            };
            return Ok(Some(Match {
                len: candidate.source.len(),
                to,
                word: candidate.target.word(),
            }));
        }
        Ok(None)
    }

    /// How many times a replacement may be rewritten again before the map is
    /// called cyclic. An acyclic chain visits each source at most once, so
    /// anything longer has come back to a source it already used.
    pub(crate) fn budget(&self) -> usize {
        self.candidates.len().saturating_add(1)
    }

    fn try_at(
        &self,
        candidate: &Candidate,
        rest: &str,
        allowed: &dyn Fn(char) -> bool,
    ) -> Result<Option<String>, Error> {
        if !rest.starts_with(&candidate.source)
            || !violates(&candidate.source, allowed)
        {
            return Ok(None);
        }
        self.target_of(&candidate.target, allowed)
    }

    fn target_of(
        &self,
        target: &Target,
        allowed: &dyn Fn(char) -> bool,
    ) -> Result<Option<String>, Error> {
        match target {
            Target::To { to, .. } => Ok(Some(to.clone())),
            Target::Class(index) => match self.classes.get(*index) {
                Some(class) => {
                    class::resolve(class, &self.tree, self.fidelity(), allowed)
                }
                None => Ok(None),
            },
        }
    }

    /// One line, dispatched by its first word.
    ///
    /// The keywords cost no source anyone could map: a `<from>` is only
    /// ever rewritten where it is DISALLOWED (V6), and `family`, `use` and
    /// `word` are ASCII, which every set holds.
    fn read(
        &mut self,
        line: &str,
        number: usize,
        origin: &dyn Fn(usize) -> Origin,
    ) -> Result<(), Error> {
        let body = line.trim();
        if body.is_empty() || body.starts_with('#') {
            return Ok(());
        }
        match body.split_once(' ') {
            Some(("family", rest)) => self.declare(rest, number),
            Some(("=", rest)) => self.classify(rest, number),
            Some(("use", rest)) => self.pull(rest.trim(), &origin(number)),
            Some(("word", rest)) => {
                self.add(word(rest, origin(number)), number)
            }
            _ => self.add(entry(body, origin(number), false), number),
        }
    }

    /// `use <name>`: read a named builtin map AT this line, so it sits in
    /// the precedence chain exactly where it was asked for and a later
    /// line still overrides one of its entries (`src/rules:V19`).
    ///
    /// Every entry it brings carries the ORIGIN of the `use` line rather
    /// than a line of the named file: the answer to "why was this
    /// rewritten" is the line somebody wrote to opt in (`src/rules:V20`),
    /// and it needs no new kind of origin to say so.
    fn pull(&mut self, name: &str, origin: &Origin) -> Result<(), Error> {
        let text = NAMED
            .iter()
            .find(|(held, _)| *held == name)
            .map(|(_, text)| *text)
            .ok_or_else(|| Error::UnknownMap {
                name: name.to_owned(),
            })?;
        for (index, line) in text.lines().enumerate() {
            self.read(line, index.saturating_add(1), &|_| origin.clone())?;
        }
        Ok(())
    }

    fn declare(&mut self, rest: &str, number: usize) -> Result<(), Error> {
        let declared = family(rest).ok_or(Error::Syntax { line: number })?;
        self.tree.declare(declared)
    }

    /// A later line wins over an earlier one for the same class name, as it
    /// does for a map entry (`src/rules:V19`).
    fn classify(&mut self, rest: &str, number: usize) -> Result<(), Error> {
        let declared =
            class::parse_line(rest).ok_or(Error::Syntax { line: number })?;
        self.classes.retain(|held| held.name != declared.name);
        self.classes.push(declared);
        Ok(())
    }

    /// A later line wins over an earlier one for the same source, which is
    /// the precedence the config chain expects (`src/rules:V19`).
    fn add(
        &mut self,
        entry: Option<MapEntry>,
        number: usize,
    ) -> Result<(), Error> {
        let entry = entry.ok_or(Error::Syntax { line: number })?;
        self.entries.push(entry);
        Ok(())
    }

    /// Keep the LAST entry for each source, where its last push left it:
    /// what dropping the earlier one on every push kept, in one walk from
    /// the end rather than one walk of every entry per line -- quadratic
    /// over the two thousand builtin lines.
    fn dedupe(&mut self) {
        let mut seen = HashSet::new();
        let mut kept: Vec<MapEntry> = std::mem::take(&mut self.entries)
            .into_iter()
            .rev()
            .filter(|entry| seen.insert(entry.from.clone()))
            .collect();
        kept.reverse();
        self.entries = kept;
    }

    /// Every family a class member names has to be in the tree, or that
    /// member could never resolve and the line is a typo nothing reports.
    fn validate_classes(&self) -> Result<(), Error> {
        for class in &self.classes {
            for member in &class.members {
                self.tree.path(&member.family)?;
            }
        }
        Ok(())
    }

    /// Build the longest-first candidate list. A map entry beats a class
    /// member of the same length, because a line naming one source is more
    /// specific than a class listing it among alternatives; the sort is
    /// stable, so entries pushed first keep that priority.
    fn index(&mut self) {
        self.candidates = Vec::new();
        for entry in &self.entries {
            self.candidates.push(Candidate {
                source: entry.from.clone(),
                target: Target::To {
                    to: entry.to.clone(),
                    word: entry.word,
                },
            });
        }
        self.index_classes();
        self.candidates
            .sort_by_key(|held| Reverse(held.source.len()));
        self.index_starts();
    }

    /// Group the sorted candidates by first character, keeping their order.
    fn index_starts(&mut self) {
        self.starts = BTreeMap::new();
        for (at, held) in self.candidates.iter().enumerate() {
            if let Some(first) = held.source.chars().next() {
                self.starts.entry(first).or_default().push(at);
            }
        }
    }

    fn index_classes(&mut self) {
        let mut found = Vec::new();
        for (index, class) in self.classes.iter().enumerate() {
            for member in &class.members {
                found.push(Candidate {
                    source: member.text.clone(),
                    target: Target::Class(index),
                });
            }
        }
        self.candidates.append(&mut found);
    }
}

/// One `family <name> <parent>` line. A name carrying `:` or `,` is
/// refused, because those two characters are what a class line is made of.
fn family(rest: &str) -> Option<Family> {
    let mut words = rest.split_whitespace();
    let name = named(words.next()?)?;
    let parent = named(words.next()?)?;
    if words.next().is_some() {
        return None;
    }
    Some(Family {
        name,
        parent: Some(parent),
    })
}

pub(crate) fn named(word: &str) -> Option<String> {
    if word.contains([':', ',']) {
        return None;
    }
    Some(word.to_owned())
}

/// Whether any character of `text` is one the caller disallows.
pub(crate) fn violates(text: &str, allowed: &dyn Fn(char) -> bool) -> bool {
    text.chars().any(|ch| !allowed(ch))
}

/// One `<from> <to>` line. An empty source is rejected: it would match at
/// every position and rewrite nothing.
fn entry(body: &str, origin: Origin, word: bool) -> Option<MapEntry> {
    let mut words = body.split_whitespace();
    let from = decode(words.next()?)?;
    let to = match words.next() {
        Some(word) => decode(word)?,
        None => String::new(),
    };
    if from.is_empty() || words.next().is_some() {
        return None;
    }
    Some(MapEntry {
        from,
        to,
        word,
        origin,
    })
}

/// One `word <from> <to>` line (V51). The replacement is required: a
/// word that deletes is a delete, and the plain line already says that.
fn word(rest: &str, origin: Origin) -> Option<MapEntry> {
    entry(rest, origin, true).filter(|held| !held.to.is_empty())
}

#[cfg(test)]
#[path = "map_test.rs"]
mod tests;
