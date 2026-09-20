//! Say it once: the human table and the stable json contract.
//!
//! See `src/render/SPEC.md`. This node owns HOW a report is written and
//! nothing about what is worth reporting. Every renderer therefore takes its
//! report as DATA: no sibling node is called from here, which is `src:V39`
//! held to its strictest reading -- this module compiles against the sibling
//! type vocabulary alone.
//!
//! The two forms are not equals. The json form is a STABLE CONTRACT (V11):
//! its keys and their meanings do not change under a caller, and its tests
//! assert whole documents exactly. The human form is COSMETIC and may
//! change; only the one line the root spec's interface section fixes is
//! pinned by a test.
//!
//! No serde, no serde_json. The documents are written by hand, compact and
//! pure ASCII: every non-ASCII code point leaves as a `\u` escape, so a
//! report never carries back in the characters the tool exists to drive out.
//! A renderer this shape needs no dependency, and whether the crate takes
//! one is a decision for the crate rather than for one node.
//!
//! Renderers return text with NO trailing newline, and an empty human report
//! as the empty string: the caller owns its line endings and decides whether
//! silence is worth printing.
//!
//! `guard` is absent on purpose. Its decision document is a HARNESS protocol
//! rather than a report of this tool's findings, and `src/cli:V35` gives it
//! to the cli node along with an open question about what non-hazard
//! violations become in it.

mod escape;
mod human;
mod json;
mod name;
mod order;
mod value;

use crate::charset::CharSet;
use crate::fix::Rewrite;
use crate::lint::Finding;
use crate::rules::Rule;
use crate::scan::Unreadable;
use crate::tokens::Count;

/// Which rendering a caller wants. The json form is a stable contract; the
/// human form is cosmetic and may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Human,
    Json,
}

/// One violation as it is REPORTED: the finding, the file it sits in, and
/// the set that did not contain it.
///
/// A `Finding` knows nothing of either, because the lint node reports what
/// it found rather than where a report should say it came from.
///
/// The path is a `&str` and not a `Path`: what a non-UTF-8 path should print
/// is not settled here, so a caller that has one decides how to name it
/// before handing it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation<'a> {
    pub path: &'a str,
    pub finding: Finding,
    pub set: &'a str,
}

/// A file that could NOT be read, named rather than dropped: one that
/// reported nothing would be indistinguishable from a clean one
/// (`src/scan:V8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skipped<'a> {
    pub path: &'a str,
    pub reason: Unreadable,
}

/// One rewrite, and the file it belongs to. The same shape serves `fix` and
/// `fix --check`, because the difference between them is whether the write
/// happened, not what is reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change<'a> {
    pub path: &'a str,
    pub rewrite: Rewrite,
}

/// One file's row in `stats`: how much sits outside the set, how big the
/// file is, and what it costs now against what it would cost after `fix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStats<'a> {
    pub path: &'a str,
    pub outside: u64,
    pub bytes: u64,
    pub now: Count,
    pub after: Count,
}

/// What `explain` answers: the effective set and the rule that won it.
///
/// `path` is absent when the question was asked of the whole repo rather
/// than of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation<'a> {
    pub path: Option<&'a str>,
    pub set: &'a CharSet,
    /// The rule that won, or NONE when nothing matched.
    ///
    /// Absent is not the same as a rule granting `ascii`: `src/rules:V1`
    /// gives an unmatched path the strict default with no line behind it,
    /// and a synthesised rule would put an origin in the report that no
    /// file could be opened at.
    pub rule: Option<&'a Rule>,
}

/// `check`: the violations, then the files that could not be read.
pub fn check(
    format: Format,
    violations: &[Violation<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    match format {
        Format::Human => human::check(violations, skipped),
        Format::Json => json::check(violations, skipped),
    }
}

/// `fix` and `fix --check`: the rewrites, then the files that could not be
/// read.
pub fn fix(
    format: Format,
    changes: &[Change<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    match format {
        Format::Human => human::fix(changes, skipped),
        Format::Json => json::fix(changes, skipped),
    }
}

/// `stats`: one row per file.
pub fn stats(format: Format, files: &[FileStats<'_>]) -> String {
    match format {
        Format::Human => human::stats(files),
        Format::Json => json::stats(files),
    }
}

/// `explain`: the effective set and the winning rule.
pub fn explain(format: Format, explanation: &Explanation<'_>) -> String {
    match format {
        Format::Human => human::explain(explanation),
        Format::Json => json::explain(explanation),
    }
}

/// `sets`: the builtin sets and their members.
pub fn sets(format: Format, sets: &[CharSet]) -> String {
    match format {
        Format::Human => human::sets(sets),
        Format::Json => json::sets(sets),
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, check, sets};
    use crate::charset::CharSet;

    #[test]
    fn the_format_chooses_the_form() {
        assert_eq!(check(Format::Human, &[], &[]), "");
        assert!(check(Format::Json, &[], &[]).starts_with('{'));
    }

    #[test]
    fn every_verb_answers_in_both_forms() {
        let ascii = CharSet {
            name: String::from("ascii"),
            ranges: vec![],
        };
        let one = std::slice::from_ref(&ascii);
        assert_eq!(sets(Format::Human, one), "ascii none");
        assert!(sets(Format::Json, one).starts_with("{\"verb\""));
    }
}
