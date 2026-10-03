//! Where a capped pre-read prefix may end (V102, B64): never inside an
//! emoji sequence, so the cut cannot MAKE a hazard -- a joiner whose
//! partner was cut off, or tag characters whose flag lost its terminator.

/// ZERO WIDTH JOINER.
const ZWJ: char = '\u{200D}';

/// The furthest a prefix backs off, in bytes. The longest RGI sequence
/// is well under this; the bound keeps a run of 16 MiB of joiners or tags
/// from backing the prefix off to nothing and so hiding what precedes it.
const REACH: usize = 64;

/// The end of `bytes` less a trailing UNFINISHED emoji sequence: trailing
/// ZWJ, VS15/VS16, U+20E3, skin tone modifiers and tags, with the base
/// before them, and a base that follows a ZWJ.
pub(super) fn settled(bytes: &[u8]) -> usize {
    let floor = bytes.len().saturating_sub(REACH);
    let (mut end, mut base_owed) = (bytes.len(), false);
    while end > floor {
        let Some((start, c)) = last_char(bytes.get(..end).unwrap_or_default())
        else {
            break;
        };
        if !(continues(c) || base_owed || joined(bytes, start)) {
            break;
        }
        base_owed = continues(c);
        end = start;
    }
    end
}

/// Whether the character before byte `start` is a ZWJ: what follows it
/// is a member of a sequence the cut may have left unfinished.
fn joined(bytes: &[u8], start: usize) -> bool {
    last_char(bytes.get(..start).unwrap_or_default())
        .is_some_and(|(_, before)| before == ZWJ)
}

/// A character that only continues an emoji sequence, never starts one.
const fn continues(c: char) -> bool {
    matches!(
        c,
        '\u{200D}'
            | '\u{FE0E}'
            | '\u{FE0F}'
            | '\u{20E3}'
            | '\u{1F3FB}'..='\u{1F3FF}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

/// The last whole character of `bytes` and where it starts, or `None`
/// when they do not end on one.
fn last_char(bytes: &[u8]) -> Option<(usize, char)> {
    (1..=4_usize)
        .filter_map(|n| bytes.len().checked_sub(n))
        .find_map(|start| {
            let tail = std::str::from_utf8(bytes.get(start..)?).ok()?;
            tail.chars().next().map(|c| (start, c))
        })
}

#[cfg(test)]
mod tests {
    use super::settled;

    fn kept(text: &str) -> usize {
        settled(text.as_bytes())
    }

    /// A family cut after a joiner, or inside its last member's place,
    /// backs off to before the family.
    #[test]
    fn an_unfinished_zwj_sequence_is_backed_off() {
        assert_eq!(kept("a\u{1F468}\u{200D}\u{1F469}\u{200D}"), 1);
        assert_eq!(kept("a\u{1F468}\u{200D}\u{1F469}"), 1);
    }

    /// A subdivision flag cut inside its tags, a keycap, a skin tone.
    #[test]
    fn unfinished_tags_keycaps_and_tones_are_backed_off() {
        assert_eq!(kept("a\u{1F3F4}\u{E0067}\u{E0062}"), 1);
        assert_eq!(kept("a1\u{FE0F}"), 1);
        assert_eq!(kept("a\u{1F44B}\u{1F3FB}"), 1);
    }

    #[test]
    fn plain_text_is_kept_whole() {
        assert_eq!(kept("abc"), 3);
        assert_eq!(kept("a\u{2014}"), 4);
        assert_eq!(kept(""), 0);
    }

    /// A long run of joiners backs off a bounded distance, so what comes
    /// before it -- here an override -- is still judged.
    #[test]
    fn the_back_off_is_bounded() {
        let text = format!("\u{202E}{}", "\u{200D}".repeat(100));
        assert!(kept(&text) >= 3, "{}", kept(&text));
    }
}
