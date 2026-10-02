//! The `check` verb (T8): report every character outside its file's set.
//!
//! Composition only. Every answer comes from the node that owns it: which
//! rule governs a path (`src/rules`), what that rule's sets contain
//! (`src/charset`), where a character sits (`src/scan`), how loudly it is
//! said (`src/lint`), and how the report reads (`src/render`). V7 makes
//! this one of the two verbs that GATE. What a file's bytes are judged to
//! hold is the shared engine's answer (`checker.rs`); this file walks the
//! tree, gathers those answers, and reports them.

use super::checker::{Checker, Looked, inspect};
use super::config::Config;
use crate::charset::CharSet;
use crate::lint::{Finding, Group, exit_code};
use crate::render::{self, Format, Skipped, Violation};
use crate::scan::Unreadable;
use crate::tokens;
use std::path::{Component, Path, PathBuf};

/// A violation with the strings it is reported against, owned so the
/// borrowed `render` rows can point at them.
struct Row {
    path: String,
    set: String,
    finding: Finding,
}

/// A file that was skipped, owned for the same reason.
struct Skip {
    path: String,
    reason: Unreadable,
}

/// A finished report: what to print, and what to exit with.
pub struct Report {
    pub text: String,
    pub code: u8,
}

/// The path as a reader typed it: relative to the root, so it matches the
/// patterns in `.ctrm` and reads like the file they meant.
///
/// Both sides are folded LEXICALLY first (V71): `sub/../sub/c.md` names
/// `sub/c.md`, and a rule anchored at `sub/c.md` has to see that spelling
/// or it silently judges the file by another rule. Lexical, not
/// canonical: a symlink is not resolved, so the path stays the one typed.
pub(super) fn shown_path(root: &Path, full: &Path) -> String {
    let (root, full) = (lexical(root), lexical(full));
    full.strip_prefix(&root)
        .unwrap_or(&full)
        .to_string_lossy()
        .into_owned()
}

/// `path` with every `.` dropped and every `..` folded into the name
/// before it. A `..` with no name before it is kept: it leaves the tree.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        let named =
            matches!(out.components().next_back(), Some(Component::Normal(_)));
        match part {
            Component::CurDir => {}
            Component::ParentDir if named => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

fn gather(
    checker: &Checker,
    root: &Path,
    paths: &[String],
) -> Result<Found, String> {
    let files = tokens::select(root, paths)
        .map_err(|e| format!("{}: {}", e.path.display(), e.reason))?;
    let mut found = Found::default();
    for full in &files {
        let shown = shown_path(root, full);
        let (set, levels) = checker.law(&shown)?;
        let looked = inspect(&read(full)?, &checker.judge(&set), &levels);
        found.absorb(shown, &set, looked);
    }
    Ok(found)
}

fn read(full: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(full).map_err(|e| format!("{}: {e}", full.display()))
}

/// What the walk accumulated.
#[derive(Default)]
struct Found {
    rows: Vec<Row>,
    skips: Vec<Skip>,
}

impl Found {
    fn absorb(&mut self, path: String, set: &CharSet, looked: Looked) {
        match looked {
            Looked::Unread(reason) => self.skips.push(Skip { path, reason }),
            Looked::Findings(findings) => {
                for finding in findings {
                    self.rows.push(Row {
                        path: path.clone(),
                        set: judged_against(&finding, set),
                        finding,
                    });
                }
            }
        }
    }
}

/// The set a finding names in the report.
///
/// A hazard names `hazard`, not the file's set: the set did not decide it
/// (V34), and `notes.md:1:1 U+202E any` would read as though the override
/// fell outside `any`, which is nonsense. Which hazard it was is the
/// lint's name, carried in the json.
fn judged_against(finding: &Finding, set: &CharSet) -> String {
    if finding.lint.group == Group::Hazard {
        String::from(Group::Hazard.name())
    } else {
        set.name.clone()
    }
}

/// Run `check` over a repository.
///
/// `paths` empty means the git-tracked fileset; naming paths reaches
/// untracked files too (`src/tokens:V9`).
pub fn run(
    config: &Config,
    paths: &[String],
    format: Format,
) -> Result<Report, String> {
    let checker = Checker::configured(config)?;
    let found = gather(&checker, &config.root, paths)?;
    let violations: Vec<Violation<'_>> = found.rows.iter().map(row).collect();
    let skipped: Vec<Skipped<'_>> = found.skips.iter().map(skip).collect();
    let findings: Vec<Finding> =
        found.rows.iter().map(|r| r.finding.clone()).collect();
    Ok(Report {
        text: render::check(format, &violations, &skipped),
        code: exit_code(&findings).max(unreadable_code(&found.skips)),
    })
}

/// Invalid UTF-8 is an ERROR, exit 1 (`src/scan:V8`): the file claims to
/// be text and is not, so no verdict about its characters was reached
/// (B22). A binary skip is not: the file never claimed to be text, and
/// failing on every tracked PNG would make the gate unusable. One code for
/// every format, since json and SARIF report the same run.
fn unreadable_code(skips: &[Skip]) -> u8 {
    let broken =
        |skip: &Skip| matches!(skip.reason, Unreadable::NotUtf8 { .. });
    u8::from(skips.iter().any(broken))
}

fn row(source: &Row) -> Violation<'_> {
    Violation {
        path: &source.path,
        finding: source.finding.clone(),
        set: &source.set,
    }
}

fn skip(source: &Skip) -> Skipped<'_> {
    Skipped {
        path: &source.path,
        reason: source.reason,
    }
}

#[cfg(test)]
#[path = "check_test.rs"]
mod tests;
