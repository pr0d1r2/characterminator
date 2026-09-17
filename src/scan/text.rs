//! Positions over a `&str`: which character sits where.
//!
//! ALLOWED-NESS IS A PARAMETER. This node answers "what character is
//! where", never "is it allowed": the sets live in `src/charset` and the
//! precedence that picks one lives in `src/rules`, so the test arrives as
//! a closure rather than being looked up here (`src:V39`).

use super::{Hit, Position};

/// A walking cursor over one text.
///
/// It carries the line and column ONLY: the byte offset comes from the
/// `char_indices` walk itself, and a second copy of a number that is
/// already exact is a second chance to get it wrong.
struct Cursor {
    line: usize,
    column: usize,
}

impl Cursor {
    /// Both units are 1-BASED, because every editor and every compiler
    /// diagnostic a reader compares this against is (`V12`).
    const fn start() -> Self {
        Self { line: 1, column: 1 }
    }

    /// The column counts CHARACTERS, so it moves once per `char` whatever
    /// that char weighs in bytes.
    ///
    /// A line ends at the line feed, and nothing else. In a CRLF file the
    /// carriage return is an ordinary character on the line it closes:
    /// one terminator, rather than two spellings of one that then have to
    /// agree. A lone carriage return therefore does NOT start a line --
    /// the old Mac ending is deliberately not honoured, because guessing
    /// between the two would make the line number depend on a heuristic.
    fn advance(&mut self, character: char) {
        if character == '\n' {
            self.line = self.line.saturating_add(1);
            self.column = 1;
        } else {
            self.column = self.column.saturating_add(1);
        }
    }

    const fn position(&self, byte: usize) -> Position {
        Position {
            line: self.line,
            column: self.column,
            byte,
        }
    }
}

/// Locate every character the caller's test rejects.
///
/// `allowed` is the caller's rule and the only opinion in the call: a
/// `false` makes a hit, a `true` makes nothing. Hits come out in byte
/// order, which is the within-a-file half of V12's sort order; ordering
/// ACROSS files is `super::sort_hits`.
pub fn scan_str<F>(text: &str, allowed: F) -> Vec<Hit>
where
    F: Fn(char) -> bool,
{
    let mut cursor = Cursor::start();
    let mut hits = Vec::new();
    for (byte, character) in text.char_indices() {
        if !allowed(character) {
            hits.push(Hit {
                position: cursor.position(byte),
                character,
            });
        }
        cursor.advance(character);
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::scan_str;
    use crate::scan::{Hit, Position};

    /// Zero width space: invisible, two tokens wide, and the reason this
    /// tool exists.
    const ZWSP: char = '\u{200b}';

    /// A stand-in for the rule a real caller resolves, so the tests here
    /// exercise the WALK rather than any set.
    fn ascii_only(character: char) -> bool {
        character.is_ascii()
    }

    fn hit(line: usize, column: usize, byte: usize, found: char) -> Hit {
        Hit {
            position: Position { line, column, byte },
            character: found,
        }
    }

    #[test]
    fn column_counts_characters_not_bytes() {
        // "e with acute" weighs two bytes, so a byte column would call
        // the zero width space after it column 3.
        let hits = scan_str("\u{00e9}\u{200b}", ascii_only);

        let expected = vec![hit(1, 1, 0, '\u{00e9}'), hit(1, 2, 2, ZWSP)];

        assert_eq!(hits, expected);
    }

    #[test]
    fn carriage_return_line_feed_starts_one_new_line() {
        // The carriage return is column 2 of line 1; the hit lands at
        // column 1 of line 2, byte 3.
        let hits = scan_str("a\r\n\u{200b}b", ascii_only);

        assert_eq!(hits, vec![hit(2, 1, 3, ZWSP)]);
    }

    #[test]
    fn hits_arrive_in_byte_order_within_one_text() {
        let hits = scan_str("\u{200b}a\u{00a0}", ascii_only);

        assert_eq!(hits, vec![hit(1, 1, 0, ZWSP), hit(1, 3, 4, '\u{00a0}')]);
    }

    #[test]
    fn the_test_is_the_callers_and_nothing_is_reported_without_it() {
        let hits = scan_str("\u{200b}\u{00a0}", |_| true);

        assert_eq!(hits, Vec::new());
    }
}
