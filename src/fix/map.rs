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

use crate::fix::codepoint::decode;
use crate::fix::{Error, MapEntry};
use crate::rules::Origin;
use std::cmp::Reverse;

/// A declared source matched at the current position.
pub(crate) struct Match {
    pub(crate) len: usize,
    pub(crate) to: String,
}

/// The effective map: every entry that survived precedence, in the order
/// `resolve_at` wants them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Map {
    entries: Vec<MapEntry>,
}

impl Map {
    /// Parse map lines. `origin` labels a 1-based line number, so the same
    /// parser serves a file, a builtin and a `--map` flag (`src/rules:V18`).
    pub fn parse(
        source: &str,
        origin: &dyn Fn(usize) -> Origin,
    ) -> Result<Self, Error> {
        let mut map = Self::default();
        for (index, line) in source.lines().enumerate() {
            map.read(line, index.saturating_add(1), origin)?;
        }
        map.entries.sort_by_key(|entry| Reverse(entry.from.len()));
        Ok(map)
    }

    /// The effective entries, longest source first. Each carries its origin,
    /// which is how `explain` answers "why" (`src/rules:V20`).
    #[must_use]
    pub fn entries(&self) -> &[MapEntry] {
        &self.entries
    }

    /// The longest declared source that both matches here and covers at
    /// least one disallowed character. A span of allowed characters is never
    /// a violation, so it is never rewritten (V6).
    pub(crate) fn resolve_at(
        &self,
        rest: &str,
        allowed: &dyn Fn(char) -> bool,
    ) -> Option<Match> {
        self.entries
            .iter()
            .filter(|entry| rest.starts_with(&entry.from))
            .find(|entry| violates(&entry.from, allowed))
            .map(|entry| Match {
                len: entry.from.len(),
                to: entry.to.clone(),
            })
    }

    /// How many times a replacement may be rewritten again before the map is
    /// called cyclic. An acyclic chain visits each entry at most once, so
    /// anything longer has come back to an entry it already used.
    pub(crate) fn budget(&self) -> usize {
        self.entries.len().saturating_add(1)
    }

    /// A later line wins over an earlier one for the same source, which is
    /// the precedence the config chain expects (`src/rules:V19`).
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
        let entry = entry(body, origin(number))
            .ok_or(Error::Syntax { line: number })?;
        self.entries.retain(|held| held.from != entry.from);
        self.entries.push(entry);
        Ok(())
    }
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
}
