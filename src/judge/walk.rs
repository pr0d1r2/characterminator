//! The one walk every file verb takes: select, read, decode.
//!
//! `check`, `fix` and `stats` each selected the fileset, read each file,
//! decoded it and named a failure the same way, three times over. Here it
//! is once, so a file one verb skips is a file every verb skips, for the
//! same reason (`src/scan:V8`, B21), shown under the same path (V71).

use crate::scan::{Unreadable, decode_owned};
use crate::tokens::{self, lexical};
use std::path::{Path, PathBuf};

/// One file of a run, read and decoded once.
pub(crate) struct File {
    /// Where it is on disk, for a verb that writes it back.
    pub(crate) full: PathBuf,
    /// The path as a reader typed it: what rules match and reports name.
    pub(crate) shown: String,
    /// Its text, or why it has none.
    pub(crate) text: Result<String, Unreadable>,
}

/// A bare run over a work tree with nothing tracked (V118). A run that
/// judged no file printed nothing and exited 0, which is the clean verdict
/// -- about files nobody looked at (B69).
pub(crate) const NOTHING_TRACKED: &str =
    "no git-tracked files to check -- `git add` them or name paths";

/// Every file `paths` names under `root` -- the git-tracked fileset when
/// it names none (`src/tokens:V9`) -- in order, each read when reached.
///
/// # Errors
///
/// A path the fileset refuses, or a bare run with nothing tracked (V118),
/// up front; then, per file, one that cannot be read, named with its path.
pub(crate) fn files(
    root: &Path,
    paths: &[String],
) -> Result<impl Iterator<Item = Result<File, String>>, String> {
    let selected = tokens::select(root, paths).map_err(|bad| {
        let shown = shown_path(root, &bad.path);
        format!("{shown}: {}", plain(&bad.reason))
    })?;
    if paths.is_empty() && selected.is_empty() {
        return Err(NOTHING_TRACKED.to_owned());
    }
    let root = root.to_owned();
    Ok(selected.into_iter().map(move |full| read(&root, full)))
}

/// One file, read and decoded.
fn read(root: &Path, full: PathBuf) -> Result<File, String> {
    let bytes =
        std::fs::read(&full).map_err(|e| format!("{}: {e}", full.display()))?;
    Ok(File {
        shown: shown_path(root, &full),
        text: decode_owned(bytes),
        full,
    })
}

/// An I/O reason without the OS code: `No such file or directory (os
/// error 2)` reads as `No such file or directory` (`src/cli/usage:V121`).
/// The number names nothing a reader can act on.
#[must_use]
pub(crate) fn plain(reason: &str) -> &str {
    reason
        .rfind(" (os error ")
        .and_then(|at| reason.get(..at))
        .unwrap_or(reason)
}

/// The path as a reader typed it: relative to the root, so it matches the
/// patterns in `.ctrm` and reads like the file they meant.
///
/// Both sides are folded LEXICALLY first (V71): `sub/../sub/c.md` names
/// `sub/c.md`, and a rule anchored at `sub/c.md` has to see that spelling
/// or it silently judges the file by another rule. Lexical, not
/// canonical: a symlink is not resolved, so the path stays the one typed.
pub(crate) fn shown_path(root: &Path, full: &Path) -> String {
    let (root, full) = (lexical(root), lexical(full));
    full.strip_prefix(&root)
        .unwrap_or(&full)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::{NOTHING_TRACKED, files, shown_path};
    use std::path::Path;

    /// `src/cli/usage:V121`: a missing named file is shown as typed,
    /// relative to the root, without the OS error code.
    #[test]
    fn a_missing_file_is_named_as_typed() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let asked = [String::from("no-such.md")];
        let said = files(root, &asked).err().unwrap_or_default();
        assert_eq!(said, "no-such.md: No such file or directory");
    }

    /// B69 / V118: a bare run in a work tree tracking nothing is refused,
    /// not answered with the silence of a clean run. Skipped without git.
    #[test]
    fn a_bare_run_with_nothing_tracked_is_refused() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("ctrm-nothing-tracked-fixture");
        let made = std::fs::create_dir_all(&root).and_then(|()| {
            std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(&root)
                .status()
        });
        if !made.is_ok_and(|status| status.success()) {
            return;
        }
        let refused = files(&root, &[]).err();
        assert_eq!(refused.as_deref(), Some(NOTHING_TRACKED));
    }

    #[test]
    fn a_path_is_shown_relative_to_the_root() {
        let root = Path::new("/repo");
        assert_eq!(shown_path(root, Path::new("/repo/src/a.rs")), "src/a.rs");
        assert_eq!(
            shown_path(root, Path::new("/elsewhere/a.rs")),
            "/elsewhere/a.rs"
        );
    }

    /// V71: `.` and `..` fold away before a path is matched or shown.
    #[test]
    fn a_path_is_shown_in_lexical_normal_form() {
        let root = Path::new("/repo/./x/..");
        let shown = |full: &str| shown_path(root, Path::new(full));
        assert_eq!(shown("/repo/sub/../sub/c.md"), "sub/c.md");
        assert_eq!(shown("/repo/./a.md"), "a.md");
        assert_eq!(shown("/repo/../repo/a.md"), "a.md");
        assert_eq!(shown("/elsewhere/../b/a.rs"), "/b/a.rs");
    }
}
