//! Put back what a lossy decode destroyed, from a copy that still has it.
//!
//! U+FFFD carries no information: a map entry from it would turn every lost
//! character into the same replacement, so the map refuses one
//! (`src/fix:V105`). The original can only come from somewhere else -- the
//! staged copy or `HEAD` -- and only when that copy shows it unambiguously:
//! the line is the same length and identical everywhere but at the `U+FFFD`
//! positions, where the copy holds a real character.
//!
//! Pure over two texts. Where the copies come from is the caller's
//! (`src/cli`), and a line that cannot be recovered is left alone, so the
//! walk reports its `U+FFFD` as it always did.

use crate::fix::Rewrite;
use crate::scan::{Hit, Position};

/// The replacement character a lossy decoder writes for bytes it could not read.
pub(crate) const LOST: char = '\u{FFFD}';

/// `text` with every `U+FFFD` that a reference line restores put back, and
/// one rewrite for each, at its place in `text`.
///
/// `references` are tried in order, so a staged copy wins over `HEAD`.
pub(crate) fn recover(
    text: &str,
    references: &[&str],
) -> (String, Vec<Rewrite>) {
    let copies: Vec<Vec<&str>> = references
        .iter()
        .map(|r| r.split_inclusive('\n').collect())
        .collect();
    let mut done = Recovery::default();
    for (at, line) in text.split_inclusive('\n').enumerate() {
        done.line(line, at, original_of(line, at, &copies));
    }
    (done.output, done.rewrites)
}

/// What the walk over the lines has produced so far.
#[derive(Default)]
struct Recovery {
    output: String,
    rewrites: Vec<Rewrite>,
    byte: usize,
}

impl Recovery {
    /// One line, as it stands or as its original shows it.
    fn line(&mut self, line: &str, at: usize, original: Option<&str>) {
        let place = Place {
            at,
            byte: self.byte,
        };
        let found = original.into_iter();
        self.rewrites
            .extend(found.flat_map(|o| restored(line, o, place)));
        self.output.push_str(original.unwrap_or(line));
        self.byte = self.byte.saturating_add(line.len());
    }
}

/// Where a line starts: its 0-based index and its byte offset.
#[derive(Clone, Copy)]
struct Place {
    at: usize,
    byte: usize,
}

/// The reference line `line` was before it lost characters, if one copy
/// shows it unambiguously. A line without `U+FFFD` has nothing to recover.
fn original_of<'r>(
    line: &str,
    at: usize,
    references: &[Vec<&'r str>],
) -> Option<&'r str> {
    if !line.contains(LOST) {
        return None;
    }
    references
        .iter()
        .find_map(|lines| in_lines(line, at, lines))
}

/// The line of one copy that `line` is a lossy reading of: the one at the
/// same line number, else the only distinct one anywhere in the copy.
fn in_lines<'r>(line: &str, at: usize, lines: &[&'r str]) -> Option<&'r str> {
    if let Some(same) = lines.get(at).filter(|c| fits(line, c)) {
        return Some(same);
    }
    let mut fitting = lines.iter().copied().filter(|c| fits(line, c));
    let first = fitting.next()?;
    fitting.all(|c| c == first).then_some(first)
}

/// Whether `lossy` reads as `original`: the same characters throughout,
/// except that a `U+FFFD` stands where the original has a real character.
fn fits(lossy: &str, original: &str) -> bool {
    lossy.chars().count() == original.chars().count()
        && lossy
            .chars()
            .zip(original.chars())
            .all(|(l, o)| l == o || (l == LOST && o != LOST))
}

/// One rewrite per `U+FFFD` of `line` that `original` restores.
fn restored(
    line: &str,
    original: &str,
    place: Place,
) -> impl Iterator<Item = Rewrite> {
    let chars = line.char_indices().zip(original.chars());
    chars
        .enumerate()
        .filter(|(_, ((_, l), o))| *l == LOST && *o != LOST)
        .map(move |(column, ((offset, _), o))| place.rewrite(column, offset, o))
}

impl Place {
    /// The rewrite of the `U+FFFD` at char `column` and byte `offset` of
    /// this line, to `to`.
    fn rewrite(self, column: usize, offset: usize, to: char) -> Rewrite {
        let position = Position {
            line: self.at.saturating_add(1),
            column: column.saturating_add(1),
            byte: self.byte.saturating_add(offset),
        };
        Rewrite {
            hit: Hit {
                position,
                character: LOST,
            },
            to: to.to_string(),
        }
    }
}

#[cfg(test)]
#[path = "lost_test.rs"]
mod tests;
