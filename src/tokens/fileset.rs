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
use std::path::{Component, Path, PathBuf};

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
/// Two SPELLINGS of one path are one path too (V83): `sub/../a.md` and
/// `a.md` are folded [`lexical`]ly before they are compared (B45).
///
/// # Errors
/// When a named path is not readable, or is a directory holding nothing
/// git tracks.
pub fn select(root: &Path, paths: &[String]) -> Result<Vec<PathBuf>, Error> {
    rooted(root, paths.is_empty())?;
    if paths.is_empty() {
        return Ok(tracked(root));
    }
    let mut seen = BTreeSet::new();
    let mut chosen = Vec::new();
    for shown in paths {
        for full in named(root, shown)? {
            if seen.insert(lexical(&full)) {
                chosen.push(full);
            }
        }
    }
    Ok(chosen)
}

/// The run root can answer the question asked of it (V69), or the run is
/// an ERROR naming it. A root that is not a directory -- `-C` misspelt, or
/// pointed at a file -- and a bare run whose root is in no git work tree
/// both used to yield an empty set, and an empty set is a clean report:
/// exit 0 about files nobody looked at (B27).
fn rooted(root: &Path, bare: bool) -> Result<(), Error> {
    let refused = |reason: &str| Error {
        path: root.to_owned(),
        reason: reason.to_owned(),
    };
    if !root.is_dir() {
        return Err(refused(NOT_A_DIR));
    }
    if bare && !in_work_tree(root) {
        return Err(refused(NO_GIT));
    }
    Ok(())
}

const NOT_A_DIR: &str = "is not a directory";

const NO_GIT: &str = "is not inside a git work tree, so there is no \
                      tracked fileset -- name the files to check";

/// Whether `root` or a directory above it holds `.git` (a directory, or
/// the file a linked worktree or submodule has). Read off the filesystem,
/// lexically, so no git process is spawned to ask.
fn in_work_tree(root: &Path) -> bool {
    let full = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    full.ancestors().any(|dir| dir.join(".git").exists())
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
///
/// Both sides of the prefix test are folded [`lexical`]ly (V83): a raw
/// `d/e/../e` is no prefix of `d/e/x.md`, so the directory was refused
/// as holding nothing tracked (B45).
fn under(root: &Path, full: PathBuf) -> Result<Vec<PathBuf>, Error> {
    let dir = lexical(&full);
    let inside: Vec<PathBuf> = tracked(root)
        .into_iter()
        .filter(|path| lexical(path).starts_with(&dir))
        .collect();
    if inside.is_empty() {
        return Err(Error {
            path: full,
            reason: EMPTY.to_owned(),
        });
    }
    Ok(inside)
}

/// `path` with every `.` dropped and every `..` folded into the name
/// before it. A `..` with no name before it is kept: it leaves the tree.
///
/// Lexical, not canonical (`src/cli:V71`): a symlink is not resolved, so
/// a path stays the one typed. The crate's one folding, so the fileset
/// and the path a report shows cannot disagree about a spelling (V83).
#[must_use]
pub fn lexical(path: &Path) -> PathBuf {
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

#[cfg(test)]
#[path = "fileset_test.rs"]
mod tests;
