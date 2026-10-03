//! Turning text into a json string literal.
//!
//! The output is PURE ASCII. A tool whose whole purpose is to drive
//! characters out of a file has no business emitting a report that carries
//! them back in, and an ASCII document survives every pipe, terminal and log
//! it will be read through. So every code point at or above `0x80` leaves as
//! a `\u` escape, and the escaping is written by hand: no serde.
//!
//! Written INTO a caller's buffer (R17): a report of two million findings
//! that built one `String` per escaped character spent its time in the
//! allocator. Text that needs no escape at all -- nearly every path -- is
//! copied in one piece.

use std::fmt::Write;

/// A json string literal: quoted, escaped, and pure ASCII.
pub(crate) fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2));
    push(&mut out, text);
    out
}

/// [`string`], appended to `out`.
pub(super) fn push(out: &mut String, text: &str) {
    out.push('"');
    inner(out, text);
    out.push('"');
}

/// [`push`] without the quotes: the escaped body of a literal whose other
/// parts the caller writes.
pub(super) fn inner(out: &mut String, text: &str) {
    if text.bytes().all(plain) {
        out.push_str(text);
    } else {
        text.chars().for_each(|character| escaped(out, character));
    }
}

/// A byte json carries as itself: printable ASCII, less the two that
/// would end or escape the literal.
const fn plain(byte: u8) -> bool {
    matches!(byte, 0x20..=0x7E) && byte != b'"' && byte != b'\\'
}

/// One character as json writes it.
///
/// The named escapes come first because they are shorter and because a
/// reader recognises `\n` where `\u000a` reads as noise.
fn escaped(out: &mut String, character: char) {
    let named = match character {
        '"' => "\\\"",
        '\\' => "\\\\",
        '\n' => "\\n",
        '\r' => "\\r",
        '\t' => "\\t",
        '\u{8}' => "\\b",
        '\u{c}' => "\\f",
        _ => return unnamed(out, character),
    };
    out.push_str(named);
}

/// Everything without a named escape: printable ASCII passes through, the
/// rest becomes a `\u` escape.
///
/// DEL (`U+007F`) is escaped although json does not demand it: it is a
/// control character, and a report is read in a terminal.
fn unnamed(out: &mut String, character: char) {
    let code = u32::from(character);
    if (0x20..0x7F).contains(&code) {
        return out.push(character);
    }
    if code > 0xFFFF {
        return surrogates(out, code);
    }
    unit(out, code);
}

/// One `\uXXXX` escape: lowercase hex, four digits. Writing into a
/// `String` cannot fail, so there is no error to pass on.
fn unit(out: &mut String, code: u32) {
    let _infallible = write!(out, "\\u{code:04x}");
}

/// An astral code point as the surrogate pair json requires.
///
/// The two additions are written as `|`, which is exact here rather than
/// merely convenient: the low ten bits of both lead constants are zero and
/// each operand is under `0x400`, so the OR sets exactly the bits an
/// addition would have carried into. It also cannot overflow, which spares
/// this from being the one place that has to prove it can.
fn surrogates(out: &mut String, code: u32) {
    let rest = code.saturating_sub(0x1_0000);
    unit(out, 0xD800 | (rest >> 10));
    unit(out, 0xDC00 | (rest & 0x3FF));
}

#[cfg(test)]
mod tests {
    use super::string;

    #[test]
    fn plain_ascii_is_left_alone() {
        assert_eq!(string("src/a.rs"), "\"src/a.rs\"");
    }

    #[test]
    fn a_quote_is_escaped() {
        assert_eq!(string("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn a_backslash_is_escaped() {
        assert_eq!(string("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn the_named_control_characters_use_their_short_escape() {
        assert_eq!(string("\n\r\t"), "\"\\n\\r\\t\"");
        assert_eq!(string("\u{8}\u{c}"), "\"\\b\\f\"");
    }

    #[test]
    fn an_unnamed_control_character_becomes_a_u_escape() {
        assert_eq!(string("\u{1}"), "\"\\u0001\"");
    }

    #[test]
    fn delete_is_escaped_although_json_does_not_demand_it() {
        assert_eq!(string("\u{7f}"), "\"\\u007f\"");
    }

    #[test]
    fn a_non_ascii_code_point_leaves_as_a_u_escape() {
        // U+2014 EM DASH, written as an escape because the source is ASCII.
        assert_eq!(string("\u{2014}"), "\"\\u2014\"");
    }

    #[test]
    fn a_non_ascii_code_point_keeps_its_neighbours() {
        assert_eq!(string("a\u{a0}b"), "\"a\\u00a0b\"");
    }

    #[test]
    fn an_astral_code_point_becomes_a_surrogate_pair() {
        // U+1F600 GRINNING FACE.
        assert_eq!(string("\u{1F600}"), "\"\\ud83d\\ude00\"");
    }
}
