//! Turning text into a json string literal.
//!
//! The output is PURE ASCII. A tool whose whole purpose is to drive
//! characters out of a file has no business emitting a report that carries
//! them back in, and an ASCII document survives every pipe, terminal and log
//! it will be read through. So every code point at or above `0x80` leaves as
//! a `\u` escape, and the escaping is written by hand: no serde.

/// A json string literal: quoted, escaped, and pure ASCII.
pub fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        out.push_str(&escaped(character));
    }
    out.push('"');
    out
}

/// One character as json writes it.
///
/// The named escapes come first because they are shorter and because a
/// reader recognises `\n` where `\u000a` reads as noise.
fn escaped(character: char) -> String {
    match character {
        '"' => String::from("\\\""),
        '\\' => String::from("\\\\"),
        '\n' => String::from("\\n"),
        '\r' => String::from("\\r"),
        '\t' => String::from("\\t"),
        '\u{8}' => String::from("\\b"),
        '\u{c}' => String::from("\\f"),
        _ => unnamed(character),
    }
}

/// Everything without a named escape: printable ASCII passes through, the
/// rest becomes a `\u` escape.
///
/// DEL (`U+007F`) is escaped although json does not demand it: it is a
/// control character, and a report is read in a terminal.
fn unnamed(character: char) -> String {
    let code = u32::from(character);
    if (0x20..0x7F).contains(&code) {
        return String::from(character);
    }
    if code > 0xFFFF {
        return surrogates(code);
    }
    unit(code)
}

/// One `\uXXXX` escape: lowercase hex, four digits.
fn unit(code: u32) -> String {
    format!("\\u{code:04x}")
}

/// An astral code point as the surrogate pair json requires.
///
/// The two additions are written as `|`, which is exact here rather than
/// merely convenient: the low ten bits of both lead constants are zero and
/// each operand is under `0x400`, so the OR sets exactly the bits an
/// addition would have carried into. It also cannot overflow, which spares
/// this from being the one place that has to prove it can.
fn surrogates(code: u32) -> String {
    let rest = code.saturating_sub(0x1_0000);
    let high = 0xD800 | (rest >> 10);
    let low = 0xDC00 | (rest & 0x3FF);
    let mut out = unit(high);
    out.push_str(&unit(low));
    out
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
