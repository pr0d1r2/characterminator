//! The unit a set is built from, and the canonical form sets are kept in.
//!
//! See `src/charset/SPEC.md`.

use super::CharRange;

impl CharRange {
    /// A range from `start` to `end` inclusive, or `None` when `end` sorts
    /// below `start`.
    ///
    /// An Option rather than swapping the two ends: a reversed range in a
    /// config file is a typo, and reading it backwards would silently grant
    /// a span of characters nobody named.
    #[must_use]
    pub const fn new(start: char, end: char) -> Option<Self> {
        if (start as u32) > (end as u32) {
            return None;
        }
        Some(Self { start, end })
    }

    /// The range holding exactly one code point.
    #[must_use]
    pub const fn single(point: char) -> Self {
        Self {
            start: point,
            end: point,
        }
    }

    /// Whether this range grants `point`.
    #[must_use]
    pub const fn contains(&self, point: char) -> bool {
        (self.start as u32) <= (point as u32)
            && (point as u32) <= (self.end as u32)
    }

    /// Whether `later` overlaps this range or starts at the very next code
    /// point, i.e. whether the union of the two is itself one range.
    ///
    /// Adjacency counts so that tab and newline, which `ascii` grants as
    /// U+0009 and U+000A, collapse into a single row instead of reading as
    /// two unrelated grants.
    ///
    /// Assumes `later` starts at or after `self`, which is what `normalize`
    /// guarantees by sorting first.
    const fn touches(&self, later: &Self) -> bool {
        (later.start as u32) <= (self.end as u32).saturating_add(1)
    }
}

/// Sort and merge ranges into canonical form: ascending, none overlapping,
/// none adjacent.
///
/// Every set passes through this, so two sets built from differently
/// spelled but equal grants compare equal, and so `explain` prints one row
/// per span rather than one per line of config that contributed to it.
#[must_use]
pub fn normalize(mut ranges: Vec<CharRange>) -> Vec<CharRange> {
    ranges.sort_unstable_by_key(|range| (range.start, range.end));
    let mut merged: Vec<CharRange> = Vec::with_capacity(ranges.len());
    for next in ranges {
        match merged.last_mut() {
            Some(last) if last.touches(&next) => {
                last.end = last.end.max(next.end);
            }
            _ => merged.push(next),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::{CharRange, normalize};

    fn span(start: char, end: char) -> CharRange {
        CharRange { start, end }
    }

    #[test]
    fn reversed_range_is_rejected() {
        assert_eq!(CharRange::new('\u{005A}', '\u{0041}'), None);
    }

    #[test]
    fn single_point_range_is_accepted() {
        let point = CharRange::new('\u{0041}', '\u{0041}');
        assert_eq!(point, Some(CharRange::single('\u{0041}')));
    }

    #[test]
    fn contains_covers_both_ends() {
        let range = span('\u{0041}', '\u{005A}');
        assert!(range.contains('\u{0041}'));
        assert!(range.contains('\u{005A}'));
        assert!(!range.contains('\u{0040}'));
        assert!(!range.contains('\u{005B}'));
    }

    #[test]
    fn normalize_sorts_and_merges_adjacent() {
        let tab = CharRange::single('\u{0009}');
        let newline = CharRange::single('\u{000A}');
        let merged = normalize(vec![newline, tab]);
        assert_eq!(merged, vec![span('\u{0009}', '\u{000A}')]);
    }

    #[test]
    fn normalize_merges_overlapping() {
        let low = span('\u{0041}', '\u{0050}');
        let high = span('\u{0045}', '\u{005A}');
        let merged = normalize(vec![low, high]);
        assert_eq!(merged, vec![span('\u{0041}', '\u{005A}')]);
    }

    #[test]
    fn normalize_merges_a_contained_range() {
        let outer = span('\u{0041}', '\u{005A}');
        let inner = span('\u{0045}', '\u{0046}');
        assert_eq!(normalize(vec![outer, inner]), vec![outer]);
    }

    #[test]
    fn normalize_keeps_a_gap() {
        let low = CharRange::single('\u{0041}');
        let high = CharRange::single('\u{0043}');
        assert_eq!(normalize(vec![high, low]), vec![low, high]);
    }

    #[test]
    fn normalize_of_nothing_is_nothing() {
        assert_eq!(normalize(Vec::new()), Vec::new());
    }
}
