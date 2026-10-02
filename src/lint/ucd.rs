//! The ONE call site of the Unicode crates (`src:C`, `src/lint:V58`):
//! `unicode-normalization` and `unicode-security`. Everything here is a
//! question put to a published table; what a finding is and which lint
//! it carries is `pedantic.rs`'s business.
//!
//! WHICH CHARACTER EACH ANSWER POINTS AT, so a finding is one character
//! really in the file and `Hit` stays as it is:
//!
//! - [`denormal`]: per NFC segment that NFC changes, the FIRST character
//!   at which the segment and its NFC diverge. A segment starts at a
//!   starter whose NFC quick check is Yes -- UAX #15's safe boundary, so
//!   no composition reaches across it.
//! - [`mixed`]: per word holding two scripts, the FIRST character whose
//!   script leaves the word's resolved script set (UTS #39) empty.
//! - [`compat`] and [`lookalike`] judge one character on its own.

use core::iter::once;

use unicode_normalization::char::{
    canonical_combining_class, is_combining_mark,
};
use unicode_normalization::{IsNormalized, UnicodeNormalization, is_nfc_quick};
use unicode_security::mixed_script::AugmentedScriptSet;
use unicode_security::skeleton;

use crate::scan::Hit;

/// Whether NFKC changes this character where NFC does not: a fullwidth
/// form, a ligature, a superscript, a circled digit.
pub fn compat(character: char) -> bool {
    !once(character).nfkc().eq(once(character).nfc())
}

/// Whether a non-ASCII character's UTS #39 skeleton is non-empty ASCII:
/// it is drawn like ASCII text a reader would take it for. An ASCII
/// character is never one, whatever its skeleton (`m` maps to `rn`).
pub fn lookalike(character: char) -> bool {
    let mut buffer = [0_u8; 4];
    let text = character.encode_utf8(&mut buffer);
    let mut drawn = skeleton(text).peekable();
    !character.is_ascii()
        && drawn.peek().is_some()
        && drawn.all(|c| c.is_ascii())
}

/// The character each NFC-changed segment of a text points at, in order.
/// Nothing at all for a text the quick check passes, which is nearly
/// every text, so the per-segment work is only paid where it can find.
pub fn denormal(hits: &[Hit]) -> Vec<Hit> {
    let characters = hits.iter().map(|hit| hit.character);
    if is_nfc_quick(characters) == IsNormalized::Yes {
        return Vec::new();
    }
    hits.chunk_by(|_, next| !boundary(next.character))
        .filter_map(divergence)
        .collect()
}

/// UAX #15: NFC never composes across a starter whose quick check is Yes.
fn boundary(character: char) -> bool {
    canonical_combining_class(character) == 0
        && is_nfc_quick(once(character)) == IsNormalized::Yes
}

/// Where one segment and its NFC first differ, or `None` if they do not.
/// NFC that is a strict prefix-extension or truncation diverges at the
/// shorter length, clamped to the segment's last character.
fn divergence(segment: &[Hit]) -> Option<Hit> {
    let original = || segment.iter().map(|hit| hit.character);
    let normal: Vec<char> = original().nfc().collect();
    let differs = normal
        .iter()
        .copied()
        .zip(original())
        .position(|(a, b)| a != b);
    let longer = (normal.len() != segment.len()).then_some(normal.len());
    let index = differs.or(longer)?;
    segment.get(index).or(segment.last()).copied()
}

/// The character each mixed-script word points at, in order. A word is
/// a run of alphanumerics and combining marks, so `foo_bar` is two words
/// and a Cyrillic letter in `p\u{0430}ypal` is in the same word as the
/// Latin around it.
pub fn mixed(hits: &[Hit]) -> Vec<Hit> {
    hits.chunk_by(|a, b| in_word(a.character) && in_word(b.character))
        .filter_map(first_foreign)
        .collect()
}

fn in_word(character: char) -> bool {
    character.is_alphanumeric() || is_combining_mark(character)
}

/// The first character after which the word's resolved script set is
/// empty. Common and Inherited characters (digits, marks) leave it as is.
fn first_foreign(word: &[Hit]) -> Option<Hit> {
    let mut scripts = AugmentedScriptSet::default();
    word.iter().copied().find(|hit| {
        scripts.intersect_with(AugmentedScriptSet::for_char(hit.character));
        scripts.is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::{compat, denormal, lookalike, mixed};
    use crate::scan::{Hit, located};

    /// One of the text questions: hits in, the hits it points at out.
    type Finder = fn(&[Hit]) -> Vec<Hit>;

    /// What `find` points at in `text`, as `(column, character)`.
    fn at(text: &str, find: Finder) -> Vec<(usize, char)> {
        let hits: Vec<_> = located(text).collect();
        find(&hits)
            .into_iter()
            .map(|hit| (hit.position.column, hit.character))
            .collect()
    }

    #[test]
    fn compat_is_what_nfkc_folds_and_nfc_keeps() {
        for c in ['\u{FF21}', '\u{FB01}', '\u{00B2}', '\u{2460}'] {
            assert!(compat(c), "{c:?}");
        }
        for c in ['a', '\u{00E9}', '\u{0416}', '\u{212B}'] {
            assert!(!compat(c), "{c:?}");
        }
    }

    #[test]
    fn a_lookalike_is_non_ascii_drawn_as_ascii() {
        for c in ['\u{0430}', '\u{03BF}', '\u{0421}'] {
            assert!(lookalike(c), "{c:?}");
        }
        for c in ['a', 'm', '\u{0416}', '\u{00E9}', '\u{4E00}'] {
            assert!(!lookalike(c), "{c:?}");
        }
    }

    #[test]
    fn nfc_text_is_not_denormal() {
        assert_eq!(at("caf\u{00E9} \u{AC00}\n", denormal), vec![]);
    }

    /// A decomposed letter points at its base; a singleton (ANGSTROM
    /// SIGN, which NFC replaces) points at itself, not at what precedes.
    #[test]
    fn denormal_points_where_nfc_first_diverges() {
        let decomposed = at("cafe\u{0301} x\u{212B}\n", denormal);
        assert_eq!(decomposed, vec![(4, 'e'), (8, '\u{212B}')]);
    }

    /// Hangul jamo compose across two starters: one finding, the L.
    #[test]
    fn conjoining_jamo_are_one_segment() {
        assert_eq!(at("\u{1100}\u{1161}\n", denormal), vec![(1, '\u{1100}')]);
    }

    /// Marks out of canonical order: NFC reorders, so it diverges at the
    /// first misplaced mark.
    #[test]
    fn misordered_marks_diverge_at_the_first_mark() {
        let text = "x\u{0301}\u{0323}\n";
        assert_eq!(at(text, denormal), vec![(2, '\u{0301}')]);
    }

    #[test]
    fn a_word_in_one_script_is_not_mixed() {
        let text = "hello \u{043C}\u{0438}\u{0440} \u{6F22}\u{3042} x2\n";
        assert_eq!(at(text, mixed), vec![]);
    }

    #[test]
    fn mixed_points_at_the_first_foreign_letter_once_per_word() {
        let text = "p\u{0430}yp\u{0430}l ok \u{0430}b\n";
        assert_eq!(at(text, mixed), vec![(2, '\u{0430}'), (12, 'b')]);
    }
}
