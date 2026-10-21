//! `fix --from-git`: the earlier copies of a file that may still hold
//! what a lossy decode threw away (`src/fix:V139`).
//!
//! The matching is `src/fix`'s; this only fetches the text. The staged
//! copy is tried before `HEAD`, since an edit already staged is the
//! nearer truth. A file git does not know, or no git at all, is a file
//! with no earlier copy: nothing is restored and nothing is invented.

use crate::fix::restored;
use std::path::Path;
use std::process::Command;

/// `text` with each U+FFFD put back that the staged copy of `full`, then
/// `HEAD`'s, can vouch for.
pub(super) fn healed(full: &Path, text: &str) -> String {
    let staged = copy(full, ":");
    let head = copy(full, "HEAD:");
    let once =
        staged.map_or_else(|| text.to_owned(), |old| restored(text, &old));
    head.map_or(once.clone(), |old| restored(&once, &old))
}

/// The text git holds for `full` under `rev` (`:` is the index), or
/// `None` when it holds none or is not UTF-8.
fn copy(full: &Path, rev: &str) -> Option<String> {
    let (dir, name) = (full.parent()?, full.file_name()?.to_str()?);
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("show")
        .arg(format!("{rev}./{name}"))
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).ok())?
}
