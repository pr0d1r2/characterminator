//! The order a report goes out in: by path, then by byte offset (`V12`).

use super::Hit;

/// Sort located characters by path, then by byte offset.
///
/// THE PATH TRAVELS BESIDE THE HIT, not inside it. A `Hit` is a position
/// in one text, and `scan_str` is handed a `&str` with no idea where it
/// came from; pairing the two here keeps this node free of any opinion
/// about how a caller names files. Generic over the path so a caller can
/// carry `PathBuf`, `&Path` or `String` without this node choosing.
///
/// The byte offset is the tiebreaker rather than line and column because
/// it is the only one of the three that is a single total order, and the
/// three agree anyway: a larger offset in one text is always a later
/// line, or the same line and a later column.
///
/// The sort is STABLE, so rows that share a path and an offset keep the
/// order the caller built them in.
pub fn sort_hits<P: Ord>(rows: &mut [(P, Hit)]) {
    rows.sort_by(|left, right| {
        let by_path = left.0.cmp(&right.0);
        by_path.then(left.1.position.byte.cmp(&right.1.position.byte))
    });
}

#[cfg(test)]
mod tests {
    use super::sort_hits;
    use crate::scan::{Hit, Position};

    fn hit(byte: usize) -> Hit {
        let position = Position {
            line: 1,
            column: 1,
            byte,
        };
        Hit {
            position,
            character: '\u{200b}',
        }
    }

    #[test]
    fn rows_sort_by_path_then_by_byte_offset() {
        let mut rows =
            vec![("b.txt", hit(1)), ("a.txt", hit(9)), ("a.txt", hit(2))];

        sort_hits(&mut rows);

        let order: Vec<_> = rows
            .iter()
            .map(|row| (row.0, row.1.position.byte))
            .collect();
        assert_eq!(order, vec![("a.txt", 2), ("a.txt", 9), ("b.txt", 1)]);
    }

    #[test]
    fn the_path_wins_over_the_offset() {
        // A late offset under an early path still sorts first: the path
        // is the outer key, not a tiebreaker.
        let mut rows = vec![("z.txt", hit(0)), ("a.txt", hit(99))];

        sort_hits(&mut rows);

        assert_eq!(rows.first().map(|row| row.0), Some("a.txt"));
    }
}
