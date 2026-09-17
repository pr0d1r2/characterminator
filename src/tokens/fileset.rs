//! Which files get counted (V9).
//!
//! The default set is what git TRACKS, read from the index by itok's own
//! walk rather than by crawling the filesystem: the tool should cost what
//! the repository costs, and a build directory is not part of that.
//!
//! Naming paths overrides the default, and a named file is counted
//! whether or not git tracks it. A file you point at is a file you meant
//! -- refusing it because it is untracked would make the tool unusable on
//! the thing you just wrote.

use crate::tokens::Error;
use std::path::{Path, PathBuf};

/// A directory is a category error, not a small file: its inode size is
/// not text, and counting it would report a number nothing wrote.
const DIRECTORY: &str = "is a directory -- name files, or name no path \
                         at all for the git-tracked set";

/// The fileset for a run: the git-tracked files under `root`, or exactly
/// the paths named. Every path comes back joined to `root`, ready to read.
///
/// # Errors
/// When a named path is not a readable file.
pub fn select(root: &Path, paths: &[String]) -> Result<Vec<PathBuf>, Error> {
    if paths.is_empty() {
        return Ok(tracked(root));
    }
    paths.iter().map(|p| named(root, p)).collect()
}

/// The git index, via itok. An empty answer where `root` is not a
/// repository is itok's, and is left alone: this crate has no second
/// opinion about what git tracks.
fn tracked(root: &Path) -> Vec<PathBuf> {
    itok::walk::tracked(root)
        .iter()
        .map(|f| root.join(f))
        .collect()
}

/// A named path must be a readable file, and says so by name when it is
/// not -- a set silently short of what was asked for is a total that
/// understates without admitting it.
fn named(root: &Path, shown: &str) -> Result<PathBuf, Error> {
    let full = root.join(shown);
    let meta = std::fs::metadata(&full).map_err(|e| Error {
        path: full.clone(),
        reason: e.to_string(),
    })?;
    if meta.is_dir() {
        return Err(Error {
            path: full,
            reason: DIRECTORY.to_owned(),
        });
    }
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V9: with no paths named, the set is what git tracks -- this
    /// crate's own manifest is in it.
    #[test]
    fn the_default_set_is_the_tracked_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        if !root.join(".git").exists() {
            return;
        }
        let set = select(root, &[]).unwrap_or_default();
        assert!(set.iter().any(|p| p.ends_with("Cargo.toml")), "{set:?}");
    }

    /// V9's other half: a named path reaches a file git does NOT track.
    #[test]
    fn a_named_path_reaches_an_untracked_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Some(shown) = untracked(root) else {
            return;
        };
        let picked = select(root, std::slice::from_ref(&shown));
        assert_eq!(picked, Ok(vec![root.join(&shown)]));
        let _ = std::fs::remove_file(root.join(&shown));
    }

    /// A file under `target/`, which `.gitignore` excludes: untracked by
    /// construction rather than by hoping. `None` if the fixture cannot
    /// be written, since a test should not fail for the disk's reasons.
    fn untracked(root: &Path) -> Option<String> {
        let name = "ctrm-untracked-fixture.txt";
        let dir = root.join("target");
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(dir.join(name), "x").ok()?;
        Some(format!("target/{name}"))
    }

    /// A named path that cannot be read is an error naming it, never a
    /// row quietly missing from the set.
    #[test]
    fn a_missing_named_path_is_an_error_naming_it() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let err = select(root, &["no/such/file".to_owned()]).err();
        assert_eq!(err.map(|e| e.path), Some(root.join("no/such/file")));
    }

    /// A directory is refused WITH its reason, so the fix is readable
    /// from the message.
    #[test]
    fn a_named_directory_is_refused_with_its_reason() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let err = select(root, &["src".to_owned()]).err();
        let reason = err.map(|e| e.reason).unwrap_or_default();
        assert!(reason.contains("directory"), "{reason}");
    }
}
