//! Membership and union: what a named set of code points answers.
//!
//! See `src/charset/SPEC.md`.

use super::{CharRange, CharSet, range::normalize};

impl CharSet {
    /// A named set over `ranges`, held in canonical form.
    #[must_use]
    pub fn new(name: String, ranges: Vec<CharRange>) -> Self {
        Self {
            name,
            ranges: normalize(ranges),
        }
    }

    /// Whether the set grants `point`.
    ///
    /// A linear walk rather than a binary search over the sorted ranges:
    /// `ranges` is a public field, so a caller can hand-build a set in any
    /// order, and a search that silently assumes an ordering the type does
    /// not enforce would answer wrongly for the first such set. A preset is
    /// a handful of ranges by V23, so there is no cost worth the risk.
    #[must_use]
    pub fn contains(&self, point: char) -> bool {
        self.ranges.iter().any(|range| range.contains(point))
    }

    /// Whether the set grants nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// The union of `parts`, named `name`.
    ///
    /// Union is the ONLY way sets compose (V3). There is deliberately no
    /// difference, subtraction or intersection here: a grant that could be
    /// taken away again means the effective set of a path can no longer be
    /// read off the winning rule's names.
    #[must_use]
    pub fn union(name: String, parts: &[Self]) -> Self {
        let ranges = parts
            .iter()
            .flat_map(|part| part.ranges.iter().copied())
            .collect();
        Self::new(name, ranges)
    }
}

#[cfg(test)]
mod tests {
    use super::{CharRange, CharSet};

    fn letters() -> CharSet {
        let range = CharRange {
            start: '\u{0041}',
            end: '\u{005A}',
        };
        CharSet::new("letters".to_owned(), vec![range])
    }

    fn dash() -> CharSet {
        let em = CharRange::single('\u{2014}');
        CharSet::new("dash".to_owned(), vec![em])
    }

    #[test]
    fn contains_only_what_was_granted() {
        let set = letters();
        assert!(set.contains('\u{0041}'));
        assert!(!set.contains('\u{2014}'));
    }

    #[test]
    fn new_canonicalizes_the_ranges() {
        let high = CharRange::single('\u{0043}');
        let low = CharRange::single('\u{0041}');
        let set = CharSet::new("two".to_owned(), vec![high, low]);
        assert_eq!(set.ranges, vec![low, high]);
    }

    #[test]
    fn union_grants_both_sides() {
        let set = CharSet::union("both".to_owned(), &[letters(), dash()]);
        assert!(set.contains('\u{0041}'));
        assert!(set.contains('\u{2014}'));
        assert_eq!(set.name, "both");
    }

    #[test]
    fn union_is_idempotent() {
        let once = CharSet::union("x".to_owned(), &[letters()]);
        let twice = CharSet::union("x".to_owned(), &[letters(), letters()]);
        assert_eq!(once, twice);
    }

    #[test]
    fn union_of_nothing_is_empty() {
        let set = CharSet::union("none".to_owned(), &[]);
        assert!(set.is_empty());
        assert!(!set.contains('\u{0041}'));
    }
}
