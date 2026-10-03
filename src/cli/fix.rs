//! The `fix` and `fix --check` verbs (T50): rewrite what the map covers.
//!
//! The engine is `src/fix`; this reads the files, hands each one the set
//! it is judged against, and writes back only when asked. The split is
//! V7's: `fix --check` GATES and writes nothing, bare `fix` rewrites and
//! is therefore something a caller invokes deliberately.
//!
//! Nothing here decides what a character becomes. That is the map's, and
//! the map is data (`src/fix:V26`) plus whatever `.ctrm-map` declares
//! over it (`src/rules:V45`).
//!
//! A run holds what it reports ONCE (`src/fix:R18`): a file's path is
//! kept per file, not per row; a rewrite is moved, never copied, from the
//! engine into the report; and what is left is held as `check` holds its
//! findings, in batches the renderer borrows.

use super::check::{Row, Skip, batches, found_code, skip, unreadable_code};
use crate::fix::{self as engine, Map};
use crate::judge::{Checker, Config, File, Judge, Looked, files, inspect};
use crate::lint::{Finding, Levels};
use crate::render::{self, Batch, Change, Format, Skipped};
use crate::scan::{Hit, Unreadable};
use std::path::PathBuf;

/// What a run of `fix` produced.
pub(super) struct Report {
    pub text: String,
    pub code: u8,
}

/// One file's rewrites, owned so the borrowed render rows can point at
/// them, with the path held once for all of them.
struct Changed {
    path: String,
    rewrites: Vec<engine::Rewrite>,
}

/// What the walk accumulated.
#[derive(Default)]
struct Found {
    changed: Vec<Changed>,
    skips: Vec<Skip>,
    /// What `check` finds in the text `fix` leaves: characters outside
    /// their set that no map entry covers, hazards, and any lint the
    /// rules ask for, at their levels (V80). V4: kept as they are,
    /// REPORTED -- a silent drop is the one thing `fix` may never do, and
    /// an exit 1 that names nothing drops the reason instead (B23). One
    /// row per file, in `check`'s own shape, so the two verbs name a
    /// character alike.
    unmapped: Vec<Row>,
    /// What a bare `fix` will write, held until EVERY file has been judged
    /// (V72): a refusal or a read error on the tenth file must not leave
    /// the first nine rewritten behind a run that reports only the error.
    pending: Vec<(PathBuf, String)>,
}

/// Run `fix` over a repository.
///
/// `write` false is `--check`: the same walk, the same report, and no
/// file touched. The difference between the two verbs is whether the
/// write happened, not what is found.
///
/// # Errors
///
/// A configuration that cannot be read, a file that cannot be written,
/// or a rewrite the engine refused to trust (`src/fix:V5`, `src/fix:V6`).
pub(super) fn run(
    config: &Config,
    paths: &[String],
    format: Format,
    write: bool,
) -> Result<Report, String> {
    let pass = Pass::new(config, write)?;
    let mut found = Found::default();
    for file in files(&config.root, paths)? {
        pass.visit(file?, &mut found)?;
    }
    written(&found.pending)?;
    Ok(report_of(&found, format, pass.write))
}

/// The second phase (V72): every file was judged and none refused, so
/// the rewrites go to disk.
fn written(pending: &[(PathBuf, String)]) -> Result<(), String> {
    for (full, text) in pending {
        std::fs::write(full, text)
            .map_err(|e| format!("{}: {e}", full.display()))?;
    }
    Ok(())
}

/// What one run holds for every file it visits, so a per-file call takes
/// the file and nothing else.
struct Pass {
    checker: Checker,
    map: Map,
    write: bool,
}

impl Pass {
    fn new(config: &Config, write: bool) -> Result<Self, String> {
        Ok(Self {
            checker: Checker::configured(config)?,
            map: config.map()?,
            write,
        })
    }

    /// One file: judge it, rewrite it, queue the write when asked.
    fn visit(&self, file: File, found: &mut Found) -> Result<(), String> {
        let File { full, shown, text } = file;
        let Some(text) = text_of(text, &shown, found) else {
            return Ok(());
        };
        let (set, levels) = self.checker.shared_law(&shown)?;
        let judge = self.checker.judge(&set);
        let fixed = self.fixed(&judge, &text, &shown)?;
        found.keep(&shown, &set.name, left(&judge, &levels, &text, &fixed));
        let engine::Fixed { output, report, .. } = fixed;
        if self.write && output != text {
            found.pending.push((full, output));
        }
        found.absorb(shown, report.rewrites);
        Ok(())
    }

    /// The engine's rewrite of one file, less its own unmapped list: what
    /// is left is judged afresh, as `check` judges it (V80), and dropping
    /// the engine's list BEFORE that judging keeps the two from peaking
    /// together (R18).
    fn fixed(
        &self,
        judge: &Judge<'_>,
        text: &str,
        shown: &str,
    ) -> Result<engine::Fixed, String> {
        let mut fixed = judge
            .fix(text, &self.map)
            .map_err(|e| format!("{shown}: {e}"))?;
        fixed.report.unmapped = Vec::new();
        Ok(fixed)
    }
}

/// What is LEFT once the rewrite is done, judged exactly as `check`
/// judges it -- set, levels, `--strict` and hazards -- by `check`'s own
/// judge, then placed IN PLACE where it sits in the file on disk (V80,
/// B42). So `fix --check` and `check` cannot disagree about a character
/// `fix` leaves alone.
fn left(
    judge: &Judge<'_>,
    levels: &Levels,
    text: &str,
    fixed: &engine::Fixed,
) -> Vec<Finding> {
    let mut findings = match inspect(fixed.output.as_bytes(), judge, levels) {
        Looked::Findings(found) => found,
        Looked::Unread(_) => Vec::new(),
    };
    fixed.place(text, &mut findings, hit_of);
    findings
}

/// A finding's hit, for [`engine::Fixed::place`].
fn hit_of(finding: &mut Finding) -> &mut Hit {
    &mut finding.hit
}

impl Found {
    /// What one file left, as `check` would hold it: nothing for a clean
    /// file, else one row with its path and set once.
    fn keep(&mut self, path: &str, set: &str, mut findings: Vec<Finding>) {
        if findings.is_empty() {
            return;
        }
        findings.shrink_to_fit();
        self.unmapped.push(Row {
            path: path.to_owned(),
            set: set.to_owned(),
            findings,
        });
    }

    /// One file's rewrites, MOVED out of the engine's report.
    fn absorb(&mut self, path: String, rewrites: Vec<engine::Rewrite>) {
        if !rewrites.is_empty() {
            self.changed.push(Changed { path, rewrites });
        }
    }
}

/// A file's text, or `None` when it was skipped and named.
///
/// Classified by `check`'s own decode (`src/scan:V8`): a NUL anywhere is
/// binary, then UTF-8 is required. Named rather than dropped, the way
/// `check` names it: a file quietly missing from a rewrite run is a file
/// nobody knows was not rewritten, and a blob `check` skips but `fix`
/// rewrites is a file corrupted by the verb meant to clean it (B21).
fn text_of(
    text: Result<String, Unreadable>,
    shown: &str,
    found: &mut Found,
) -> Option<String> {
    match text {
        Ok(text) => Some(text),
        Err(reason) => {
            found.skips.push(Skip {
                path: shown.to_owned(),
                reason,
            });
            None
        }
    }
}

/// The report, and the code that goes with it.
///
/// `--check` GATES (V7): exit 1 on drift (the root spec's interface
/// section) or on whatever `check` would fail in what is left (V80). A
/// bare `fix` does not gate on what it just repaired: exit 1 only when
/// `check` would still fail what is LEFT, because then the tree is still
/// not clean. A finding at `warn` is reported and fails neither, as in
/// `check`, unless `--strict`.
/// A run that cleaned everything exits 0. The `ctrm-fix` pre-commit hook
/// still refuses the commit: pre-commit fails any hook that modified files.
///
/// Both forms exit 1 on a file that is not UTF-8, as `check` does
/// (`src/scan:V8`, B41): it claims to be text, so it was neither judged
/// nor rewritten, and an exit 0 would read as "clean".
fn report_of(found: &Found, format: Format, write: bool) -> Report {
    let changes: Vec<Change<'_>> =
        found.changed.iter().flat_map(changes).collect();
    let skipped: Vec<Skipped<'_>> = found.skips.iter().map(skip).collect();
    let kept: Vec<Batch<'_>> =
        found.unmapped.iter().flat_map(batches).collect();
    let drifted = !write && !found.changed.is_empty();
    let failed = drifted
        || found_code(&found.unmapped) != 0
        || unreadable_code(&found.skips) != 0;
    Report {
        text: render::fix(format, &changes, &kept, &skipped),
        code: u8::from(failed),
    }
}

/// One file's rewrites as the rows `render` borrows.
fn changes(file: &Changed) -> impl Iterator<Item = Change<'_>> {
    file.rewrites.iter().map(|rewrite| Change {
        path: &file.path,
        rewrite,
    })
}

#[cfg(test)]
#[path = "fix_test.rs"]
mod tests;
