//! The `.ctrm-sets` grammar: one line declares one named set.
//!
//! `<name> <member>...`, where a member is a literal character, `U+XXXX`,
//! `U+XXXX-U+YYYY`, or the name of another set (V25). The grammar lives
//! with its parser, and builtin preset data files are written in the same
//! grammar (V22), so this is the only reader of either.
//!
//! The unit is a LINE, not a file: `--set <line>` is the flag twin of a
//! `.ctrm-sets` line (`src/rules:V18`), and both must reach the same
//! parser. Reading files and ordering them by precedence belongs to
//! `src/rules`.

use super::CharRange;
use std::fmt;

/// What a set is built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetMember {
    /// A single character, written as itself.
    Literal(char),
    /// A span, written `U+XXXX` or `U+XXXX-U+YYYY`.
    Range(CharRange),
    /// The name of another set, to be composed in (V25).
    Named(String),
}

/// One parsed line: a name and the members granted to it.
///
/// Members stay UNRESOLVED here. A line can name a set declared later in
/// the same file, or in a file loaded after it, so resolving at parse time
/// would make a definition's meaning depend on the order lines are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetDefinition {
    pub name: String,
    pub members: Vec<SetMember>,
}

/// Why a line could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A name with no members: an empty set is almost always a typo.
    NoMembers { name: String },
    /// A name that could never be referenced as a member, because it would
    /// be read as something else (V25).
    UnusableName { name: String },
    /// A `U+XXXX` token that is not a code point.
    BadCodePoint { token: String },
    /// A range whose ends are the wrong way round.
    ReversedRange { token: String },
}

/// The prefix that marks a code point, per V22.
const CODE_POINT: &str = "U+";

/// The marker for a whole-line comment.
const COMMENT: char = '\u{0023}';

/// Why a token that is not a name cannot declare a set (V25).
const UNUSABLE: &str = "cannot name a set: a name is 2 or more \
                        characters and is never written U+XXXX";

impl ParseError {
    /// The offending token, whichever kind of fault this is.
    ///
    /// Every message is built as token-then-reason so that a line naming
    /// several bad tokens reports them in one recognisable shape.
    fn token(&self) -> &str {
        match self {
            Self::NoMembers { name } | Self::UnusableName { name } => name,
            Self::BadCodePoint { token } | Self::ReversedRange { token } => {
                token
            }
        }
    }

    /// What is wrong with it.
    const fn reason(&self) -> &'static str {
        match self {
            Self::NoMembers { .. } => "grants nothing",
            Self::UnusableName { .. } => UNUSABLE,
            Self::BadCodePoint { .. } => "is not a code point",
            Self::ReversedRange { .. } => "ends below where it starts",
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' {}", self.token(), self.reason())
    }
}

impl std::error::Error for ParseError {}

/// Read one line of the `.ctrm-sets` grammar.
///
/// `Ok(None)` is a line that declares nothing: blank, or a comment. Only a
/// WHOLE-line comment is recognised, because `#` is an ordinary character
/// and so a legal literal member; a trailing comment could not be told from
/// a grant of `#`.
///
/// # Errors
///
/// Returns a [`ParseError`] naming the offending token.
pub fn parse_line(line: &str) -> Result<Option<SetDefinition>, ParseError> {
    let content = line.trim();
    if content.is_empty() || content.starts_with(COMMENT) {
        return Ok(None);
    }
    let mut tokens = content.split_whitespace();
    let Some(name) = tokens.next() else {
        return Ok(None);
    };
    check_name(name)?;
    let members = tokens.map(member).collect::<Result<Vec<_>, _>>()?;
    definition(name, members)
}

/// Pair a name with its members, rejecting a set that grants nothing.
fn definition(
    name: &str,
    members: Vec<SetMember>,
) -> Result<Option<SetDefinition>, ParseError> {
    if members.is_empty() {
        return Err(ParseError::NoMembers {
            name: name.to_owned(),
        });
    }
    Ok(Some(SetDefinition {
        name: name.to_owned(),
        members,
    }))
}

/// A name must be a token that reads back as a NAME when used as a member.
///
/// A one-character name, or one spelled `U+XXXX`, would be read as a
/// literal or a code point everywhere it was referenced, so the set could
/// never be composed into another. Declaring it is a trap rather than a
/// grant, and this fails closed instead.
fn check_name(name: &str) -> Result<(), ParseError> {
    match member(name)? {
        SetMember::Named(_) => Ok(()),
        SetMember::Literal(_) | SetMember::Range(_) => {
            Err(ParseError::UnusableName {
                name: name.to_owned(),
            })
        }
    }
}

/// Classify one whitespace-separated token.
fn member(token: &str) -> Result<SetMember, ParseError> {
    if let Some(digits) = token.strip_prefix(CODE_POINT) {
        return code_form(token, digits);
    }
    let mut chars = token.chars();
    match (chars.next(), chars.next()) {
        (Some(only), None) => Ok(SetMember::Literal(only)),
        _ => Ok(SetMember::Named(token.to_owned())),
    }
}

/// A token that began `U+`: either one code point or a span.
fn code_form(token: &str, digits: &str) -> Result<SetMember, ParseError> {
    match digits.split_once('\u{002D}') {
        None => code_point(token, digits)
            .map(|point| SetMember::Range(CharRange::single(point))),
        Some((low, high)) => span(token, low, high),
    }
}

/// `U+XXXX-U+YYYY`: the high end repeats the prefix, per V22.
fn span(token: &str, low: &str, high: &str) -> Result<SetMember, ParseError> {
    let upper = high.strip_prefix(CODE_POINT).ok_or_else(|| {
        ParseError::BadCodePoint {
            token: token.to_owned(),
        }
    })?;
    let start = code_point(token, low)?;
    let end = code_point(token, upper)?;
    CharRange::new(start, end)
        .map(SetMember::Range)
        .ok_or_else(|| ParseError::ReversedRange {
            token: token.to_owned(),
        })
}

/// Hexadecimal digits to a character.
///
/// The digits are checked before conversion because `from_str_radix` also
/// accepts a leading sign, which would read `U++41` as a letter.
fn code_point(token: &str, digits: &str) -> Result<char, ParseError> {
    let bad = || ParseError::BadCodePoint {
        token: token.to_owned(),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let value = u32::from_str_radix(digits, 16).map_err(|_| bad())?;
    char::from_u32(value).ok_or_else(bad)
}

#[cfg(test)]
mod tests {
    use super::{CharRange, ParseError, SetMember, parse_line};

    fn members(line: &str) -> Result<Vec<SetMember>, ParseError> {
        parse_line(line)
            .map(|parsed| parsed.map(|set| set.members).unwrap_or_default())
    }

    #[test]
    fn blank_and_comment_declare_nothing() {
        assert_eq!(parse_line("   "), Ok(None));
        assert_eq!(parse_line("# a note"), Ok(None));
        assert_eq!(parse_line("  # indented note"), Ok(None));
    }

    #[test]
    fn a_single_character_is_a_literal() {
        let one = Ok(vec![SetMember::Literal('\u{00A9}')]);
        assert_eq!(members("legal \u{00A9}"), one);
    }

    #[test]
    fn a_two_character_token_is_a_set_name() {
        let named = Ok(vec![SetMember::Named("box".to_owned())]);
        assert_eq!(members("spec box"), named);
    }

    #[test]
    fn a_code_point_is_a_single_range() {
        let point = SetMember::Range(CharRange::single('\u{2014}'));
        assert_eq!(members("dash U+2014"), Ok(vec![point]));
    }

    #[test]
    fn a_span_is_a_range() {
        let range = CharRange {
            start: '\u{2500}',
            end: '\u{257F}',
        };
        let span = SetMember::Range(range);
        assert_eq!(members("box U+2500-U+257F"), Ok(vec![span]));
    }

    #[test]
    fn the_name_itself_is_not_a_member() {
        assert_eq!(
            parse_line("ab cd").map(|set| set.map(|s| s.name)),
            Ok(Some("ab".to_owned()))
        );
    }

    #[test]
    fn a_set_granting_nothing_is_rejected() {
        let err = Err(ParseError::NoMembers {
            name: "lonely".to_owned(),
        });
        assert_eq!(parse_line("lonely"), err);
    }

    #[test]
    fn a_one_character_name_is_rejected() {
        let err = Err(ParseError::UnusableName {
            name: "x".to_owned(),
        });
        assert_eq!(parse_line("x U+2014"), err);
    }

    #[test]
    fn a_name_spelled_as_a_code_point_is_rejected() {
        let err = Err(ParseError::UnusableName {
            name: "U+2014".to_owned(),
        });
        assert_eq!(parse_line("U+2014 U+2015"), err);
    }

    #[test]
    fn a_surrogate_is_not_a_code_point() {
        let err = Err(ParseError::BadCodePoint {
            token: "U+D800".to_owned(),
        });
        assert_eq!(parse_line("bad U+D800"), err);
    }

    #[test]
    fn nonsense_digits_are_rejected() {
        let err = Err(ParseError::BadCodePoint {
            token: "U+ZZZZ".to_owned(),
        });
        assert_eq!(parse_line("bad U+ZZZZ"), err);
    }

    #[test]
    fn a_signed_code_point_is_rejected() {
        let err = Err(ParseError::BadCodePoint {
            token: "U++41".to_owned(),
        });
        assert_eq!(parse_line("bad U++41"), err);
    }

    #[test]
    fn a_span_without_the_second_prefix_is_rejected() {
        let err = Err(ParseError::BadCodePoint {
            token: "U+0041-005A".to_owned(),
        });
        assert_eq!(parse_line("bad U+0041-005A"), err);
    }

    #[test]
    fn a_reversed_span_is_rejected() {
        let err = Err(ParseError::ReversedRange {
            token: "U+005A-U+0041".to_owned(),
        });
        assert_eq!(parse_line("bad U+005A-U+0041"), err);
    }

    #[test]
    fn a_hyphen_alone_is_a_literal() {
        let dash = Ok(vec![SetMember::Literal('\u{002D}')]);
        assert_eq!(members("punct \u{002D}"), dash);
    }

    #[test]
    fn several_members_keep_their_order() {
        let parsed = members("mix \u{00A9} U+2014 box");
        assert_eq!(
            parsed,
            Ok(vec![
                SetMember::Literal('\u{00A9}'),
                SetMember::Range(CharRange::single('\u{2014}')),
                SetMember::Named("box".to_owned()),
            ])
        );
    }
}
