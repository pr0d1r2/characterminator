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
use std::collections::BTreeMap;

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
pub const BUILTIN: &str = concat!(
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
    /// Parse map lines. `origin` labels a 1-based line number, so the same
    /// parser serves a file, a builtin and a `--map` flag (`src/rules:V18`).
    pub fn parse(
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
    pub fn layer(
        mut self,
        source: &str,
        origin: &dyn Fn(usize) -> Origin,
    ) -> Result<Self, Error> {
        for (index, line) in source.lines().enumerate() {
            self.read(line, index.saturating_add(1), origin)?;
        }
        self.entries.sort_by_key(|entry| Reverse(entry.from.len()));
        self.tree.validate()?;
        self.validate_classes()?;
        self.index();
        Ok(self)
    }

    /// The family tree these lines declared, on top of the builtin one.
    #[must_use]
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The equivalence classes these lines declared.
    #[must_use]
    pub fn classes(&self) -> &[Class] {
        &self.classes
    }

    /// The family classes resolve for. `ascii` unless the caller says
    /// otherwise, which is the strict default `src/rules:V1` asks for; the
    /// rules node supplies `text` and any `@<family>` per `src/rules:V29`.
    #[must_use]
    pub fn fidelity(&self) -> &str {
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
    pub fn entries(&self) -> &[MapEntry] {
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
        self.entries.retain(|held| held.from != entry.from);
        self.entries.push(entry);
        Ok(())
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
mod tests {
    use super::Map;
    use crate::fix::Error;
    use crate::rules::Origin;

    fn parse(source: &str) -> Result<Map, Error> {
        Map::parse(source, &|line| Origin::Builtin { line })
    }

    fn parsed(source: &str) -> Map {
        parse(source).unwrap_or_default()
    }

    #[test]
    fn reads_a_line_in_either_form() {
        let map = parsed("U+2014 --\n\u{2013} -\n");
        assert!(parse("U+2014 --\n\u{2013} -\n").is_ok());
        assert_eq!(map.entries().len(), 2);
    }

    #[test]
    fn an_absent_replacement_is_an_explicit_delete() {
        let map = parsed("U+200B\n");
        assert_eq!(map.entries().first().map(|e| e.to.as_str()), Some(""));
    }

    #[test]
    fn a_replacement_of_one_space_is_written_in_code_point_form() {
        let map = parsed("U+00A0 U+0020\n");
        assert_eq!(map.entries().first().map(|e| e.to.as_str()), Some(" "));
    }

    #[test]
    fn skips_blank_lines_and_comments() {
        assert_eq!(parsed("\n# a note\n   \nU+2014 --\n").entries().len(), 1);
    }

    #[test]
    fn carries_the_origin_of_the_line_it_came_from() {
        let map = parsed("# a note\nU+2014 --\n");
        let origin = map.entries().first().map(|e| e.origin.clone());
        assert_eq!(origin, Some(Origin::Builtin { line: 2 }));
    }

    #[test]
    fn a_later_line_wins_for_the_same_source() {
        let map = parsed("U+2014 --\nU+2014 -\n");
        assert_eq!(map.entries().len(), 1);
        assert_eq!(map.entries().first().map(|e| e.to.as_str()), Some("-"));
    }

    #[test]
    fn holds_the_longest_source_first() {
        let map = parsed("U+1F44D +1\nU+1F44D+U+1F3FD U+1F44D\n");
        let first = map.entries().first().map(|e| e.from.chars().count());
        assert_eq!(first, Some(2));
    }

    #[test]
    fn rejects_a_line_it_cannot_read() {
        assert_eq!(parse("U+2014 -- --\n"), Err(Error::Syntax { line: 1 }));
        assert_eq!(parse("\nU+ZZZZ -\n"), Err(Error::Syntax { line: 2 }));
    }

    #[test]
    fn a_family_line_declares_a_family() {
        let map = parsed("family nerd emoji\n");
        let want = Ok(vec!["nerd", "emoji", "text", "ascii"]);
        assert_eq!(map.tree().path("nerd"), want);
        assert_eq!(map.entries().len(), 0);
    }

    #[test]
    fn a_family_line_with_an_unknown_parent_fails_to_parse() {
        let name = String::from("runic");
        let found = parse("family nerd runic\n");
        assert_eq!(found, Err(Error::UnknownFamily { name }));
    }

    #[test]
    fn a_family_cycle_fails_to_parse() {
        let found = parse("family a b\nfamily b a\n");
        assert!(matches!(found, Err(Error::FamilyCycle { .. })));
    }

    #[test]
    fn a_class_line_declares_a_class() {
        let map = parsed("= tick ascii:[x] emoji:U+2705\n");
        assert_eq!(map.classes().len(), 1);
        assert_eq!(map.entries().len(), 0);
    }

    #[test]
    fn a_later_class_line_wins_for_the_same_name() {
        let map = parsed("= tick ascii:[x]\n= tick ascii:(x)\n");
        assert_eq!(map.classes().len(), 1);
        let first = map.classes().first().map(|held| held.members.len());
        assert_eq!(first, Some(1));
    }

    #[test]
    fn a_member_in_an_unknown_family_fails_to_parse() {
        let name = String::from("runic");
        let found = parse("= tick runic:x\n");
        assert_eq!(found, Err(Error::UnknownFamily { name }));
    }

    #[test]
    fn the_fidelity_family_defaults_to_the_root_and_must_be_known() {
        let map = parsed("= tick ascii:[x] emoji:U+2705\n");
        assert_eq!(map.fidelity(), "ascii");
        assert!(map.clone().with_fidelity("emoji").is_ok());
        assert!(map.with_fidelity("runic").is_err());
    }

    #[test]
    fn rejects_a_class_line_it_cannot_read() {
        let line = Err(Error::Syntax { line: 1 });
        assert_eq!(parse("= tick\n"), line);
        assert_eq!(parse("= tick emoji\n"), line);
    }

    #[test]
    fn rejects_a_family_line_it_cannot_read() {
        let line = Err(Error::Syntax { line: 1 });
        assert_eq!(parse("family nerd\n"), line);
        assert_eq!(parse("family a b c\n"), line);
        assert_eq!(parse("family we:ird emoji\n"), line);
    }
    /// The builtin map ships as DATA in this grammar (V26,
    /// `src/charset:V22`), so it has to parse -- and a defect here is a
    /// defect in this crate, not in anyone's configuration.
    #[test]
    fn the_builtin_map_parses() {
        assert!(parse(super::BUILTIN).is_ok());
    }

    #[test]
    fn the_builtin_map_is_pure_ascii() {
        assert!(super::BUILTIN.is_ascii());
    }

    /// V26 names what it targets. Written out rather than read back from
    /// the map: a test that asked the file what it declares would pass
    /// just as happily after an entry was deleted from it.
    fn rewrites(map: &Map, from: char, to: &str) {
        let found = map
            .entries()
            .iter()
            .find(|entry| entry.from == from.to_string());
        assert_eq!(found.map(|e| e.to.as_str()), Some(to), "{from:?}");
    }

    /// V60: the presentation selectors and skin tones are deleted, so an
    /// emoji with a modifier compresses to its base without a sequence
    /// scan.
    #[test]
    fn the_builtin_map_deletes_the_emoji_modifiers() {
        let map = parsed(super::BUILTIN);
        for from in ['\u{FE0E}', '\u{FE0F}', '\u{1F3FB}', '\u{1F3FF}'] {
            rewrites(&map, from, "");
        }
    }

    #[test]
    fn the_builtin_map_targets_what_v26_names() {
        let map = parsed(super::BUILTIN);
        let rewrites = |from, to| rewrites(&map, from, to);
        rewrites('\u{2014}', "--");
        rewrites('\u{2013}', "-");
        rewrites('\u{2212}', "-");
        rewrites('\u{2018}', "'");
        rewrites('\u{2019}', "'");
        rewrites('\u{201C}', "\"");
        rewrites('\u{201D}', "\"");
        rewrites('\u{00AB}', "\"");
        rewrites('\u{00BB}', "\"");
        rewrites('\u{2026}', "...");
        rewrites('\u{00A0}', " ");
    }

    /// The two that leave rather than change: they carry no glyph, so any
    /// visible replacement would put a character on the page the author
    /// never typed (V4 makes the delete explicit).
    #[test]
    fn the_builtin_map_deletes_the_invisibles() {
        let map = parsed(super::BUILTIN);
        for gone in ['\u{200B}', '\u{FEFF}'] {
            let found = map
                .entries()
                .iter()
                .find(|entry| entry.from == gone.to_string());
            assert_eq!(found.map(|e| e.to.as_str()), Some(""), "{gone:?}");
        }
    }

    /// `src/lint:V34`: a bidi control is REPORTED, never auto-removed. A
    /// Trojan Source override deleted by `fix` would leave code that now
    /// reads as it runs and no record that anyone tried to hide it, so
    /// the map has no entry for any of them and they stay unmapped (V4).
    #[test]
    fn the_builtin_map_leaves_every_bidi_control_alone() {
        let map = parsed(super::BUILTIN);
        let bidi = ['\u{061C}', '\u{200E}', '\u{200F}']
            .into_iter()
            .chain('\u{202A}'..='\u{202E}')
            .chain('\u{2066}'..='\u{2069}');
        for kept in bidi {
            let from = kept.to_string();
            let mapped = map.entries().iter().any(|entry| entry.from == from);
            assert!(!mapped, "U+{:04X} is mapped", u32::from(kept));
        }
    }

    /// Every replacement the builtin offers is itself ASCII. A map whose
    /// answer was another non-ASCII character would move the problem
    /// rather than solve it, and `fix` would report the result as a
    /// violation of the same rule (V4).
    ///
    /// The ONE exception is V62's: a ZWJ sequence compresses to a single
    /// emoji, which has no ASCII form. It is still ONE code point the
    /// `emoji` preset grants, so a file under `emoji` is settled by it,
    /// and a file under `ascii` is left one finding instead of several.
    #[test]
    fn every_builtin_replacement_is_ascii_or_one_emoji() {
        let emoji = crate::charset::builtin::catalog()
            .ok()
            .and_then(|c| c.resolve("emoji", "text").ok());
        for entry in parsed(super::BUILTIN).entries() {
            let one = entry.to.chars().count() == 1
                && entry
                    .to
                    .chars()
                    .all(|c| emoji.as_ref().is_some_and(|e| e.contains(c)));
            let ok = entry.to.is_ascii() || one;
            assert!(ok, "{} -> {}", entry.from, entry.to);
        }
    }

    fn found(map: &Map, from: char) -> Option<super::MapEntry> {
        let from = from.to_string();
        map.entries()
            .iter()
            .find(|entry| entry.from == from)
            .cloned()
    }

    #[test]
    fn a_word_line_declares_a_word_and_a_plain_line_does_not() {
        let map = parsed("word U+22A5 not\nU+2014 --\n");
        assert_eq!(found(&map, '\u{22A5}').map(|e| e.word), Some(true));
        assert_eq!(found(&map, '\u{2014}').map(|e| e.word), Some(false));
    }

    /// A word that deletes is a delete, which the plain line already says;
    /// a `word` line without a word is a typo, not a second spelling.
    #[test]
    fn a_word_line_needs_a_word() {
        assert_eq!(parse("word U+22A5\n"), Err(Error::Syntax { line: 1 }));
        assert_eq!(parse("word a b c\n"), Err(Error::Syntax { line: 1 }));
    }

    /// V51: `use words` reads the named map at that line, and each entry
    /// it brings answers "why" with the line that opted in (V20).
    #[test]
    fn a_use_line_pulls_in_a_named_map_under_its_own_origin() {
        let map = parsed("# opt in\nuse words\n");
        let up_tack = found(&map, '\u{22A5}');
        assert_eq!(up_tack.clone().map(|e| e.to), Some(String::from("not")));
        let origin = up_tack.map(|e| e.origin);
        assert_eq!(origin, Some(Origin::Builtin { line: 2 }));
    }

    #[test]
    fn a_use_line_naming_no_builtin_map_is_refused() {
        let name = String::from("klingon");
        assert_eq!(parse("use klingon\n"), Err(Error::UnknownMap { name }));
    }

    /// Precedence is POSITIONAL (`src/rules:V19`): a line after `use`
    /// overrides what it pulled in, and `use` overrides a line before it.
    #[test]
    fn a_use_line_sits_in_the_chain_where_it_was_written() {
        let after = parsed("use words\nword U+22A5 never\n");
        let before = parsed("word U+22A5 never\nuse words\n");
        let to = |map: &Map| found(map, '\u{22A5}').map(|e| e.to);
        assert_eq!(to(&after), Some(String::from("never")));
        assert_eq!(to(&before), Some(String::from("not")));
    }

    /// V18 for the two new line kinds: every line of a map, layered one
    /// at a time the way a `--map` flag arrives, is the same map as the
    /// file -- origins aside, which differ by design (V20).
    #[test]
    fn a_map_read_line_by_line_as_flags_is_the_same_map() {
        let text = "use words\nword U+2234 thus\nU+2014 --\n= t ascii:x\n";
        let flags = as_flags(text);
        assert_eq!(flags.clone().map(forget), parse(text).map(forget));
        let classes = flags.map(|map| map.classes().len());
        assert_eq!(classes, Ok(1));
    }

    /// Each line layered alone, labelled as the argv slot it would hold.
    fn as_flags(text: &str) -> Result<Map, Error> {
        text.lines().enumerate().try_fold(
            Map::default(),
            |map, (index, line)| {
                map.layer(line, &|_| Origin::Argument { index })
            },
        )
    }

    /// The entries with their origins dropped.
    fn forget(map: Map) -> Vec<(String, String, bool)> {
        let entries = map.entries().iter();
        entries
            .map(|e| (e.from.clone(), e.to.clone(), e.word))
            .collect()
    }

    /// Every named map parses, is pure ASCII, offers only ASCII, and
    /// pulls in no other map: a `use` inside one would be read again on
    /// every `use` of it, and a map naming itself would never finish.
    #[test]
    fn every_named_map_is_well_formed_data() {
        for (name, text) in super::NAMED {
            assert!(text.is_ascii(), "{name}");
            let use_lines = text.lines().filter(|l| l.starts_with("use "));
            assert_eq!(use_lines.count(), 0, "{name}");
            let map = parse(text);
            assert!(map.is_ok(), "{name}");
            for entry in map.unwrap_or_default().entries() {
                assert!(entry.to.is_ascii(), "{name}: {}", entry.from);
            }
        }
    }

    /// V51 names what the `words` map targets. Written out, for the
    /// reason `the_builtin_map_targets_what_v26_names` states.
    #[test]
    fn the_words_map_targets_what_v51_names() {
        let map = parsed(super::WORDS);
        let word = |from, to: &str| {
            let held = found(&map, from).map(|e| (e.to, e.word));
            assert_eq!(held, Some((to.to_owned(), true)), "{from:?}");
        };
        word('\u{22A5}', "not");
        word('\u{2234}', "so");
        word('\u{2235}', "because");
        word('\u{2200}', "all");
        word('\u{2208}', "in");
        word('\u{2203}', "exists");
        rewrites(&map, '\u{2260}', "!=");
        rewrites(&map, '\u{2192}', "->");
        rewrites(&map, '\u{21D2}', "=>");
        assert_eq!(map.entries().len(), 9);
    }

    /// V26 holds: the DEFAULT map rewrites none of the notation `words`
    /// covers. Opting in is the only way those symbols are touched.
    #[test]
    fn the_builtin_map_leaves_the_notation_alone() {
        let builtin = parsed(super::BUILTIN);
        for entry in parsed(super::WORDS).entries() {
            let from = entry.from.chars().next().unwrap_or_default();
            assert_eq!(found(&builtin, from), None, "{from:?}");
        }
    }
}
