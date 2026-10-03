//! Decoding one map token into the text it stands for.
//!
//! A token is either literal text or `U+XXXX` form, and a sequence is code
//! points joined with `+` (`U+1F44D+U+1F3FD`) or simply written out as
//! itself (`src/fix/SPEC.md` I.file). The two forms do not mix inside one
//! token: a token that starts with `U+` is read as code points to its end,
//! anything else is taken literally, so a literal `+` is never ambiguous.
//!
//! `U+` form is what lets a compiled-in data file stay pure ASCII
//! (`src/charset:V22`), which is the same rule this crate's own source
//! obeys.

use crate::charset::code_points;

/// The text a token stands for, or `None` if it is not a well-formed token.
/// The `U+` form is read by `src/charset`'s one decoder, so a map token
/// and a set member cannot disagree about what a code point is.
pub(super) fn decode(token: &str) -> Option<String> {
    if token.starts_with("U+") {
        code_points(token)
    } else {
        Some(token.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn reads_one_code_point() {
        assert_eq!(decode("U+2014"), Some(String::from("\u{2014}")));
    }

    #[test]
    fn reads_a_joined_sequence() {
        let want = String::from("\u{1F44D}\u{1F3FD}");
        assert_eq!(decode("U+1F44D+U+1F3FD"), Some(want));
    }

    #[test]
    fn takes_anything_else_literally() {
        assert_eq!(decode("--"), Some(String::from("--")));
        assert_eq!(decode("a+b"), Some(String::from("a+b")));
    }

    #[test]
    fn a_literal_sequence_needs_no_joiner() {
        let want = String::from("\u{2714}\u{FE0F}");
        assert_eq!(decode("\u{2714}\u{FE0F}"), want.into());
    }

    #[test]
    fn rejects_a_malformed_code_point() {
        assert_eq!(decode("U+"), None);
        assert_eq!(decode("U+ZZZZ"), None);
        assert_eq!(decode("U+2014+"), None);
        assert_eq!(decode("U+2014 U+2013"), None);
        assert_eq!(decode("U+D800"), None);
        assert_eq!(decode("U+110000"), None);
    }
}
