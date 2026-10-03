//! "Did you mean": the closest name to a word that matched none.
//!
//! One implementation for every closed list a reader can mistype -- the
//! verbs (`src/cli/usage:V119`) and the declared set names
//! (`src/cli/usage:V121`) -- so the threshold means the same everywhere.

/// The furthest a suggestion may be from what was typed. Two edits catch
/// a dropped, doubled or swapped letter (`chek`, `asci`, `cehck`) without
/// offering a stranger's name for a word that resembles nothing.
const FURTHEST: usize = 2;

/// The choice closest to `word`, if one is within two edits. Ties go to
/// the earlier choice, so the answer does not depend on hashing.
pub(crate) fn nearest<'a, I>(word: &str, choices: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let scored = choices.into_iter().map(|c| (distance(word, c), c));
    let close = scored.filter(|(d, _)| *d <= FURTHEST && *d > 0);
    close.min_by_key(|(d, _)| *d).map(|(_, choice)| choice)
}

/// Levenshtein distance over chars, one row at a time.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        row = next_row(&row, ca, &b, i.saturating_add(1));
    }
    row.last().copied().unwrap_or_default()
}

/// The row for one more char of `a`, from the row before it.
fn next_row(row: &[usize], ca: char, b: &[char], first: usize) -> Vec<usize> {
    let mut next = vec![first];
    let mut diagonal = row.first().copied().unwrap_or_default();
    for (j, cb) in b.iter().enumerate() {
        let up = row.get(j.saturating_add(1)).copied().unwrap_or(usize::MAX);
        let left = next.last().copied().unwrap_or(usize::MAX);
        let swap = diagonal.saturating_add(usize::from(ca != *cb));
        next.push(swap.min(up.saturating_add(1)).min(left.saturating_add(1)));
        diagonal = up;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::{distance, nearest};

    #[test]
    fn distance_counts_single_char_edits() {
        assert_eq!(distance("chek", "check"), 1);
        assert_eq!(distance("check", "check"), 0);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("kitten", "sitting"), 3);
    }

    #[test]
    fn a_suggestion_is_close_or_absent() {
        let verbs = ["check", "explain", "sets", "fix", "stats"];
        assert_eq!(nearest("chek", verbs), Some("check"));
        assert_eq!(nearest("stat", verbs), Some("stats"));
        assert_eq!(nearest("deploy", verbs), None);
        assert_eq!(nearest("check", verbs), None);
    }
}
