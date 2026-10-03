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
    /// A binary search over the canonical ranges, because a locale preset
    /// is not a handful of ranges: `zh` holds 1,716, and a linear walk made
    /// a 10 MB Chinese file cost fifteen times what `* any` costs (V86).
    ///
    /// `ranges` stays a public field: `render` and `lint` read it, and
    /// tests in other nodes build sets literally, so making it private
    /// would ripple into all of them. The ordering is instead a guarantee
    /// of [`Self::new`] and [`Self::union`], which every set built from
    /// config passes through; a hand-built set must already be canonical.
    ///
    /// ASCII first, by a walk over the leading ranges only: every grant
    /// starts with `ascii`, so those are a few at most, and most text a
    /// check reads is ASCII (R1).
    #[must_use]
    pub fn contains(&self, point: char) -> bool {
        if point.is_ascii() {
            let mut leading =
                self.ranges.iter().take_while(|r| r.start <= point);
            return leading.any(|range| range.contains(point));
        }
        let after = self.ranges.partition_point(|range| range.start <= point);
        let candidate = after.checked_sub(1).and_then(|i| self.ranges.get(i));
        candidate.is_some_and(|range| range.contains(point))
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

#[cfg(test)]
#[path = "set_test.rs"]
mod search_tests;
