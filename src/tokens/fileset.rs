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
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A named directory holding nothing git tracks (V43). An error rather
/// than an empty run, which would report a clean answer about no files.
const EMPTY: &str = "holds no git-tracked file -- name a file inside it, \
                     or track what is there";

/// The fileset for a run: the git-tracked files under `root`, or exactly
/// the paths named. Every path comes back joined to `root`, ready to read.
///
/// A named DIRECTORY expands to the tracked files under it (V43). Naming
/// a directory and a file inside it yields that file once: a path asked
/// for twice is still one file, and a count that added it twice would be
/// wrong rather than merely repetitive.
///
/// # Errors
/// When a named path is not readable, or is a directory holding nothing
/// git tracks.
pub fn select(root: &Path, paths: &[String]) -> Result<Vec<PathBuf>, Error> {
    if paths.is_empty() {
        return Ok(tracked(root));
    }
    let mut seen = BTreeSet::new();
    let mut chosen = Vec::new();
    for shown in paths {
        for full in named(root, shown)? {
            if seen.insert(full.clone()) {
                chosen.push(full);
            }
        }
    }
    Ok(chosen)
}

/// The git index, via itok. An empty answer where `root` is not a
/// repository is itok's, and is left alone: this crate has no second
/// opinion about what git tracks.
fn tracked(root: &Path) -> Vec<PathBuf> {
    itok::walk::tracked(root)
        .iter()
        .map(|f| root.join(f))
        .filter(|full| !is_link(full) && full.exists())
        .collect()
}

// A tracked path that is no longer on disk -- deleted, the deletion not
// yet staged -- leaves the set too (V70). It has no bytes to judge, and
// reading it anyway aborted the whole run with exit 2 (B25): one `rm`
// made every other file in the repository uncheckable.

/// A tracked SYMLINK is left out of the set (V48). Git stores the link's
/// TEXT, not what it points at, so reading through one scans something
/// else: a directory aborted the whole run (B4), and a file is counted a
/// second time, or from outside the repository altogether.
///
/// Only the tracked set is filtered. A link named on the command line is
/// followed, because a path you point at is a path you meant (V9).
fn is_link(full: &Path) -> bool {
    std::fs::symlink_metadata(full).is_ok_and(|m| m.file_type().is_symlink())
}

/// What one named path contributes: itself, or the tracked files under
/// it when it names a directory.
///
/// A path that cannot be read says so BY NAME -- a set silently short of
/// what was asked for is a total that understates without admitting it.
fn named(root: &Path, shown: &str) -> Result<Vec<PathBuf>, Error> {
    let full = root.join(shown);
    let meta = std::fs::metadata(&full).map_err(|e| Error {
        path: full.clone(),
        reason: e.to_string(),
    })?;
    if meta.is_dir() {
        return under(root, full);
    }
    Ok(vec![full])
}

/// The tracked files under a named directory (V43).
///
/// Filtered from the git-tracked set rather than walked: `ctrm check src/`
/// then answers about exactly the files a bare run would, and a build
/// directory inside the named one does not quietly join the set.
fn under(root: &Path, full: PathBuf) -> Result<Vec<PathBuf>, Error> {
    let inside: Vec<PathBuf> = tracked(root)
        .into_iter()
        .filter(|path| path.starts_with(&full))
        .collect();
    if inside.is_empty() {
        return Err(Error {
            path: full,
            reason: EMPTY.to_owned(),
        });
    }
    Ok(inside)
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

    /// V43: a named directory EXPANDS to the tracked files under it,
    /// which is the spelling everybody types.
    #[test]
    fn a_named_directory_expands_to_its_tracked_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        if !root.join(".git").exists() {
            return;
        }
        let set = select(root, &["src".to_owned()]).unwrap_or_default();
        assert!(set.iter().any(|p| p.ends_with("src/lib.rs")), "{set:?}");
        // Bounded by the directory named: the manifest sits above it.
        assert!(!set.iter().any(|p| p.ends_with("Cargo.toml")), "{set:?}");
    }

    /// A directory holding nothing tracked is an ERROR naming it, not an
    /// empty run -- which would report a clean answer about no files.
    #[test]
    fn a_directory_with_nothing_tracked_is_refused_with_its_reason() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        if std::fs::create_dir_all(root.join("target")).is_err() {
            return;
        }
        let err = select(root, &["target".to_owned()]).err();
        let reason = err.map(|e| e.reason).unwrap_or_default();
        assert!(reason.contains("no git-tracked file"), "{reason}");
    }

    /// B4: a link to a directory is what aborted a whole run, and a link
    /// to a file is what would count one file twice. Both are links; the
    /// file and the directory they point at are not.
    #[cfg(unix)]
    #[test]
    fn a_symlink_is_told_apart_from_what_it_points_at() {
        let Some(root) = links() else {
            return;
        };
        assert!(is_link(&root.join("to-dir")));
        assert!(is_link(&root.join("to-file")));
        assert!(!is_link(&root.join("real.md")));
        assert!(!is_link(&root.join("dir")));
        assert!(!is_link(&root.join("absent")));
    }

    /// A file, a directory, and a link to each. `None` if the disk refuses,
    /// for the reason `untracked` gives.
    #[cfg(unix)]
    fn links() -> Option<PathBuf> {
        use std::os::unix::fs::symlink;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("ctrm-symlink-fixture");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("dir"))
            .and_then(|()| std::fs::write(root.join("real.md"), "x"))
            .and_then(|()| symlink("dir", root.join("to-dir")))
            .and_then(|()| symlink("real.md", root.join("to-file")))
            .ok()?;
        Some(root)
    }

    /// B25: a tracked file deleted from the working tree leaves the set,
    /// rather than aborting the run when it cannot be read (V70).
    #[test]
    fn a_tracked_file_deleted_from_the_tree_is_left_out() {
        let Some(root) = deleted() else {
            return;
        };
        assert_eq!(select(&root, &[]), Ok(vec![root.join("kept.md")]));
    }

    /// A repository of its own under `target/`: two files tracked, then
    /// one deleted without staging the deletion. `None` without git.
    fn deleted() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("ctrm-deleted-fixture");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).ok()?;
        std::fs::write(root.join("kept.md"), "x").ok()?;
        std::fs::write(root.join("gone.md"), "x").ok()?;
        git(&root, &["init", "-q"])?;
        git(&root, &["add", "kept.md", "gone.md"])?;
        std::fs::remove_file(root.join("gone.md")).ok()?;
        Some(root)
    }

    /// `git` in `root`, scrubbed of the variables a hook exports, so the
    /// fixture's repository is the one it acts on.
    fn git(root: &Path, args: &[&str]) -> Option<()> {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .status()
            .ok()?
            .success()
            .then_some(())
    }

    /// A file named twice, once by itself and once inside a directory, is
    /// one file: a count that added it twice would be wrong.
    #[test]
    fn a_file_reached_two_ways_appears_once() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        if !root.join(".git").exists() {
            return;
        }
        let asked = ["src".to_owned(), "src/lib.rs".to_owned()];
        let set = select(root, &asked).unwrap_or_default();
        let hits = set.iter().filter(|p| p.ends_with("src/lib.rs")).count();
        assert_eq!(hits, 1, "{set:?}");
    }
}
