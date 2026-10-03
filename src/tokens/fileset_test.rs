//! The tests of `fileset.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `fileset`.

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

/// B27: a root that is no directory, or a bare run outside any git
/// work tree, is an error naming the root -- never an empty, clean set.
#[test]
fn a_root_that_cannot_answer_is_refused_by_name() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    for root in [Path::new("/no/such/ctrm/root"), manifest.as_path()] {
        let said = select(root, &[]).err().map(|e| (e.path, e.reason));
        assert_eq!(said, Some((root.to_owned(), NOT_A_DIR.to_owned())));
    }
    let Some(bare) = outside_git() else {
        return;
    };
    let said = select(&bare, &[]).err().map(|e| e.reason);
    assert_eq!(said.as_deref(), Some(NO_GIT));
    // V9: a NAMED file is still reached without git.
    assert!(select(&bare, &["a.md".to_owned()]).is_ok());
}

/// A directory with a file in it and no `.git` above it, or `None`
/// when the temp dir happens to sit inside a work tree.
fn outside_git() -> Option<PathBuf> {
    let root = std::env::temp_dir().join("ctrm-outside-git-fixture");
    std::fs::create_dir_all(&root).ok()?;
    std::fs::write(root.join("a.md"), "x").ok()?;
    (!in_work_tree(&root)).then_some(root)
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

/// B45: two spellings of one file are one file (V83), not a file
/// judged twice.
#[test]
fn two_spellings_of_one_path_are_one_file() {
    let Some(root) = spelled("twice") else {
        return;
    };
    let asked = ["sub/../a.md".to_owned(), "a.md".to_owned()];
    assert_eq!(select(&root, &asked).map(|set| set.len()), Ok(1));
}

/// B45: a named directory spelled with `..` still expands (V83, V43),
/// rather than being refused as holding nothing tracked.
#[test]
fn a_directory_spelled_with_dot_dot_still_expands() {
    let Some(root) = spelled("dotdot") else {
        return;
    };
    let set = select(&root, &["d/e/../e".to_owned()]).unwrap_or_default();
    assert_eq!(set.len(), 1, "{set:?}");
    assert!(set.iter().all(|p| p.ends_with("x.md")), "{set:?}");
}

/// B46: a named link to a directory is followed (V84), as a named link
/// to a file is (V9): it expands to its target's tracked files, and
/// naming both is still one set.
#[cfg(unix)]
#[test]
fn a_named_link_to_a_directory_is_followed() {
    let Some(root) = spelled("dirlink") else {
        return;
    };
    if std::os::unix::fs::symlink("d", root.join("dl")).is_err() {
        return;
    }
    let want = Ok(vec![root.join("d/e/x.md")]);
    assert_eq!(select(&root, &["dl".to_owned()]), want);
    assert_eq!(select(&root, &["d".to_owned(), "dl/e".to_owned()]), want);
}

/// V84: a link to a directory outside the root says so by name, not
/// that the directory holds nothing.
#[cfg(unix)]
#[test]
fn a_named_link_leaving_the_root_says_so() {
    let Some(root) = spelled("outlink") else {
        return;
    };
    let away = root.with_file_name("ctrm-spelling-away");
    let made = std::fs::create_dir_all(&away)
        .and_then(|()| std::os::unix::fs::symlink(&away, root.join("out")));
    if made.is_err() {
        return;
    }
    let said = select(&root, &["out".to_owned()]).err().map(|e| e.reason);
    assert_eq!(said.as_deref(), Some(OUTSIDE));
}

/// A repository under `target/` tracking `a.md`, `sub/b.md` and
/// `d/e/x.md`, one per test so parallel tests never share one. `None`
/// without git.
fn spelled(name: &str) -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("ctrm-spelling-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("sub")).ok()?;
    std::fs::create_dir_all(root.join("d/e")).ok()?;
    for file in ["a.md", "sub/b.md", "d/e/x.md"] {
        std::fs::write(root.join(file), "x").ok()?;
    }
    git(&root, &["init", "-q"])?;
    git(&root, &["add", "."])?;
    Some(root)
}
