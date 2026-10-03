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

use crate::render::line::Line;
use crate::render::{Batch, Change, FileStats, Skipped, Violation};

/// Violations in report order.
pub(super) fn violations<'s, 'v>(
    items: &'s [Violation<'v>],
) -> Vec<&'s Violation<'v>> {
    let mut sorted: Vec<&Violation<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| (item.path, item.finding.hit.position.byte));
    sorted
}

/// Violations in report order, as the writers read them.
pub(super) fn lines<'s>(items: &'s [Violation<'_>]) -> Vec<Line<'s>> {
    violations(items).into_iter().map(Line::from).collect()
}

/// Batched violations in report order, WITHOUT a row per finding when
/// that order is already there (R17).
///
/// The one order is a stable sort of every violation by path, then byte
/// offset. Batches sorted stably by path and then expanded give exactly
/// that whenever the expansion is already ordered by both keys: equal
/// keys then sit in the order they were handed over in, which is what a
/// stable sort keeps. When it is not -- one path in two batches whose
/// offsets interleave, or a batch out of byte order -- every violation is
/// laid out and sorted the old way, so the answer never depends on which
/// path was taken.
pub(super) fn batches<'a>(items: &'a [Batch<'a>]) -> Lines<'a> {
    let total = items.iter().map(|b| b.findings.len()).sum();
    let mut sorted: Vec<&Batch<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| item.path);
    let flat = || sorted.iter().copied().flat_map(expand);
    let inner: Box<dyn Iterator<Item = Line<'a>> + 'a> =
        if flat().is_sorted_by_key(|line| key(&line)) {
            Box::new(sorted.into_iter().flat_map(expand))
        } else {
            Box::new(laid_out(items).into_iter())
        };
    Lines { inner, left: total }
}

/// Every violation, sorted the way [`violations`] sorts: the fallback.
fn laid_out<'a>(items: &'a [Batch<'a>]) -> Vec<Line<'a>> {
    let mut all: Vec<Line<'a>> = items.iter().flat_map(expand).collect();
    all.sort_by_key(key);
    all
}

/// Where a line falls in the one order.
fn key<'a>(line: &Line<'a>) -> (&'a str, usize) {
    (line.path, line.finding.hit.position.byte)
}

/// One batch, as the violations it stands for.
fn expand<'a>(batch: &'a Batch<'a>) -> impl Iterator<Item = Line<'a>> + 'a {
    batch.findings.iter().map(|finding| Line {
        path: batch.path,
        set: batch.set,
        finding,
    })
}

/// [`batches`]' answer: the lines, and how many are left, so a writer can
/// size its buffer once.
pub(super) struct Lines<'a> {
    inner: Box<dyn Iterator<Item = Line<'a>> + 'a>,
    left: usize,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Line<'a>> {
        let line = self.inner.next()?;
        self.left = self.left.saturating_sub(1);
        Some(line)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

/// Rewrites in report order.
pub(super) fn changes<'s, 'c>(items: &'s [Change<'c>]) -> Vec<&'s Change<'c>> {
    let mut sorted: Vec<&Change<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| (item.path, item.rewrite.hit.position.byte));
    sorted
}

/// Unread files in report order. There is no offset to fall back on, so the
/// path alone decides.
pub(super) fn skipped<'s, 'k>(
    items: &'s [Skipped<'k>],
) -> Vec<&'s Skipped<'k>> {
    let mut sorted: Vec<&Skipped<'_>> = items.iter().collect();
    sorted.sort_by_key(|item| item.path);
    sorted
}

/// Stats rows in report order, by path for the same reason.
pub(super) fn stats<'s, 'f>(
    items: &'s [FileStats<'f>],
) -> Vec<&'s FileStats<'f>> {
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
