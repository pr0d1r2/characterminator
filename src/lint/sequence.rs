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
use std::sync::LazyLock;

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
    /// The longest listed sequence, in bytes: how far before a joinable
    /// character a sequence holding it can start.
    widest: usize,
    /// Whether this stands for the compiled-in list NOT YET READ: the
    /// list is parsed on the first text that holds a joinable character,
    /// once per process, and never for a run whose text holds none (R17).
    deferred: bool,
}

/// The compiled-in list, read on first use.
static BUILTIN: LazyLock<Sequences> = LazyLock::new(Sequences::builtin);

impl Sequences {
    /// The compiled-in list, as [`Sequences::builtin`] reads it, but read
    /// only when a text first needs it. Parsing it costs about half a
    /// millisecond, which every run paid before a byte was looked at.
    pub fn deferred() -> Self {
        Self {
            deferred: true,
            ..Self::default()
        }
    }

    /// The `zwj` and `tag` lines of the compiled-in list. A line this
    /// cannot read is skipped rather than guessed at: the test below
    /// counts every one, so a skipped line cannot ship.
    pub fn builtin() -> Self {
        let by_first = listed();
        let widest = by_first.values().flatten().map(String::len).max();
        Self {
            by_first,
            widest: widest.unwrap_or(0),
            deferred: false,
        }
    }

    /// The byte offset of every joiner and tag character in `text` that
    /// sits inside a listed sequence, ascending. A text holding neither
    /// costs one pass and no lookup.
    ///
    /// Matching is tried only in a WINDOW before each joinable character
    /// (R17). Every listed sequence holds one (a test holds the list to
    /// that), so a sequence starting more than [`Sequences::widest`]
    /// bytes before the next one cannot reach it and cannot match at
    /// all. Every position skipped is one the walk would have stepped
    /// past one character at a time, so the matches -- longest first,
    /// left to right, a match consuming what it covers -- are exactly
    /// those of trying every position.
    pub fn exempt(&self, text: &str) -> Vec<usize> {
        if self.deferred {
            let wanted = next_joined(text, 0).is_some();
            return if wanted {
                BUILTIN.exempt(text)
            } else {
                Vec::new()
            };
        }
        self.walk(text)
    }

    /// [`Sequences::exempt`], over this list.
    fn walk(&self, text: &str) -> Vec<usize> {
        let mut found = Vec::new();
        let mut at = 0_usize;
        while let Some(next) = next_joined(text, at) {
            at = at.max(self.window(text, next));
            while at <= next {
                let rest = text.get(at..).unwrap_or_default();
                at = at.saturating_add(self.step(rest, at, &mut found));
            }
        }
        found
    }

    /// The first char boundary at which a listed sequence could start
    /// and still reach the joinable character at `next`.
    fn window(&self, text: &str, next: usize) -> usize {
        let reach = self.widest.saturating_sub(1);
        text.ceil_char_boundary(next.saturating_sub(reach))
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

/// The list's sequences by first character, longest first.
fn listed() -> BTreeMap<char, Vec<String>> {
    let mut by_first: BTreeMap<char, Vec<String>> = BTreeMap::new();
    for text in builtin::EMOJI_SEQUENCES.lines().filter_map(exempting) {
        if let Some(first) = text.chars().next() {
            by_first.entry(first).or_default().push(text);
        }
    }
    for held in by_first.values_mut() {
        held.sort_by_key(|text| Reverse(text.len()));
    }
    by_first
}

/// Whether a character is one this exemption can let off.
fn joined(c: char) -> bool {
    c == ZWJ || c == VS16 || TAGS.contains(&c)
}

/// The byte offset of the first joinable character at or after `at`.
fn next_joined(text: &str, at: usize) -> Option<usize> {
    let rest = text.get(at..)?;
    let (offset, _) = rest.char_indices().find(|(_, c)| joined(*c))?;
    at.checked_add(offset)
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

/// R17: the windowed walk against the walk it replaced, which tried every
/// position once any joinable character was present. Kept here, as the
/// reference, and nowhere else.
#[cfg(test)]
mod windowed {
    use super::{Sequences, joined};

    fn everywhere(held: &Sequences, text: &str) -> Vec<usize> {
        let mut found = Vec::new();
        if !text.contains(joined) {
            return found;
        }
        let mut at = 0_usize;
        while let Some(rest) = text.get(at..).filter(|r| !r.is_empty()) {
            at = at.saturating_add(held.step(rest, at, &mut found));
        }
        found
    }

    /// Pieces a text is built from: letters of one to four bytes, every
    /// joinable kind, the first characters of listed sequences, a skin
    /// tone, a keycap, and a run of padding longer than any sequence.
    const PIECES: [&str; 20] = [
        "a",
        "\u{e9}",
        "\u{4E2D}",
        "\u{1F468}",
        "\u{1F469}",
        "\u{1F467}",
        "\u{200D}",
        "\u{FE0F}",
        "\u{2764}",
        "\u{1F3F4}",
        "\u{E0067}",
        "\u{E0062}",
        "\u{E0065}",
        "\u{E006E}",
        "\u{E007F}",
        "1",
        "\u{20E3}",
        "\u{1F3FB}",
        "\u{1F600}",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ];

    /// A fixed pseudo-random walk (64-bit LCG), so a failure reproduces.
    struct Draw(u64);

    impl Draw {
        /// A number below `n`, or 0 when `n` is 0.
        fn below(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let high = usize::try_from(self.0 >> 33).unwrap_or(0);
            high.checked_rem(n).unwrap_or(0)
        }
    }

    /// One text of up to 23 pieces, a quarter of them listed sequences.
    fn text(draw: &mut Draw, listed: &[&str]) -> String {
        let mut text = String::new();
        for _ in 0..draw.below(24) {
            let piece = match draw.below(4) {
                0 => listed.get(draw.below(listed.len())),
                _ => PIECES.get(draw.below(PIECES.len())),
            };
            text.push_str(piece.copied().unwrap_or_default());
        }
        text
    }

    /// The windowed walk, the deferred list and the old walk agree.
    fn same(held: &Sequences, text: &str) {
        let found = held.exempt(text);
        assert_eq!(found, everywhere(held, text), "{text:?}");
        assert_eq!(found, Sequences::deferred().exempt(text), "{text:?}");
    }

    #[test]
    fn the_window_finds_what_trying_everywhere_finds() {
        let held = Sequences::builtin();
        let listed: Vec<&str> = held
            .by_first
            .values()
            .flatten()
            .map(String::as_str)
            .collect();
        let mut draw = Draw(0x2545_F491_4F6C_DD1D);
        for _ in 0..4000 {
            same(&held, &text(&mut draw, &listed));
        }
    }

    /// Every listed sequence, behind padding wider than the window, with
    /// one character cut off its end, and doubled.
    #[test]
    fn every_listed_sequence_is_found_behind_padding_and_cut_short() {
        let held = Sequences::builtin();
        let pad = "y".repeat(held.widest.saturating_add(3));
        for listed in held.by_first.values().flatten() {
            let mut cut = listed.clone();
            cut.pop();
            same(&held, &format!("{pad}{listed}"));
            same(&held, &format!("{pad}{cut}"));
            same(&held, &listed.repeat(2));
        }
    }

    /// What the window rests on: no listed sequence lacks a joinable
    /// character, so none can match out of reach of one.
    #[test]
    fn every_listed_sequence_holds_a_joinable_character() {
        let held = Sequences::builtin();
        let bare = held
            .by_first
            .values()
            .flatten()
            .find(|s| !s.contains(joined));
        assert_eq!(bare, None);
    }
}
