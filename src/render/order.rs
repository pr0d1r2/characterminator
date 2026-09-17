//! ONE order for every report: by path, then by byte offset
//! (`src/scan:V12`).
//!
//! Sorting lives here rather than in either form so that the human line and
//! the json document list the same things in the same sequence. A report
//! whose order depended on which form you asked for would make two runs over
//! one tree impossible to diff.
//!
//! Every function returns REFERENCES into the caller's slice rather than
//! reordering it: a renderer has no business mutating its input.

use crate::render::{Change, FileStats, Skipped, Violation};

/// Violations in report order.
pub fn violations<'s, 'v>(
    items: &'s [Violation<'v>],
) -> Vec<&'s Violation<'v>> {
    let mut sorted: Vec<&Violation<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| (item.path, item.finding.hit.position.byte));
    sorted
}

/// Rewrites in report order.
pub fn changes<'s, 'c>(items: &'s [Change<'c>]) -> Vec<&'s Change<'c>> {
    let mut sorted: Vec<&Change<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| (item.path, item.rewrite.hit.position.byte));
    sorted
}

/// Unread files in report order. There is no offset to fall back on, so the
/// path alone decides.
pub fn skipped<'s, 'k>(items: &'s [Skipped<'k>]) -> Vec<&'s Skipped<'k>> {
    let mut sorted: Vec<&Skipped<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| item.path);
    sorted
}

/// Stats rows in report order, by path for the same reason.
pub fn stats<'s, 'f>(items: &'s [FileStats<'f>]) -> Vec<&'s FileStats<'f>> {
    let mut sorted: Vec<&FileStats<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| item.path);
    sorted
}

#[cfg(test)]
mod tests {
    use super::{skipped, violations};
    use crate::lint::{Finding, Group, Level, Lint};
    use crate::render::{Skipped, Violation};
    use crate::scan::{Hit, Position, Unreadable};

    const CHARSET: Lint = Lint {
        name: "charset",
        group: Group::Charset,
    };

    fn hit(byte: usize) -> Hit {
        let position = Position {
            line: 1,
            column: 1,
            byte,
        };
        Hit {
            position,
            character: 'x',
        }
    }

    fn at(path: &str, byte: usize) -> Violation<'_> {
        let finding = Finding {
            hit: hit(byte),
            lint: CHARSET,
            level: Level::Deny,
        };
        Violation {
            path,
            finding,
            set: "ascii",
        }
    }

    fn skip(path: &str) -> Skipped<'_> {
        Skipped {
            path,
            reason: Unreadable::Binary,
        }
    }

    fn seen<'a>(items: &[&'a Violation<'a>]) -> Vec<(&'a str, usize)> {
        items
            .iter()
            .map(|item| (item.path, item.finding.hit.position.byte))
            .collect()
    }

    #[test]
    fn violations_sort_by_path_then_byte_offset() {
        let input = [at("b.rs", 1), at("a.rs", 9), at("a.rs", 2)];
        let order = [("a.rs", 2), ("a.rs", 9), ("b.rs", 1)];
        assert_eq!(seen(&violations(&input)), order);
    }

    #[test]
    fn the_byte_offset_breaks_a_tie_within_one_path() {
        let input = [at("a.rs", 30), at("a.rs", 4)];
        assert_eq!(seen(&violations(&input)), [("a.rs", 4), ("a.rs", 30)]);
    }

    #[test]
    fn sorting_leaves_the_callers_slice_alone() {
        let input = [at("b.rs", 1), at("a.rs", 2)];
        let _sorted = violations(&input);
        let untouched: Vec<&Violation<'_>> = input.iter().collect();
        assert_eq!(seen(&untouched), [("b.rs", 1), ("a.rs", 2)]);
    }

    #[test]
    fn unread_files_sort_by_path() {
        let items = [skip("z.bin"), skip("a.bin")];
        let sorted = skipped(&items);
        let paths: Vec<&str> = sorted.iter().map(|item| item.path).collect();
        assert_eq!(paths, ["a.bin", "z.bin"]);
    }
}
