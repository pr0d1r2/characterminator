//! The `stats` TOTAL: every counted file summed, so a reader of a
//! thousand rows is not left to add them up (V96).
//!
//! Only files that were COUNTED are in it: a skipped file has no figure,
//! and is named below the total rather than folded into it as a zero
//! (`src/scan:V8`).

use crate::render::FileStats;
use crate::tokens::Count;

/// The sum of a run's rows, and how many rows it summed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Total {
    pub files: usize,
    pub outside: u64,
    pub bytes: u64,
    pub now: Count,
    pub after: Count,
}

/// The total of `rows`, or `None` when nothing was counted: a total of
/// no files is a row of zeros that reads like a measurement.
pub(super) fn of(rows: &[FileStats<'_>]) -> Option<Total> {
    let first = rows.first()?;
    let start = Total {
        files: 0,
        outside: 0,
        bytes: 0,
        now: zero(first.now),
        after: zero(first.after),
    };
    Some(rows.iter().fold(start, add))
}

/// A run counts every file one way (`src/tokens:V10`), so the method of
/// the first row is the method of the sum.
const fn zero(like: Count) -> Count {
    Count {
        tokens: 0,
        method: like.method,
    }
}

fn add(sum: Total, row: &FileStats<'_>) -> Total {
    Total {
        files: sum.files.saturating_add(1),
        outside: sum.outside.saturating_add(row.outside),
        bytes: sum.bytes.saturating_add(row.bytes),
        now: plus(sum.now, row.now),
        after: plus(sum.after, row.after),
    }
}

const fn plus(sum: Count, more: Count) -> Count {
    Count {
        tokens: sum.tokens.saturating_add(more.tokens),
        method: sum.method,
    }
}

#[cfg(test)]
mod tests {
    use super::of;
    use crate::render::FileStats;
    use crate::tokens::{Count, Method};

    fn row(path: &str, outside: u64) -> FileStats<'_> {
        let count = |tokens| Count {
            tokens,
            method: Method::Estimate,
        };
        FileStats {
            path,
            outside,
            bytes: 10,
            now: count(4),
            after: count(3),
        }
    }

    #[test]
    fn every_counted_file_is_summed() {
        let total = of(&[row("a", 1), row("b", 2)]);
        let said = total.map(|t| (t.files, t.outside, t.bytes));
        assert_eq!(said, Some((2, 3, 20)));
        let tokens = total.map(|t| (t.now.tokens, t.after.tokens));
        assert_eq!(tokens, Some((8, 6)));
    }

    #[test]
    fn nothing_counted_has_no_total() {
        assert_eq!(of(&[]), None);
    }
}
