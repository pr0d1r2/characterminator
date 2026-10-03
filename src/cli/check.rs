//! The `check` verb (T8): report every character outside its file's set.
//!
//! Composition only. Every answer comes from the node that owns it: which
//! rule governs a path (`src/rules`), what that rule's sets contain
//! (`src/charset`), where a character sits (`src/scan`), how loudly it is
//! said (`src/lint`), and how the report reads (`src/render`). V7 makes
//! this one of the two verbs that GATE. What a file's bytes are judged to
//! hold is the shared engine's answer (`src/judge`); this file walks the
//! tree, gathers those answers, and reports them.

use super::remedy::Case;
use crate::charset::CharSet;
use crate::fix::Map;
use crate::judge::{Checker, Config, File, Looked, files, judged};
use crate::lint::{Finding, Group, Levels, Lint, exit_code};
use crate::render::{self, Batch, Format, Remedy, Skipped};
use crate::scan::Unreadable;
use std::path::Path;

/// One file's findings with the strings they are reported against, owned so the
/// borrowed `render` batches can point at them. The path and set are held ONCE
/// per file, not copied into every finding (`src/render:R17`).
pub(super) struct Row {
    pub path: String,
    pub set: String,
    pub findings: Vec<Finding>,
    /// What `fix` would do with each finding, parallel to `findings`, or
    /// empty when the report does not carry it (`src/render:V122`).
    pub remedies: Vec<Remedy>,
}

/// A file that was skipped, owned for the same reason.
pub(super) struct Skip {
    pub path: String,
    pub reason: Unreadable,
}

/// A finished report: what to print, and what to exit with.
pub(super) struct Report {
    pub text: String,
    pub code: u8,
}

/// What one run holds for every file: the judge, and the map when the
/// report says what `fix` would write (`src/render:V122`).
struct Walk {
    checker: Checker,
    map: Option<Map>,
}

impl Walk {
    fn gather(&self, root: &Path, paths: &[String]) -> Result<Found, String> {
        let mut found = Found::default();
        for file in files(root, paths)? {
            let File { shown, text, .. } = file?;
            let (set, levels) = self.checker.shared_law(&shown)?;
            let looked = match text {
                Ok(text) => self.judged(&text, &set, &levels),
                Err(reason) => (Looked::Unread(reason), Vec::new()),
            };
            found.absorb(shown, &set, looked);
        }
        Ok(found)
    }

    /// One text's findings, each with its remedy when the map is here.
    fn judged(&self, text: &str, set: &CharSet, levels: &Levels) -> Judged {
        let judge = self.checker.judge(set);
        let findings = judged(text, &judge, levels);
        let remedies = match &self.map {
            Some(map) if !findings.is_empty() => {
                Case::new(&judge, levels, text).remedies(map, &findings)
            }
            _ => Vec::new(),
        };
        (Looked::Findings(findings), remedies)
    }
}

/// A text's verdict and the remedies of its findings, parallel.
type Judged = (Looked, Vec<Remedy>);

/// What the walk accumulated.
#[derive(Default)]
struct Found {
    rows: Vec<Row>,
    skips: Vec<Skip>,
}

impl Found {
    /// A clean file adds no row: only a finding needs a path to print.
    fn absorb(&mut self, path: String, set: &CharSet, looked: Judged) {
        match looked {
            (Looked::Unread(reason), _) => {
                self.skips.push(Skip { path, reason });
            }
            (Looked::Findings(found), _) if found.is_empty() => {}
            (Looked::Findings(findings), remedies) => {
                self.rows
                    .push(Row::new(path, &set.name, findings, remedies));
            }
        }
    }
}

impl Row {
    /// One file's row, shrunk: a run of millions holds each once (R17).
    pub(super) fn new(
        path: String,
        set: &str,
        mut findings: Vec<Finding>,
        remedies: Vec<Remedy>,
    ) -> Self {
        findings.shrink_to_fit();
        let set = set.to_owned();
        Self {
            path,
            set,
            findings,
            remedies,
        }
    }

    /// The batch of `findings`, a run of this row's starting at `at`,
    /// with the remedies that sit beside them.
    fn batch<'r>(&'r self, at: usize, findings: &'r [Finding]) -> Batch<'r> {
        let end = at.saturating_add(findings.len());
        let set = findings.first().map_or(self.set.as_str(), |first| {
            judged_against(first.lint, &self.set)
        });
        Batch {
            path: &self.path,
            set,
            findings,
            remedies: self.remedies.get(at..end).unwrap_or(&[]),
        }
    }
}

/// The set a finding names in the report.
///
/// A hazard names `hazard`, not the file's set: the set did not decide it
/// (V34), and `notes.md:1:1 U+202E any` would read as though the override
/// fell outside `any`, which is nonsense. Which hazard it was is the
/// lint's name, carried in the json.
pub(super) fn judged_against(lint: Lint, set: &str) -> &str {
    if lint.group == Group::Hazard {
        Group::Hazard.name()
    } else {
        set
    }
}

/// One file's findings as the batches `render` reads: each run of
/// findings that names one set, in the file's own byte order.
pub(super) fn batches(source: &Row) -> impl Iterator<Item = Batch<'_>> {
    let named = |f: &Finding| judged_against(f.lint, &source.set);
    let mut start = 0_usize;
    source
        .findings
        .chunk_by(move |a, b| named(a) == named(b))
        .map(move |findings| {
            let at = start;
            start = start.saturating_add(findings.len());
            source.batch(at, findings)
        })
}

/// Run `check` over a repository.
///
/// `paths` empty means the git-tracked fileset; naming paths reaches
/// untracked files too (`src/tokens:V9`).
pub(super) fn run(
    config: &Config,
    paths: &[String],
    format: Format,
) -> Result<Report, String> {
    // Only the json carries what `fix` would write: running the fix costs
    // a second pass over every file with a finding, which a human report
    // and a SARIF log would pay for nothing.
    let walk = Walk {
        checker: Checker::configured(config)?,
        map: (format == Format::Json).then(|| config.map()).transpose()?,
    };
    let found = walk.gather(&config.root, paths)?;
    let batches: Vec<Batch<'_>> = found.rows.iter().flat_map(batches).collect();
    let skipped: Vec<Skipped<'_>> = found.skips.iter().map(skip).collect();
    Ok(Report {
        text: render::check_batches(format, &batches, &skipped),
        code: found_code(&found.rows).max(unreadable_code(&found.skips)),
    })
}

/// The findings' exit code, read in place: a run of millions need not copy
/// every finding just to ask whether one of them fails (`src/render:R17`).
pub(super) fn found_code(rows: &[Row]) -> u8 {
    rows.iter()
        .map(|row| exit_code(&row.findings))
        .max()
        .unwrap_or(0)
}

/// Invalid UTF-8 is an ERROR, exit 1 (`src/scan:V8`): the file claims to
/// be text and is not, so no verdict about its characters was reached
/// (B22). A binary skip is not: the file never claimed to be text, and
/// failing on every tracked PNG would make the gate unusable. One code for
/// every format, since json and SARIF report the same run.
pub(super) fn unreadable_code(skips: &[Skip]) -> u8 {
    let broken =
        |skip: &Skip| matches!(skip.reason, Unreadable::NotUtf8 { .. });
    u8::from(skips.iter().any(broken))
}

pub(super) fn skip(source: &Skip) -> Skipped<'_> {
    Skipped {
        path: &source.path,
        reason: source.reason,
    }
}

#[cfg(test)]
#[path = "check_test.rs"]
mod tests;
