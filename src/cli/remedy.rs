//! What `fix` would do with each thing `check` found (`src/render:V122`),
//! and what `fix` leaves (`src/cli:V80`).
//!
//! Both questions are answered by RUNNING the fix, never by guessing from
//! the map: a rewrite depends on the family, the classes, the emoji
//! sequences and the words map, and a guess that disagreed with `fix`
//! would be a `replacement` the tool then does not write.

use crate::fix::{Fixed, Map};
use crate::judge::{Judge, Looked, inspect};
use crate::lint::{Finding, Levels};
use crate::render::Remedy;
use crate::scan::Hit;

/// One text, judged and fixed: what a remedy is read from.
pub(super) struct Case<'a> {
    pub judge: &'a Judge<'a>,
    pub levels: &'a Levels,
    pub text: &'a str,
}

impl<'a> Case<'a> {
    pub(super) const fn new(
        judge: &'a Judge<'a>,
        levels: &'a Levels,
        text: &'a str,
    ) -> Self {
        Self {
            judge,
            levels,
            text,
        }
    }

    /// What the rewrite LEAVES, judged as `check` judges it, in the
    /// coordinates of the fixed output: where it sits in the file `fix`
    /// writes.
    pub(super) fn left(&self, fixed: &Fixed) -> Vec<Finding> {
        let output = fixed.output.as_bytes();
        match inspect(output, self.judge, self.levels) {
            Looked::Findings(found) => found,
            Looked::Unread(_) => Vec::new(),
        }
    }

    /// [`Case::left`], placed back where each finding sits in the text
    /// that was fixed (`src/fix/emoji:V65`): the file on disk when nothing
    /// is written.
    pub(super) fn left_in_place(&self, fixed: &Fixed) -> Vec<Finding> {
        let mut findings = self.left(fixed);
        fixed.place(self.text, &mut findings, hit_of);
        findings
    }

    /// What is left, where it sits on disk once the run is over (`src/cli:V80`,
    /// B71): in the output when `fix` writes it, else in the text as it is.
    /// Reported at the ORIGINAL position after a write, it named a column the
    /// written file no longer had, and the next `check` disagreed.
    pub(super) fn left_on_disk(
        &self,
        fixed: &Fixed,
        wrote: bool,
    ) -> Vec<Finding> {
        if wrote {
            self.left(fixed)
        } else {
            self.left_in_place(fixed)
        }
    }

    /// One remedy per finding, in `findings`' order. A text the engine
    /// refuses to fix (`src/fix:V5`, `src/fix:V6`) is written by no `fix`
    /// run, so every finding in it has none.
    pub(super) fn remedies(
        &self,
        map: &Map,
        findings: &[Finding],
    ) -> Vec<Remedy> {
        let Ok(fixed) = self.judge.fix(self.text, map) else {
            return vec![Remedy::default(); findings.len()];
        };
        let left = bytes(&self.left_in_place(&fixed));
        let mut starts: Vec<(usize, &str)> = fixed
            .report
            .rewrites
            .iter()
            .map(|r| (r.hit.position.byte, r.to.as_str()))
            .collect();
        starts.sort_unstable_by_key(|start| start.0);
        let one = |finding: &Finding| remedy(finding, &starts, &left);
        findings.iter().map(one).collect()
    }
}

/// Each finding's byte offset, ascending, for a binary search.
fn bytes(findings: &[Finding]) -> Vec<usize> {
    let mut at: Vec<usize> =
        findings.iter().map(|f| f.hit.position.byte).collect();
    at.sort_unstable();
    at
}

fn remedy(
    finding: &Finding,
    starts: &[(usize, &str)],
    left: &[usize],
) -> Remedy {
    let byte = finding.hit.position.byte;
    let to = starts
        .binary_search_by_key(&byte, |start| start.0)
        .ok()
        .and_then(|at| starts.get(at))
        .map(|start| start.1.to_owned());
    Remedy {
        to,
        fixable: left.binary_search(&byte).is_err(),
    }
}

/// A finding's hit, for [`Fixed::place`].
fn hit_of(finding: &mut Finding) -> &mut Hit {
    &mut finding.hit
}
