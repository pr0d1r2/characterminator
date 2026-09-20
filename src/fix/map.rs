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

use crate::fix::class;
use crate::fix::codepoint::decode;
use crate::fix::family::{ROOT, Tree};
use crate::fix::{Class, Error, Family, MapEntry};
use crate::rules::Origin;
use std::cmp::Reverse;

/// The builtin map, in the `.ctrm-map` grammar (`src/charset:V22`).
///
/// Public as TEXT for the reason the sets file is: the builtin is the
/// lowest source of the precedence chain (`src/rules:V19`) and arrives
/// there the same way a `.ctrm-map` or a `--map` flag does. One grammar,
/// one parser, and an entry a user overrides by declaring it again.
pub const BUILTIN: &str = include_str!("map.ctrm-map");

/// A declared source matched at the current position.
pub(crate) struct Match {
    pub(crate) len: usize,
    pub(crate) to: String,
}

/// What a matched source rewrites to: a literal replacement from a map
/// entry, or a class whose answer depends on the fidelity family (V28).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    To(String),
    Class(usize),
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
        for candidate in &self.candidates {
            let Some(to) = self.try_at(candidate, rest, allowed)? else {
                continue;
            };
            return Ok(Some(Match {
                len: candidate.source.len(),
                to,
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
            Target::To(to) => Ok(Some(to.clone())),
            Target::Class(index) => match self.classes.get(*index) {
                Some(class) => {
                    class::resolve(class, &self.tree, self.fidelity(), allowed)
                }
                None => Ok(None),
            },
        }
    }

    /// One line, dispatched by its shape.
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
        if let Some(rest) = body.strip_prefix("family ") {
            return self.declare(rest, number);
        }
        if let Some(rest) = body.strip_prefix("= ") {
            return self.classify(rest, number);
        }
        self.add(body, number, origin)
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
        body: &str,
        number: usize,
        origin: &dyn Fn(usize) -> Origin,
    ) -> Result<(), Error> {
        let entry = entry(body, origin(number))
            .ok_or(Error::Syntax { line: number })?;
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
                target: Target::To(entry.to.clone()),
            });
        }
        self.index_classes();
        self.candidates
            .sort_by_key(|held| Reverse(held.source.len()));
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
fn entry(body: &str, origin: Origin) -> Option<MapEntry> {
    let mut words = body.split_whitespace();
    let from = decode(words.next()?)?;
    let to = match words.next() {
        Some(word) => decode(word)?,
        None => String::new(),
    };
    if from.is_empty() || words.next().is_some() {
        return None;
    }
    Some(MapEntry { from, to, origin })
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

    /// Every replacement the builtin offers is itself ASCII. A map whose
    /// answer was another non-ASCII character would move the problem
    /// rather than solve it, and `fix` would report the result as a
    /// violation of the same rule (V4).
    #[test]
    fn every_builtin_replacement_is_ascii() {
        for entry in parsed(super::BUILTIN).entries() {
            assert!(entry.to.is_ascii(), "{} -> {}", entry.from, entry.to);
        }
    }
}
