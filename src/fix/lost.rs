//! Putting back what a lossy decode threw away, from a copy that still has it.
//!
//! U+FFFD carries no information about the character it replaced, so the
//! map cannot restore it (`map::LOST`). An earlier copy of the same file
//! can: where a line of it matches the damaged line everywhere except at
//! the U+FFFD positions, the characters there ARE the lost ones. Anything
//! less certain -- a different length, two different candidates -- is left
//! alone, because a guess here would be a silent rewrite of the author's
//! text.

use super::map::LOST;

/// `current` with every U+FFFD that `original` can vouch for put back.
///
/// A line is restored when the line of `original` at the same number
/// matches it, else when exactly one distinct line of `original` does. A
/// match is the same number of characters, equal at every position that
/// is not U+FFFD, and a real character at every position that is.
pub(crate) fn restored(current: &str, original: &str) -> String {
    let before: Vec<&str> = original.split('\n').collect();
    let lines: Vec<String> = current
        .split('\n')
        .enumerate()
        .map(|(number, line)| healed(line, before.get(number), &before))
        .collect();
    lines.join("\n")
}

/// How many characters `restored` put back.
pub(crate) fn lost_in(text: &str) -> usize {
    text.chars().filter(|&ch| ch == LOST).count()
}

fn healed(line: &str, same: Option<&&str>, all: &[&str]) -> String {
    if !line.contains(LOST) {
        return line.to_owned();
    }
    same.and_then(|held| mend(line, held))
        .or_else(|| only(line, all))
        .unwrap_or_else(|| line.to_owned())
}

/// The one restoration of `line` that every matching line agrees on.
fn only(line: &str, all: &[&str]) -> Option<String> {
    let mut found: Option<String> = None;
    for mended in all.iter().filter_map(|held| mend(line, held)) {
        if found.as_ref().is_some_and(|first| *first != mended) {
            return None;
        }
        found = Some(mended);
    }
    found
}

/// `line` with each U+FFFD replaced by the character of `original` at its
/// position, or `None` when the two do not line up everywhere else.
fn mend(line: &str, original: &str) -> Option<String> {
    let mut theirs = original.chars();
    let mut out = String::with_capacity(original.len());
    for ch in line.chars() {
        let was = theirs.next()?;
        if ch != LOST && ch != was {
            return None;
        }
        out.push(if ch == LOST { was } else { ch });
    }
    theirs.next().is_none().then_some(out)
}

#[cfg(test)]
#[path = "lost_test.rs"]
mod tests;
