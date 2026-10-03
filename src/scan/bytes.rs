//! Bytes in, text out -- or a NAMED refusal.
//!
//! Neither refusal is silent (`V8`): a file that reported nothing because
//! it could not be read is indistinguishable from a clean one, which is
//! the failure mode that makes a checker worth ignoring.

use super::text::scan_str;
use super::{Hit, Unreadable};

/// OPEN QUESTION, still the human's: `src/scan/SPEC.md` V8 marks the
/// binary heuristic `?` -- "binary file (NUL byte ?)". The marker is on
/// the DETAIL, so a single NUL byte anywhere is the whole detector here
/// and nothing more elaborate was invented.
///
/// It is deliberately ONE function so it can be replaced wholesale. What
/// a richer rule would weigh, when the human decides: a NUL only in the
/// first few kilobytes rather than the whole file (cost), a ratio of
/// control bytes, or a byte order mark naming UTF-16, which this rule
/// calls binary today because UTF-16 ASCII text is half NUL bytes.
pub(crate) fn looks_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

/// What [`scan_bytes`] hands back: the decoded text, and the hits in it.
pub type Scanned<'a> = (&'a str, Vec<Hit>);

/// Scan raw bytes, refusing by name when they are not text.
///
/// ORDER MATTERS: the binary test runs FIRST. A binary file is a skip and
/// is named as a skip; reporting it as a UTF-8 error would send a reader
/// to fix an encoding in a file that was never text. Only what survives
/// that test is decoded, and a decode failure carries the byte offset
/// where it failed so the report can name a place rather than a file.
///
/// Lossy decoding is not an option here (`V8`): it would invent
/// replacement characters and then this node would report positions for
/// characters that are not in the file.
///
/// The decoded text comes back WITH the hits, so a caller that asks the
/// text another question -- a line, an exemption -- decodes it once.
///
/// ```
/// use characterminator::{Unreadable, scan_bytes};
///
/// let scanned = scan_bytes("a\u{2014}b".as_bytes(), |c| c.is_ascii());
/// assert_eq!(scanned.map(|(text, hits)| (text.len(), hits.len())), Ok((5, 1)));
/// let binary = scan_bytes(b"a\0b", |c| c.is_ascii());
/// assert_eq!(binary, Err(Unreadable::Binary));
/// ```
///
/// # Errors
///
/// [`Unreadable::Binary`] for a NUL byte anywhere, else
/// [`Unreadable::NotUtf8`] at the first byte that does not decode.
pub fn scan_bytes(
    bytes: &[u8],
    allowed: impl Fn(char) -> bool,
) -> Result<Scanned<'_>, Unreadable> {
    decode(bytes).map(|text| (text, scan_str(text, allowed)))
}

/// The text in raw bytes, or the NAMED reason there is none: the refusal
/// half of [`scan_bytes`], for a caller that asks more than one question
/// of the same text (`src/lint/pedantic:V55` walks it a second time for line
/// endings) and should neither decode it twice nor refuse it two ways.
///
/// # Errors
///
/// [`Unreadable::Binary`] first, then [`Unreadable::NotUtf8`], in the
/// order and for the reasons [`scan_bytes`] gives.
pub(crate) fn decode(bytes: &[u8]) -> Result<&str, Unreadable> {
    if looks_binary(bytes) {
        return Err(Unreadable::Binary);
    }
    core::str::from_utf8(bytes).map_err(|error| Unreadable::NotUtf8 {
        byte: error.valid_up_to(),
    })
}

/// [`decode`], keeping the bytes: a file read once becomes its text with
/// no copy, refused for the same reasons in the same order.
///
/// # Errors
///
/// As [`decode`].
pub(crate) fn decode_owned(bytes: Vec<u8>) -> Result<String, Unreadable> {
    if looks_binary(&bytes) {
        return Err(Unreadable::Binary);
    }
    String::from_utf8(bytes).map_err(|error| Unreadable::NotUtf8 {
        byte: error.utf8_error().valid_up_to(),
    })
}

#[cfg(test)]
mod tests {
    use super::{decode, decode_owned, looks_binary, scan_bytes};
    use crate::scan::{Hit, Position, Unreadable};

    fn ascii_only(character: char) -> bool {
        character.is_ascii()
    }

    #[test]
    fn invalid_utf8_reports_the_byte_offset_where_decoding_failed() {
        // 0xFF is not a legal UTF-8 lead byte; two bytes decoded first.
        let outcome = scan_bytes(b"ab\xffcd", ascii_only);

        assert_eq!(outcome, Err(Unreadable::NotUtf8 { byte: 2 }));
    }

    #[test]
    fn a_nul_byte_makes_the_input_binary() {
        let outcome = scan_bytes(b"ab\x00cd", ascii_only);

        assert_eq!(outcome, Err(Unreadable::Binary));
    }

    #[test]
    fn binary_is_decided_before_utf8_so_a_skip_is_not_an_error() {
        let outcome = scan_bytes(b"\x00\xff", ascii_only);

        assert_eq!(outcome, Err(Unreadable::Binary));
    }

    #[test]
    fn readable_bytes_are_scanned_as_text() {
        // Zero width space, three bytes, at the end of a two-char line.
        let text = "ab\u{200b}";
        let outcome = scan_bytes(text.as_bytes(), ascii_only);
        let (line, column, byte) = (1, 3, 2);
        let position = Position { line, column, byte };
        let character = '\u{200b}';
        let hit = Hit {
            position,
            character,
        };
        assert_eq!(outcome, Ok((text, vec![hit])));
    }

    /// The owned decode refuses exactly as the borrowed one does.
    #[test]
    fn the_owned_decode_agrees_with_the_borrowed_one() {
        for bytes in [&b"ab\xffcd"[..], b"ab\x00cd", b"\x00\xff", b"ok\n"] {
            let owned = decode_owned(bytes.to_vec());
            let borrowed = decode(bytes).map(str::to_owned);
            assert_eq!(owned, borrowed);
        }
    }

    #[test]
    fn decoding_refuses_the_way_scanning_does() {
        assert_eq!(decode(b"ab\x00\xff"), Err(Unreadable::Binary));
        assert_eq!(decode(b"ab\xff"), Err(Unreadable::NotUtf8 { byte: 2 }));
        assert_eq!(decode("a\u{e9}".as_bytes()), Ok("a\u{e9}"));
    }

    #[test]
    fn the_heuristic_stands_alone_and_looks_only_for_a_nul() {
        assert!(looks_binary(b"\x00"));
        assert!(!looks_binary(b"\xff\xfe plain high bytes"));
    }
}
