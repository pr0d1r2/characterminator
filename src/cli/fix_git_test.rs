//! `fix --from-git` (`src/fix:V139`): a U+FFFD put back from the staged
//! copy or `HEAD`, and left alone when git cannot vouch for it. A child of
//! `fix_test.rs`; every test skips where git cannot run.

use crate::cli::fix::{Mode, run_with};
use crate::cli::testkit::fixture;
use crate::render::Format;
use std::path::{Path, PathBuf};
use std::process::Command;

const WAS: &str = "// hyphenated \u{A7}F node cannot be cited\n";
const LOST: &str = "// hyphenated \u{FFFD}F node cannot be cited\n";

/// A fresh repository whose `HEAD` holds `WAS` and whose work tree holds
/// `LOST`, or `None` when git is not there.
fn damaged(name: &str) -> Option<PathBuf> {
    let root = fixture(name, &[("notes.md", WAS)])?;
    let _gone = std::fs::remove_dir_all(root.join(".git"));
    let git = |args: &[&str]| {
        let done = Command::new("git").args(args).current_dir(&root).output();
        done.ok().filter(|out| out.status.success())
    };
    git(&["init", "-q"])?;
    git(&["add", "notes.md"])?;
    let who = ["-c", "user.name=t", "-c", "user.email=t@t"];
    git(&[who[0], who[1], who[2], who[3], "commit", "-qm", "x"])?;
    std::fs::write(root.join("notes.md"), LOST).ok()?;
    Some(root)
}

fn ran(root: &Path, mode: Mode) -> Result<(String, u8), String> {
    let asked = ["notes.md".to_owned()];
    let config = crate::cli::config::discovered(root);
    run_with(&config, &asked, Format::Human, mode).map(|r| (r.text, r.code))
}

/// What a bare `fix` is asked, with or without `--from-git`.
const fn writing(from_git: bool) -> Mode {
    Mode {
        write: true,
        from_git,
    }
}

fn read(root: &Path) -> String {
    std::fs::read_to_string(root.join("notes.md")).unwrap_or_default()
}

#[test]
fn from_git_puts_the_character_back_from_head() {
    let Some(root) = damaged("ctrm-fix-from-git-head") else {
        return;
    };
    let done = ran(&root, writing(true));
    assert!(done.is_ok(), "{done:?}");
    assert_eq!(read(&root), WAS);
}

#[test]
fn from_git_prefers_the_staged_copy() {
    let Some(root) = damaged("ctrm-fix-from-git-staged") else {
        return;
    };
    let staged = "// hyphenated \u{B6}F node cannot be cited\n";
    let wrote = std::fs::write(root.join("notes.md"), staged);
    let added = Command::new("git")
        .args(["add", "notes.md"])
        .current_dir(&root)
        .status();
    assert!(wrote.is_ok() && added.is_ok_and(|s| s.success()));
    assert!(std::fs::write(root.join("notes.md"), LOST).is_ok());
    assert!(ran(&root, writing(true)).is_ok());
    assert_eq!(read(&root), staged);
}

#[test]
fn without_the_flag_the_character_stays_and_is_named() {
    let Some(root) = damaged("ctrm-fix-no-from-git") else {
        return;
    };
    let (text, code) = ran(&root, writing(false)).unwrap_or_default();
    assert_eq!(read(&root), LOST);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("U+FFFD"), "{text}");
}

#[test]
fn a_changed_line_is_not_guessed_at() {
    let Some(root) = damaged("ctrm-fix-from-git-changed") else {
        return;
    };
    let edited = "// other \u{FFFD}F words entirely\n";
    assert!(std::fs::write(root.join("notes.md"), edited).is_ok());
    let (text, code) = ran(&root, writing(true)).unwrap_or_default();
    assert_eq!((read(&root).as_str(), code), (edited, 1), "{text}");
}

/// Restoring is a write: a gate that wrote nothing would read as clean.
#[test]
fn from_git_cannot_be_combined_with_check() {
    let Some(root) = damaged("ctrm-fix-from-git-check") else {
        return;
    };
    let refused = ran(
        &root,
        Mode {
            write: false,
            from_git: true,
        },
    )
    .err()
    .unwrap_or_default();
    assert!(refused.contains("--check"), "{refused}");
    assert_eq!(read(&root), LOST);
}
