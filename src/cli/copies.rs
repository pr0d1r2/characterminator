//! The copies of a file git still holds, for `fix --from-git`
//! (`src/fix:V106`): what a lossy decode has not reached.
//!
//! Asked of the `git` binary, as the gate asks of its sibling tools. A file
//! git does not know, a tree that is no repository, or no `git` on `PATH`
//! each answer with no copy, and `fix` then leaves the `U+FFFD` it cannot
//! restore where it is, reported.

use std::path::Path;
use std::process::Command;

/// The staged copy of `full`, then its copy in `HEAD`, those that exist
/// and are text. The staged one comes first: it is the newer decision.
pub(super) fn of(full: &Path) -> Vec<String> {
    let (Some(dir), Some(name)) = (full.parent(), full.file_name()) else {
        return Vec::new();
    };
    let name = name.to_string_lossy();
    [format!(":./{name}"), format!("HEAD:./{name}")]
        .iter()
        .filter_map(|spec| show(dir, spec))
        .collect()
}

/// One object `git show` prints, when it prints one.
fn show(dir: &Path, spec: &str) -> Option<String> {
    let shown = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["show", spec])
        .output()
        .ok()?;
    shown
        .status
        .success()
        .then(|| String::from_utf8(shown.stdout).ok())
        .flatten()
}
