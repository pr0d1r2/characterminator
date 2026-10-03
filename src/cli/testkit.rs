//! The helpers every cli test reached for, written ONCE. Test-only, after
//! the `rules::corpus` precedent: six copies of a fixture writer are six
//! places for one of them to stop being untracked.

use std::path::{Path, PathBuf};

/// A tree carrying the files named, or `None` if it cannot be written: a
/// test should not fail for the disk's reasons.
///
/// It lives under `target/`, which `.gitignore` excludes, so it is
/// untracked by construction rather than by hoping, and each caller names
/// its own directory so two tests never share one tree. A verb is handed
/// the files BY NAME, so the git-tracked default never applies.
pub(crate) fn fixture(name: &str, files: &[(&str, &str)]) -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(name);
    std::fs::create_dir_all(&root).ok()?;
    for (file, text) in files {
        std::fs::write(root.join(file), text).ok()?;
    }
    Some(root)
}

/// Words as argv: verb first, program name not included.
pub(crate) fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}
