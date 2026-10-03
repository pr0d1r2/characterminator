//! Pedantic detection (V37, V55): the four lints that need no vendored
//! data.
//!
//! WHAT A FINDING POINTS AT. Every finding here is a character that is
//! really in the file, so the scan node's `Hit` carries it unchanged and
//! the json and SARIF shapes stay what they were:
//!
//! - `unicode-space`: the space itself.
//! - `crlf`: the CARRIAGE RETURN of each CR LF pair.
//! - `trailing-whitespace`: the FIRST character of the run of whitespace
//!   a line ends on, one finding per line however long the run.
//! - `final-newline`: the LAST character of a file that does not end in a
//!   line feed -- where an editor shows "no newline at end of file".
//!
//! A position past the end of the file was the alternative for the last
//! one, and it was rejected: it would need a character that is not there,
//! and `Hit` would have to grow an "absent" case every renderer and the
//! `fix` node then had to handle.
//!
//! The four that need Unicode tables (V58) ask `ucd.rs`, the one file
//! that calls the Unicode crates, and point where it says:
//!
//! - `nfkc-compat`: the character NFKC folds and NFC keeps.
//! - `confusable`: a non-ASCII character drawn as ASCII (UTS #39).
//! - `not-nfc`: the first character where a segment and its NFC differ.
//! - `mixed-script`: the first character that empties a word's scripts.
//!
//! A character `unicode-space` claims is claimed by no other pedantic
//! lint: NFKC folds a no-break space and UTS #39 draws it as U+0020, and
//! one space with three names would be one finding said three ways.

use crate::lint::{Group, Lint, ucd};
use crate::scan::{Hit, located};

/// A space that is not U+0020.
pub(super) const UNICODE_SPACE: Lint =
    Lint::new("unicode-space", Group::Pedantic);

/// A line ended by CR LF rather than LF.
pub(crate) const CRLF: Lint = Lint::new("crlf", Group::Pedantic);

/// Whitespace between a line's last visible character and its end.
pub(super) const TRAILING_WHITESPACE: Lint =
    Lint::new("trailing-whitespace", Group::Pedantic);

/// A non-empty file whose last character is not a line feed.
pub(super) const FINAL_NEWLINE: Lint =
    Lint::new("final-newline", Group::Pedantic);

/// Text NFC would change.
pub(super) const NOT_NFC: Lint = Lint::new("not-nfc", Group::Pedantic);

/// A character NFKC folds and NFC keeps: fullwidth, ligature, superscript.
pub(super) const NFKC_COMPAT: Lint = Lint::new("nfkc-compat", Group::Pedantic);

/// One word written in two scripts.
pub(super) const MIXED_SCRIPT: Lint =
    Lint::new("mixed-script", Group::Pedantic);

/// A non-ASCII character UTS #39 draws as ASCII.
pub(super) const CONFUSABLE: Lint = Lint::new("confusable", Group::Pedantic);

/// The lints [`text_hits`] can fire, for the same question as below.
pub(crate) const TEXT_LINTS: [Lint; 2] = [NOT_NFC, MIXED_SCRIPT];

/// The lints [`char_lints`] can fire.
pub(crate) const CHAR_LINTS: [Lint; 3] =
    [UNICODE_SPACE, NFKC_COMPAT, CONFUSABLE];

/// The lints [`line_hits`] can fire, so a caller can ask whether any of
/// them is switched on before paying for a second walk of the text.
pub(crate) const LINE_LINTS: [Lint; 3] =
    [CRLF, TRAILING_WHITESPACE, FINAL_NEWLINE];

/// Every pedantic lint in CLAIM order (V55, V58): when two point at one
/// character, the earlier keeps it. The three walks each keep their own
/// order; this is the one that holds across them.
pub(super) const CLAIM_ORDER: [Lint; 8] = [
    UNICODE_SPACE,
    CRLF,
    TRAILING_WHITESPACE,
    FINAL_NEWLINE,
    NFKC_COMPAT,
    CONFUSABLE,
    NOT_NFC,
    MIXED_SCRIPT,
];

/// What `unicode-space` fires on: General_Category `Zs` minus U+0020.
///
/// V37 names NBSP and U+2000-U+200A. The rest of `Zs` is the same kind
/// of character -- a space a reader cannot tell from U+0020 -- and the
/// French NARROW NO-BREAK SPACE (U+202F) is the commonest of them in
/// real prose, so leaving it out would miss the case the lint exists for.
///
/// NOT here: U+2028 and U+2029 (`Zl`, `Zp`) are separators rather than
/// spaces; U+200B and U+180E are default-ignorable and therefore a
/// HAZARD, which forbids, so a pedantic name for them would never speak.
/// The test below holds this list against the standard library's
/// White_Space table, which contains every `Zs`.
const SPACES: [char; 16] = [
    '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}',
    '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}',
];

/// The pedantic lint one character fires on its own, wherever it sits.
pub(crate) fn unicode_space(character: char) -> Option<Lint> {
    SPACES.contains(&character).then_some(UNICODE_SPACE)
}

/// Every pedantic lint one character fires on its own, the strongest
/// claim first. ASCII is none of them, and pays no table lookup.
pub(crate) fn char_lints(character: char) -> [Option<Lint>; 3] {
    if character.is_ascii() {
        return [None; 3];
    }
    if let Some(space) = unicode_space(character) {
        return [Some(space), None, None];
    }
    let compat = ucd::compat(character).then_some(NFKC_COMPAT);
    let lookalike = ucd::lookalike(character).then_some(CONFUSABLE);
    [None, compat, lookalike]
}

/// Every finding that needs the characters around it to decide, in byte
/// order, `not-nfc` before `mixed-script` on the same character.
///
/// Pure ASCII answers NOTHING, at once (`src/render:R17`): it is NFC by
/// definition (no ASCII character decomposes or composes, and each passes the
/// quick check), and its words are Latin letters and Common digits, whose
/// script sets never intersect to empty. Otherwise the text is walked, not
/// copied into a list of hits: `mixed-script` streams, and `not-nfc` lays the
/// hits out only for a text the quick check cannot clear.
pub(crate) fn text_hits(text: &str) -> Vec<(Lint, Hit)> {
    if text.is_ascii() {
        return Vec::new();
    }
    let nfc = ucd::denormal_in(text).into_iter().map(|hit| (NOT_NFC, hit));
    let words = ucd::mixed_in(located(text));
    let words = words.into_iter().map(|hit| (MIXED_SCRIPT, hit));
    let mut found: Vec<(Lint, Hit)> = nfc.chain(words).collect();
    found.sort_by_key(|(_, hit)| hit.position.byte);
    found
}

/// Every line-shaped finding in a text, as the lint and the character it
/// points at, in byte order within each kind.
pub(crate) fn line_hits(text: &str) -> Vec<(Lint, Hit)> {
    let mut walk = Walk::default();
    for hit in located(text) {
        walk.step(hit);
    }
    walk.finish()
}

/// The state a line walk carries: the start of the whitespace run the
/// line currently ends on, and the character just seen.
#[derive(Default)]
struct Walk {
    run: Option<Hit>,
    last: Option<Hit>,
    found: Vec<(Lint, Hit)>,
}

impl Walk {
    /// A line ends at LF and nothing else, as the scan node counts lines,
    /// so a LONE carriage return is an ordinary character here: it is not
    /// `crlf`, and at a line's end it is whitespace like any other.
    fn step(&mut self, hit: Hit) {
        if hit.character == '\n' {
            self.end_line();
        } else if hit.character.is_whitespace() {
            self.run = self.run.or(Some(hit));
        } else {
            self.run = None;
        }
        self.last = Some(hit);
    }

    /// The CR of a CR LF is the TERMINATOR, so it is reported as `crlf`
    /// and is not counted as trailing whitespace too: `abc\r\n` is one
    /// finding, `abc \r\n` is two.
    fn end_line(&mut self) {
        let cr = self.last.filter(|hit| hit.character == '\r');
        let run = self.run.take();
        let trailing = run.filter(|start| cr != Some(*start));
        if let Some(start) = trailing {
            self.found.push((TRAILING_WHITESPACE, start));
        }
        if let Some(cr) = cr {
            self.found.push((CRLF, cr));
        }
    }

    /// An EMPTY file has no last character, so it is no finding: there
    /// is no line in it left unterminated. A file that ends mid-line also
    /// ends that line, so its trailing run is reported as any other.
    fn finish(mut self) -> Vec<(Lint, Hit)> {
        if let Some(last) = self.last.filter(|hit| hit.character != '\n') {
            if let Some(start) = self.run.take() {
                self.found.push((TRAILING_WHITESPACE, start));
            }
            self.found.push((FINAL_NEWLINE, last));
        }
        self.found
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CHAR_LINTS, CONFUSABLE, CRLF, FINAL_NEWLINE, LINE_LINTS, MIXED_SCRIPT,
        NFKC_COMPAT, NOT_NFC, SPACES, TEXT_LINTS, TRAILING_WHITESPACE,
        UNICODE_SPACE, char_lints, line_hits, text_hits, unicode_space,
    };
    use crate::lint::{Group, LINTS, Lint};

    /// Every pedantic lint is fired by exactly one of the three walks.
    #[test]
    fn every_pedantic_lint_has_one_detector() {
        let pedantic = LINTS.iter().filter(|l| l.group == Group::Pedantic);
        for lint in pedantic {
            let homes = [&CHAR_LINTS[..], &TEXT_LINTS, &LINE_LINTS]
                .iter()
                .filter(|walk| walk.contains(lint))
                .count();
            assert_eq!(homes, 1, "{}", lint.name);
        }
    }

    /// A space is `unicode-space` and nothing else; a fullwidth letter is
    /// both compatibility and a lookalike, compatibility first.
    #[test]
    fn char_lints_claim_in_order() {
        assert_eq!(char_lints('a'), [None; 3]);
        let space = [Some(UNICODE_SPACE), None, None];
        assert_eq!(char_lints('\u{a0}'), space);
        let both = [None, Some(NFKC_COMPAT), Some(CONFUSABLE)];
        assert_eq!(char_lints('\u{ff21}'), both);
        assert_eq!(char_lints('\u{b2}'), [None, Some(NFKC_COMPAT), None]);
    }

    /// Text hits come in byte order, `not-nfc` first on a tie.
    #[test]
    fn text_hits_are_in_byte_order() {
        let found: Vec<(&str, usize)> = text_hits("\u{3bb}e\u{301} x\u{212b}")
            .into_iter()
            .map(|(lint, hit)| (lint.name, hit.position.byte))
            .collect();
        let expected =
            vec![(NOT_NFC.name, 2), (MIXED_SCRIPT.name, 2), (NOT_NFC.name, 7)];
        assert_eq!(found, expected);
    }

    /// One finding as `(lint, line, column, character)`.
    type Fired = (&'static str, usize, usize, char);

    /// What fired, in that shape.
    fn fired(text: &str) -> Vec<Fired> {
        line_hits(text)
            .into_iter()
            .map(|(lint, hit)| {
                let at = hit.position;
                (lint.name, at.line, at.column, hit.character)
            })
            .collect()
    }

    #[test]
    fn the_lints_here_are_the_registered_ones() {
        for lint in [UNICODE_SPACE, CRLF, TRAILING_WHITESPACE, FINAL_NEWLINE] {
            assert_eq!(Lint::named(lint.name), Some(lint), "{}", lint.name);
            assert_eq!(lint.group, Group::Pedantic, "{}", lint.name);
        }
        assert!(!LINE_LINTS.contains(&UNICODE_SPACE));
    }

    #[test]
    fn a_clean_text_fires_nothing() {
        assert_eq!(fired("one\ntwo\n"), vec![]);
        assert_eq!(fired("\n\n"), vec![]);
    }

    #[test]
    fn an_empty_file_is_not_missing_a_newline() {
        assert_eq!(fired(""), vec![]);
    }

    #[test]
    fn crlf_points_at_each_carriage_return() {
        let expected = vec![("crlf", 1, 2, '\r'), ("crlf", 2, 3, '\r')];
        assert_eq!(fired("a\r\nbc\r\n"), expected);
    }

    #[test]
    fn a_lone_carriage_return_is_not_crlf() {
        assert_eq!(fired("a\rb\n"), vec![]);
    }

    #[test]
    fn trailing_whitespace_points_at_the_start_of_the_run_once() {
        let expected = vec![("trailing-whitespace", 1, 4, ' ')];
        assert_eq!(fired("abc \t \ndef\n"), expected);
    }

    #[test]
    fn a_whitespace_only_line_trails_from_its_first_column() {
        assert_eq!(fired("a\n  \n"), vec![("trailing-whitespace", 2, 1, ' ')]);
    }

    #[test]
    fn whitespace_inside_a_line_is_not_trailing() {
        assert_eq!(fired("a  b\n"), vec![]);
    }

    /// The CR of a CR LF is the terminator, not trailing whitespace.
    #[test]
    fn a_crlf_line_trails_only_when_whitespace_precedes_the_cr() {
        assert_eq!(fired("abc\r\n"), vec![("crlf", 1, 4, '\r')]);
        let both =
            vec![("trailing-whitespace", 1, 4, ' '), ("crlf", 1, 5, '\r')];
        assert_eq!(fired("abc \r\n"), both);
    }

    #[test]
    fn final_newline_points_at_the_last_character() {
        assert_eq!(fired("a\nbc"), vec![("final-newline", 2, 2, 'c')]);
    }

    #[test]
    fn an_unterminated_last_line_can_trail_too() {
        let expected = vec![
            ("trailing-whitespace", 1, 2, ' '),
            ("final-newline", 1, 3, ' '),
        ];
        assert_eq!(fired("a  "), expected);
    }

    /// A NO-BREAK SPACE trails like any other whitespace: White_Space
    /// holds it, and the line still ends in something a reader cannot see.
    #[test]
    fn a_trailing_no_break_space_is_trailing_whitespace() {
        let expected = vec![("trailing-whitespace", 1, 2, '\u{a0}')];
        assert_eq!(fired("a\u{a0}\n"), expected);
    }

    #[test]
    fn unicode_space_fires_on_the_listed_spaces_and_nothing_else() {
        for point in ['\u{00A0}', '\u{2000}', '\u{200A}', '\u{202F}'] {
            assert_eq!(unicode_space(point), Some(UNICODE_SPACE), "{point:?}");
        }
        for point in [' ', '\t', '\u{200B}', '\u{2028}', 'a'] {
            assert_eq!(unicode_space(point), None, "{point:?}");
        }
    }

    /// The list is `Zs` minus U+0020, checked against std's White_Space
    /// table (a superset of `Zs`): every non-ASCII whitespace character is
    /// either listed or one of the three that are whitespace but not
    /// `Zs` -- NEXT LINE (a control), and the line and paragraph
    /// separators.
    #[test]
    fn the_list_is_every_non_ascii_space_separator() {
        let not_zs = ['\u{0085}', '\u{2028}', '\u{2029}'];
        let whitespace: Vec<char> = (char::from(0x80_u8)..=char::MAX)
            .filter(|c| c.is_whitespace() && !not_zs.contains(c))
            .collect();
        assert_eq!(whitespace, SPACES.to_vec());
    }
}
