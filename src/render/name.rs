//! The words BOTH forms share: how a code point is written, and the names
//! of a level and of a counting method.
//!
//! They live here rather than in either form because a name that drifted
//! between the human line and the json document would make the two reports
//! disagree about the same fact.

use crate::lint::Level;
use crate::tokens::Method;
use std::fmt::{self, Display, Formatter};

/// `U+XXXX`: uppercase, at least four digits.
///
/// Uppercase and the four-digit floor are the Unicode convention, and this
/// is the spelling the root spec's interface section prints and that
/// `src/scan:V12` requires of a violation.
pub fn codepoint(character: char) -> String {
    Codepoint(character).to_string()
}

/// [`codepoint`], written straight into a report rather than allocated.
#[derive(Debug, Clone, Copy)]
pub struct Codepoint(pub char);

impl Display for Codepoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "U+{:04X}", u32::from(self.0))
    }
}

/// The rustc and clippy words for a level (`src/lint:V36`).
///
/// These are the STABLE identifiers: the json contract carries them, so they
/// track the level names a rule line writes rather than any message text.
pub const fn level_name(level: Level) -> &'static str {
    match level {
        Level::Allow => "allow",
        Level::Warn => "warn",
        Level::Deny => "deny",
        Level::Forbid => "forbid",
    }
}

/// How a count was arrived at (`src/tokens:V10`).
///
/// The json form spells the method out; the human form writes a tilde
/// instead, because a person reads a tilde faster than a word.
pub const fn method_name(method: Method) -> &'static str {
    match method {
        Method::Estimate => "estimate",
        Method::Bpe => "bpe",
    }
}

#[cfg(test)]
mod tests {
    use super::{codepoint, level_name, method_name};
    use crate::lint::Level;
    use crate::tokens::Method;

    #[test]
    fn a_code_point_is_padded_to_four_uppercase_digits() {
        assert_eq!(codepoint('\u{a0}'), "U+00A0");
    }

    #[test]
    fn a_code_point_past_four_digits_is_not_truncated() {
        // U+1F600 GRINNING FACE.
        assert_eq!(codepoint('\u{1F600}'), "U+1F600");
    }

    #[test]
    fn levels_use_the_rustc_words() {
        assert_eq!(level_name(Level::Forbid), "forbid");
        assert_eq!(level_name(Level::Allow), "allow");
    }

    #[test]
    fn methods_name_themselves() {
        assert_eq!(method_name(Method::Estimate), "estimate");
        assert_eq!(method_name(Method::Bpe), "bpe");
    }
}
