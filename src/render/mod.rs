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
//! A third form, SARIF, exists for `check` alone, so code scanning can read
//! its findings (V50). It is built from the same json primitives, and its
//! `rules` array is the one place this node READS a sibling's table: the
//! lint registry, as data, so a newly registered lint needs no edit here.
//!
//! `guard` has no renderer here, on purpose. Its decision document is a
//! HARNESS protocol rather than a report of this tool's findings, so
//! `src/cli/guard:V35` gives it to the guard node. What it borrows from
//! this one is the two spellings a report must not fork: a json string
//! literal, pure ASCII, and a code point as `U+XXXX`. Both are re-exported
//! below, so the harness document and the reports cannot disagree about
//! either.

mod escape;
mod human;
mod json;
mod line;
mod name;
mod order;
mod sarif;
mod total;
mod value;

pub(crate) use escape::string as json_string;
pub(crate) use name::codepoint;

use crate::charset::CharSet;
use crate::fix::Rewrite;
use crate::lint::{Finding, Level};
use crate::rules::{LevelChoice, Origin, Rule, Sourced};
use crate::scan::Unreadable;
use crate::tokens::Count;

/// Which rendering a caller wants. The json form is a stable contract; the
/// human form is cosmetic and may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Human,
    Json,
    /// A SARIF 2.1.0 log (V50). SARIF carries RESULTS, and only `check`
    /// has findings to put in it, so only `check` has a SARIF shape: the
    /// cli refuses the pairing for every other verb, and a library caller
    /// that asks anyway gets that verb's json contract rather than a log
    /// claiming a clean run nobody performed.
    Sarif,
}

/// One violation as it is REPORTED: the finding, the file it sits in, and
/// the set it was judged against -- the set that did not contain it, for
/// the ordinary `outside-set` finding.
///
/// A PEDANTIC finding (`src/lint/pedantic:V55`) fires inside the set, so `set`
/// carries the set in force, which is still true of the file, and the
/// lint name says what is wrong. The human line prints that lint name in
/// the set's place; the json keeps both, unchanged in shape (V11).
///
/// A `Finding` knows nothing of either, because the lint node reports what
/// it found rather than where a report should say it came from.
///
/// The path is a `&str` and not a `Path`: what a non-UTF-8 path should print
/// is not settled here, so a caller that has one decides how to name it
/// before handing it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Violation<'a> {
    pub path: &'a str,
    pub finding: Finding,
    pub set: &'a str,
}

/// A file that could NOT be read, named rather than dropped: one that
/// reported nothing would be indistinguishable from a clean one
/// (`src/scan:V8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Skipped<'a> {
    pub path: &'a str,
    pub reason: Unreadable,
}

/// One rewrite, and the file it belongs to. The same shape serves `fix` and
/// `fix --check`, because the difference between them is whether the write
/// happened, not what is reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change<'a> {
    pub path: &'a str,
    /// BORROWED: a run of a million rewrites holds each one once (R18).
    pub rewrite: &'a Rewrite,
}

/// One file's row in `stats`: how much sits outside the set, how big the
/// file is, and what it costs now against what it would cost after `fix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStats<'a> {
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
pub(crate) struct Explanation<'a> {
    pub path: Option<&'a str>,
    pub set: &'a CharSet,
    /// The rule that won, or NONE when nothing matched.
    ///
    /// Absent is not the same as a rule granting `ascii`: `src/rules:V1`
    /// gives an unmatched path the strict default with no line behind it,
    /// and a synthesised rule would put an origin in the report that no
    /// file could be opened at.
    pub rule: Option<&'a Rule>,
    /// What is IN FORCE, each setting with the line that set it
    /// (`src/rules:V20`). The rule above won the grant, and that is all
    /// it won: a level-only line moves levels (`src/rules:V56`) and
    /// another may name the family (`src/rules:V29`), so a report naming
    /// only the winner hid the lines that decided the rest (B28).
    pub in_force: InForce<'a>,
}

/// The effective family and levels of an [`Explanation`], with origins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InForce<'a> {
    /// The fidelity family, never empty: `text` when no rule named one.
    pub family: &'a str,
    /// The line that named the family, or none for the default.
    pub family_origin: Option<&'a Origin>,
    /// The bare level (`!warn`) and its line, or none when no rule set one.
    pub level: Option<Sourced<'a, Level>>,
    /// Per lint or group, each with the line that set it last.
    pub levels: Vec<Sourced<'a, LevelChoice>>,
}

/// `check`: the violations, then the files that could not be read.
/// Test-only: the binary reports through [`check_batches`].
#[cfg(test)]
pub(crate) fn check(
    format: Format,
    violations: &[Violation<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    match format {
        Format::Human => human::check(violations, skipped),
        Format::Json => json::check(violations, skipped),
        Format::Sarif => sarif::check(violations, skipped),
    }
}

/// Violations that share one path and one set, held ONCE for all of them
/// rather than once per finding (R17): a file with two million findings
/// is two million findings, not two million copies of its name.
///
/// [`check_batches`] reports a list of these exactly as [`check`] reports
/// the [`Violation`]s they expand to, in that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Batch<'a> {
    pub path: &'a str,
    pub set: &'a str,
    pub findings: &'a [Finding],
}

/// [`check`], over batches: the same report, byte for byte, as over the
/// violations they expand to.
pub(crate) fn check_batches(
    format: Format,
    batches: &[Batch<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    let lines = order::batches(batches);
    match format {
        Format::Human => human::check_lines(lines, skipped),
        Format::Json => json::check_lines(lines, skipped),
        Format::Sarif => sarif::check_lines(lines, skipped),
    }
}

/// `fix` and `fix --check`: the rewrites, then the characters no map entry
/// covers (`src/fix:V4`), in `check`'s own row shape, then the files that
/// could not be read. What is left arrives as [`Batch`]es, as `check`'s
/// findings do: a path and a set held once per file, not once per row
/// (`src/fix:R18`).
pub(crate) fn fix(
    format: Format,
    changes: &[Change<'_>],
    unmapped: &[Batch<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    let unmapped = order::batches(unmapped);
    match format {
        Format::Human => human::fix(changes, unmapped, skipped),
        Format::Json | Format::Sarif => json::fix(changes, unmapped, skipped),
    }
}

/// `stats`: one row per file, then the files that are not text, named as
/// `check` names them (`src/scan:V8`).
pub(crate) fn stats(
    format: Format,
    files: &[FileStats<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    match format {
        Format::Human => human::stats(files, skipped),
        Format::Json | Format::Sarif => json::stats(files, skipped),
    }
}

/// `explain`: the effective set and the winning rule.
pub(crate) fn explain(format: Format, explanation: &Explanation<'_>) -> String {
    match format {
        Format::Human => human::explain(explanation),
        Format::Json | Format::Sarif => json::explain(explanation),
    }
}

/// `sets`: the builtin sets and their members.
pub(crate) fn sets(format: Format, sets: &[CharSet]) -> String {
    match format {
        Format::Human => human::sets(sets),
        Format::Json | Format::Sarif => json::sets(sets),
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

    /// SARIF carries results, so only `check` has a log; every other verb
    /// answers with its json contract rather than an empty, clean log.
    #[test]
    fn only_check_has_a_sarif_shape() {
        let log = check(Format::Sarif, &[], &[]);
        assert!(log.contains("\"version\":\"2.1.0\""), "{log}");
        assert_eq!(sets(Format::Sarif, &[]), sets(Format::Json, &[]));
    }
}

/// R17: a path is spelled once per RUN of findings rather than once per
/// finding, so these hold the spelling to the path it belongs to when
/// the path changes and changes back.
#[cfg(test)]
mod spelled_once {
    use super::{Format, Violation, check};
    use crate::lint::{Finding, Level};
    use crate::scan::{Hit, Position};

    /// U+202E RIGHT-TO-LEFT OVERRIDE, in a file name.
    const ODD: &str = "b\u{202e}.md";

    fn at(path: &str, byte: usize) -> Violation<'_> {
        let (line, column, character) = (1, byte, '\u{2014}');
        let position = Position { line, column, byte };
        let hit = Hit {
            position,
            character,
        };
        let (lint, level) = (crate::lint::OUTSIDE_SET, Level::Deny);
        let (finding, set) = (Finding { hit, lint, level }, "ascii");
        Violation { path, finding, set }
    }

    fn report(format: Format) -> String {
        let items = [at(ODD, 2), at("a.md", 1), at(ODD, 1), at("c.md", 1)];
        check(format, &items, &[])
    }

    #[test]
    fn every_human_line_names_its_own_path() {
        let expected = concat!(
            "a.md:1:1 U+2014 ascii\n",
            "b<U+202E>.md:1:1 U+2014 ascii\n",
            "b<U+202E>.md:1:2 U+2014 ascii\n",
            "c.md:1:1 U+2014 ascii"
        );
        assert_eq!(report(Format::Human), expected);
    }

    #[test]
    fn every_json_violation_names_its_own_path() {
        let said = report(Format::Json);
        let paths: Vec<&str> = said
            .split("\"path\":")
            .skip(1)
            .filter_map(|rest| rest.split(',').next())
            .collect();
        let odd = "\"b\\u202e.md\"";
        assert_eq!(paths, ["\"a.md\"", odd, odd, "\"c.md\""]);
    }

    #[test]
    fn every_sarif_result_names_its_own_uri() {
        let said = report(Format::Sarif);
        let uris: Vec<&str> = said
            .split("\"uri\":")
            .skip(1)
            .filter_map(|rest| rest.split('}').next())
            .collect();
        let odd = "\"b%E2%80%AE.md\"";
        assert_eq!(uris, ["\"a.md\"", odd, odd, "\"c.md\""]);
    }
}

/// R17: a batch is reported exactly as the violations it expands to,
/// whatever order the batches come in, and when one path's batches
/// interleave, which takes the fallback.
#[cfg(test)]
mod batched {
    use super::{Batch, Format, Violation, check, check_batches};
    use crate::lint::{Finding, Level};
    use crate::scan::{Hit, Position};

    fn finding(byte: usize) -> Finding {
        let (line, column, character) = (1, byte, '\u{2014}');
        let position = Position { line, column, byte };
        let hit = Hit {
            position,
            character,
        };
        let lint = crate::lint::OUTSIDE_SET;
        let level = Level::Deny;
        Finding { hit, lint, level }
    }

    fn batch<'a>(
        path: &'a str,
        set: &'a str,
        findings: &'a [Finding],
    ) -> Batch<'a> {
        Batch {
            path,
            set,
            findings,
        }
    }

    fn expanded<'a>(batches: &[Batch<'a>]) -> Vec<Violation<'a>> {
        let one = |b: &Batch<'a>| {
            let (path, set) = (b.path, b.set);
            b.findings.iter().map(move |f| Violation {
                path,
                finding: f.clone(),
                set,
            })
        };
        batches.iter().flat_map(one).collect()
    }

    fn same_report(batches: &[Batch<'_>]) {
        let flat = expanded(batches);
        for format in [Format::Human, Format::Json, Format::Sarif] {
            let said = check_batches(format, batches, &[]);
            assert_eq!(said, check(format, &flat, &[]), "{format:?}");
        }
    }

    #[test]
    fn batches_in_any_path_order_report_as_their_violations() {
        let (a, b) = ([finding(1), finding(4)], [finding(2), finding(9)]);
        let hazard = [finding(3)];
        same_report(&[
            batch("b.md", "ascii", &b),
            batch("a.md", "ascii", a.get(..1).unwrap_or_default()),
            batch("a.md", "hazard", &hazard),
            batch("a.md", "ascii", a.get(1..).unwrap_or_default()),
        ]);
    }

    /// One path in two batches whose offsets interleave: the fallback.
    #[test]
    fn interleaved_batches_of_one_path_report_as_their_violations() {
        let (one, two) = ([finding(1), finding(7)], [finding(3), finding(5)]);
        same_report(&[
            batch("a.md", "ascii", &one),
            batch("a.md", "any", &two),
        ]);
    }
}
