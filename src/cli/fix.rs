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

use super::check::{
    Row, Skip, batches, found_code, judged_against, skip, unreadable_code,
};
use super::remedy::Case;
use crate::fix::{self as engine, Map};
use crate::judge::{Checker, Config, File, Judge, files, judged};
use crate::lint::{Finding, Level, Lint, OUTSIDE_SET};
use crate::render::{self, Batch, Change, Format, Skipped};
use crate::scan::Unreadable;
use std::path::PathBuf;

/// What a run of `fix` produced.
pub(super) struct Report {
    pub text: String,
    pub code: u8,
    /// The tally for stderr (`src/render:V123`), empty when none.
    pub note: String,
}

/// One file's rewrites, owned so the borrowed render rows can point at
/// them, with the path held once for all of them.
struct Changed {
    path: String,
    set: String,
    rewrites: Vec<engine::Rewrite>,
    /// Parallel to `rewrites`: the verdict each character was judged at.
    verdicts: Vec<Verdict>,
}

impl Changed {
    fn new(path: String, set: &str) -> Self {
        Self {
            path,
            set: set.to_owned(),
            rewrites: Vec::new(),
            verdicts: Vec::new(),
        }
    }
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
        let case = Case::new(&judge, &levels, &text);
        let mut changed = Changed::new(shown, &set.name);
        let output = self.rewritten(&case, &mut changed, found)?;
        if self.write && output != text {
            found.pending.push((full, output));
        }
        found.absorb(changed);
        Ok(())
    }

    /// One text's rewrite: what is left is kept, the rewrites are MOVED
    /// into `changed` with the verdict each character was judged at, and
    /// the output is handed back for writing.
    fn rewritten(
        &self,
        case: &Case<'_>,
        changed: &mut Changed,
        found: &mut Found,
    ) -> Result<String, String> {
        let fixed = self.fixed(case.judge, case.text, &changed.path)?;
        found.keep(
            &changed.path,
            &changed.set,
            case.left_on_disk(&fixed, self.write),
        );
        changed.verdicts = verdicts(case, &fixed.report.rewrites);
        let engine::Fixed { output, report, .. } = fixed;
        changed.rewrites = report.rewrites;
        Ok(output)
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

/// The set, lint and level each rewritten character was judged at in the
/// ORIGINAL text, so a rewrite carries a violation's shape
/// (`src/render:V95`). A rewrite with no finding under it -- its lint set
/// to `allow`, which `check` never reports -- says so: `outside-set` at
/// `allow`.
fn verdicts(case: &Case<'_>, rewrites: &[engine::Rewrite]) -> Vec<Verdict> {
    let findings = judged(case.text, case.judge, case.levels);
    let at = |byte: usize| {
        findings
            .binary_search_by_key(&byte, |f| f.hit.position.byte)
            .ok()
            .and_then(|i| findings.get(i))
            .map_or((OUTSIDE_SET, Level::Allow), |f| (f.lint, f.level))
    };
    rewrites.iter().map(|r| at(r.hit.position.byte)).collect()
}

/// A rewritten character's lint and level.
type Verdict = (Lint, Level);

impl Found {
    /// What one file left, as `check` would hold it: nothing for a clean
    /// file, else one row with its path and set once.
    fn keep(&mut self, path: &str, set: &str, findings: Vec<Finding>) {
        if !findings.is_empty() {
            let row = Row::new(path.to_owned(), set, findings, Vec::new());
            self.unmapped.push(row);
        }
    }

    /// One file's rewrites, MOVED out of the engine's report.
    fn absorb(&mut self, changed: Changed) {
        if !changed.rewrites.is_empty() {
            self.changed.push(changed);
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
    let left = kept.iter().map(|batch| batch.findings.len()).sum();
    Report {
        text: render::fix(format, &changes, &kept, &skipped),
        code: code_of(found, write),
        note: render::fix_note(format, changes.len(), left, write),
    }
}

/// Exit 1 on drift under `--check`, on what `check` would fail in what
/// is left, or on a file that is not UTF-8; else 0.
fn code_of(found: &Found, write: bool) -> u8 {
    let drifted = !write && !found.changed.is_empty();
    let failed = drifted
        || found_code(&found.unmapped) != 0
        || unreadable_code(&found.skips) != 0;
    u8::from(failed)
}

/// One file's rewrites as the rows `render` borrows.
fn changes(file: &Changed) -> impl Iterator<Item = Change<'_>> {
    let verdict = |at: usize| file.verdicts.get(at).copied();
    file.rewrites.iter().enumerate().map(move |(at, rewrite)| {
        let (lint, level) = verdict(at).unwrap_or((OUTSIDE_SET, Level::Allow));
        let set = judged_against(lint, &file.set);
        let path = &file.path;
        Change {
            path,
            rewrite,
            set,
            lint,
            level,
        }
    })
}

#[cfg(test)]
#[path = "fix_test.rs"]
mod tests;
