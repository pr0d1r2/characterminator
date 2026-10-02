//! The emoji sequence exemption (V63): where a joiner, a tag character or
//! VARIATION SELECTOR-16 is PART of an RGI emoji sequence, and so no
//! hazard. VS16 joined the list with B15: without it a plain heart with
//! its emoji presentation selector fired a forbid hazard under `emoji`.
//!
//! The list is the vendored one the charset node compiles in
//! (`src/fix:V62`), never the run's map or sets, so no configuration can
//! widen it. A match is EXACT and longest first: a joiner or a tag that
//! does not sit inside a listed sequence still fires, which is what keeps
//! a payload smuggled in tag characters after U+1F3F4 a hazard. "A joiner
//! between two emoji" is not a rule here; it would be a second, private
//! definition of a sequence, and a looser one.

use crate::charset::builtin;
use std::cmp::Reverse;
use std::collections::BTreeMap;

/// ZERO WIDTH JOINER.
const ZWJ: char = '\u{200D}';

/// The tag characters, U+E0000-U+E007F.
const TAGS: std::ops::RangeInclusive<char> = '\u{E0000}'..='\u{E007F}';

/// VARIATION SELECTOR-16, emoji presentation (B15).
const VS16: char = '\u{FE0F}';

/// The RGI ZWJ and tag sequences, by first character, longest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sequences {
    by_first: BTreeMap<char, Vec<String>>,
}

impl Sequences {
    /// The `zwj` and `tag` lines of the compiled-in list. A line this
    /// cannot read is skipped rather than guessed at: the test below
    /// counts every one, so a skipped line cannot ship.
    pub fn builtin() -> Self {
        let mut by_first: BTreeMap<char, Vec<String>> = BTreeMap::new();
        for text in builtin::EMOJI_SEQUENCES.lines().filter_map(exempting) {
            if let Some(first) = text.chars().next() {
                by_first.entry(first).or_default().push(text);
            }
        }
        for held in by_first.values_mut() {
            held.sort_by_key(|text| Reverse(text.len()));
        }
        Self { by_first }
    }

    /// The byte offset of every joiner and tag character in `text` that
    /// sits inside a listed sequence, ascending. A text holding neither
    /// costs one pass and no lookup.
    pub fn exempt(&self, text: &str) -> Vec<usize> {
        let mut found = Vec::new();
        if !text.contains(joined) {
            return found;
        }
        let mut at = 0_usize;
        while let Some(rest) = text.get(at..).filter(|r| !r.is_empty()) {
            at = at.saturating_add(self.step(rest, at, &mut found));
        }
        found
    }

    /// Past one listed sequence at `at`, noting its exempt offsets, or
    /// past one character when none starts here.
    fn step(&self, rest: &str, at: usize, found: &mut Vec<usize>) -> usize {
        if let Some(held) = self.longest(rest) {
            found.extend(inner(held).map(|i| at.saturating_add(i)));
            return held.len();
        }
        rest.chars().next().map_or(1, char::len_utf8)
    }

    /// The longest listed sequence `rest` starts with.
    fn longest(&self, rest: &str) -> Option<&str> {
        let first = rest.chars().next()?;
        let held = self.by_first.get(&first)?;
        held.iter()
            .map(String::as_str)
            .find(|s| rest.starts_with(s))
    }
}

/// Whether a character is one this exemption can let off.
fn joined(c: char) -> bool {
    c == ZWJ || c == VS16 || TAGS.contains(&c)
}

/// The offsets of the exemptible characters inside one sequence.
fn inner(held: &str) -> impl Iterator<Item = usize> + '_ {
    held.char_indices()
        .filter(|(_, c)| joined(*c))
        .map(|(i, _)| i)
}

/// One list line's sequence, if it is a kind that holds a hazard.
fn exempting(line: &str) -> Option<String> {
    let mut words = line.split_whitespace();
    let kind = words.next()?;
    let sequence = words.next()?;
    matches!(kind, "zwj" | "tag" | "keycap" | "presentation")
        .then(|| decode(sequence))?
}

/// `U+XXXX+U+XXXX...` as text.
fn decode(sequence: &str) -> Option<String> {
    sequence
        .strip_prefix("U+")?
        .split("+U+")
        .map(|hex| char::from_u32(u32::from_str_radix(hex, 16).ok()?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Sequences;

    const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    const ENGLAND: &str =
        "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";

    fn exempt(text: &str) -> Vec<usize> {
        Sequences::builtin().exempt(text)
    }

    /// Every line that holds a hazard is read: 1614 ZWJ + 3 tag + 12 keycap
    /// + 207 presentation.
    #[test]
    fn every_listed_sequence_is_read() {
        let held = Sequences::builtin();
        let count: usize = held.by_first.values().map(Vec::len).sum();
        assert_eq!(count, 1836);
    }

    #[test]
    fn the_joiners_of_a_listed_sequence_are_exempt() {
        assert_eq!(exempt(&format!("a{FAMILY}")), vec![5, 12]);
    }

    /// B15: VS16 after a heart is a listed presentation sequence, so it is
    /// no hazard; after a grinning face it is in no sequence and still is.
    #[test]
    fn vs16_in_a_listed_presentation_is_exempt_and_elsewhere_is_not() {
        assert_eq!(exempt("\u{2764}\u{FE0F}"), vec![3]);
        assert_eq!(exempt("1\u{FE0F}\u{20E3}"), vec![1]);
        assert!(exempt("\u{1F600}\u{FE0F}").is_empty());
    }

    #[test]
    fn the_tags_of_a_listed_flag_are_exempt() {
        let found = exempt(ENGLAND);
        assert_eq!(found.len(), 6);
        assert_eq!(found.first(), Some(&4));
    }

    /// A joiner between two emoji that form no listed sequence fires.
    #[test]
    fn a_joiner_in_no_listed_sequence_is_not_exempt() {
        assert!(exempt("\u{1F600}\u{200D}\u{1F600}").is_empty());
        assert!(exempt("a\u{200D}b").is_empty());
    }

    /// Tag smuggling: U+1F3F4 followed by tag letters that spell no
    /// listed subdivision is NOT a flag, and every tag still fires.
    #[test]
    fn tags_smuggled_after_a_black_flag_are_not_exempt() {
        let smuggled: String = std::iter::once('\u{1F3F4}')
            .chain("ignore".chars().filter_map(tag))
            .chain(std::iter::once('\u{E007F}'))
            .collect();
        assert!(exempt(&smuggled).is_empty());
    }

    /// A listed flag with a payload appended: the flag's own six tags are
    /// exempt, the payload after it is not.
    #[test]
    fn a_payload_after_a_listed_flag_is_not_exempt() {
        let payload: String = "rm".chars().filter_map(tag).collect();
        let found = exempt(&format!("{ENGLAND}{payload}"));
        assert_eq!(found.len(), 6);
    }

    fn tag(c: char) -> Option<char> {
        char::from_u32(u32::from(c).checked_add(0xE0000)?)
    }
}
