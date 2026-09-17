//! The skeleton every data-file kind shares: one entry per line.
//!
//! The three grammars (`.ctrm`, `.ctrm-map`, `.ctrm-sets`) differ only in
//! what a single line MEANS. How lines are FOUND -- one entry per line, a
//! number sign starting a comment, blank lines ignored -- is identical for
//! all three, so it lives here once and each kind's line parser is passed
//! in as a function. That is what lets a flag twin and a file reach the
//! SAME parser (V18): a file is many lines, a flag is exactly one.
//!
//! Only the `.ctrm` parser lives in this node. The map and sets grammars
//! belong to `src/fix` and `src/charset`, so their parsers arrive here as
//! arguments rather than as calls into a sibling.

use crate::rules::Origin;
use std::fmt;
use std::path::Path;

/// A line that could not be read, and where it came from.
///
/// The origin travels with the message rather than a bare line number,
/// because "line 3" names nothing when two files and four flags all
/// contributed lines to the same configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub origin: Origin,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", describe(&self.origin), self.message)
    }
}

impl std::error::Error for ParseError {}

/// Spell an origin the one way V20 fixes: `<file>:<line>`, `argv[<n>]` or
/// `builtin:<line>`. One spelling serves both a parse error and what
/// `explain` prints, so the two can never drift apart.
pub fn describe(origin: &Origin) -> String {
    match origin {
        Origin::File { path, line } => format!("{}:{line}", path.display()),
        Origin::Argument { index } => format!("argv[{index}]"),
        Origin::Builtin { line } => format!("builtin:{line}"),
    }
}

/// Build a parse error at an origin.
pub fn error(origin: Origin, message: impl Into<String>) -> ParseError {
    ParseError {
        origin,
        message: message.into(),
    }
}

/// A line carries no entry when it is blank or a comment.
///
/// The number sign comments only when it is the FIRST non-blank character
/// of the line. Later in a line it stays literal, so a path may contain
/// one and a map entry may mention one; a map line whose subject IS the
/// number sign writes it in the `U+0023` form the map grammar already has.
fn is_skippable(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// Parse every entry-bearing line of `text`, numbering origins from one.
///
/// Line numbers count ALL lines, including the blank and comment lines
/// that yield no entry, because the number has to lead a reader to the
/// right line of the file they are looking at.
///
/// A line reaches its parser TRIMMED, which is what lets an indented
/// file line and a flag value be the same line. The consequence is worth
/// stating for the kinds whose parser lives elsewhere: a trailing field
/// that is meant to be EMPTY cannot be written as trailing blanks, so a
/// grammar wanting one -- the map's empty replacement, which is its
/// explicit delete -- needs a spelling of its own.
pub fn parse_lines<T, O, P>(
    text: &str,
    origin: O,
    parse: P,
) -> Result<Vec<T>, ParseError>
where
    O: Fn(usize) -> Origin,
    P: Fn(&str, Origin) -> Result<T, ParseError>,
{
    let mut entries = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if is_skippable(line) {
            continue;
        }
        entries.push(parse(line.trim(), origin(index.saturating_add(1)))?);
    }
    Ok(entries)
}

/// A whole data file of one kind, read from `path`.
pub fn parse_file<T, P>(
    text: &str,
    path: &Path,
    parse: P,
) -> Result<Vec<T>, ParseError>
where
    P: Fn(&str, Origin) -> Result<T, ParseError>,
{
    let origin = |line| Origin::File {
        path: path.to_path_buf(),
        line,
    };
    parse_lines(text, origin, parse)
}

/// A compiled-in data file, in the same grammar as a user's file.
pub fn parse_builtin<T, P>(text: &str, parse: P) -> Result<Vec<T>, ParseError>
where
    P: Fn(&str, Origin) -> Result<T, ParseError>,
{
    parse_lines(text, |line| Origin::Builtin { line }, parse)
}

/// One inline flag: `--rule`, `--map` or `--set`.
///
/// A flag's value is EXACTLY one line of the file kind it twins (V18).
/// Several lines in one value are refused rather than quietly split,
/// because accepting them would make the flag a second way to write a
/// whole file, and a twin that can do more than its sibling is not a twin.
///
/// A blank or comment value yields no entry, exactly as such a line in a
/// file yields none. That is what makes V18's property hold for a file
/// carrying comments: one flag per line, comments included, still equals
/// the file.
pub fn parse_flag<T, P>(
    value: &str,
    index: usize,
    parse: P,
) -> Result<Option<T>, ParseError>
where
    P: Fn(&str, Origin) -> Result<T, ParseError>,
{
    let origin = Origin::Argument { index };
    if value.lines().nth(1).is_some() {
        return Err(error(origin, "a flag value is exactly one line"));
    }
    if is_skippable(value) {
        return Ok(None);
    }
    parse(value.trim(), origin).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in line parser: it keeps the text it was handed and the
    /// origin it was given, which is all these tests are about.
    fn keep(
        text: &str,
        origin: Origin,
    ) -> Result<(String, Origin), ParseError> {
        Ok((text.to_string(), origin))
    }

    fn refuse(_: &str, origin: Origin) -> Result<(), ParseError> {
        Err(error(origin, "no"))
    }

    fn path() -> &'static Path {
        Path::new(".ctrm")
    }

    #[test]
    fn blank_and_comment_lines_yield_no_entry() {
        let text = "\n# a comment\n   \n*.md caveman\n";
        let entries = parse_file(text, path(), keep).unwrap_or_default();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn line_numbers_count_the_skipped_lines() {
        let text = "# one\n\n*.md caveman\n";
        let entries = parse_file(text, path(), keep).unwrap_or_default();
        let origin = entries.first().map(|entry| entry.1.clone());
        assert_eq!(
            origin,
            Some(Origin::File {
                path: path().to_path_buf(),
                line: 3,
            })
        );
    }

    #[test]
    fn a_number_sign_after_the_first_field_stays_literal() {
        let entries = parse_file("a#b ascii\n", path(), keep);
        let texts = entries.unwrap_or_default();
        assert_eq!(
            texts.first().map(|entry| entry.0.as_str()),
            Some("a#b ascii")
        );
    }

    #[test]
    fn a_builtin_line_is_numbered_like_a_file() {
        let entries = parse_builtin("x ascii\n", keep).unwrap_or_default();
        let origin = entries.first().map(|entry| entry.1.clone());
        assert_eq!(origin, Some(Origin::Builtin { line: 1 }));
    }

    #[test]
    fn a_flag_carries_its_argv_index() {
        let entry = parse_flag("*.md caveman", 7, keep).unwrap_or(None);
        assert_eq!(
            entry.map(|entry| entry.1),
            Some(Origin::Argument { index: 7 })
        );
    }

    #[test]
    fn a_multi_line_flag_value_is_refused() {
        let entry = parse_flag("a ascii\nb ascii", 1, keep);
        assert!(entry.is_err());
    }

    #[test]
    fn a_comment_flag_value_yields_no_entry() {
        let entry = parse_flag("# nothing here", 1, keep).unwrap_or(None);
        assert!(entry.is_none());
    }

    #[test]
    fn a_failing_line_stops_the_file_and_names_its_origin() {
        let failure = parse_file("\na ascii\n", path(), refuse);
        let shown = failure.err().map(|error| error.to_string());
        assert_eq!(shown, Some(".ctrm:2: no".to_string()));
    }
}
