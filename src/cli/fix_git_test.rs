//! `src/fix:V106` through the verb: `fix --from-git` puts back a character a
//! lossy decode destroyed, from the staged copy or `HEAD`, and leaves the
//! `U+FFFD` it cannot restore where it is, reported.

use super::super::{Mode, run};
use crate::cli::testkit::fixture;
use crate::render::Format;
use std::path::Path;
use std::process::Command;

const COMMITTED: &str = "a\n// hyphenated \u{A7}F node\n";
const LOSSY: &str = "a\n// hyphenated \u{FFFD}F node\n";
const GRANT: &str = "* ascii\n*.md ascii+caveman\n";

fn git(root: &Path, args: &[&str]) -> bool {
    let ran = Command::new("git").arg("-C").arg(root).args(args).output();
    ran.is_ok_and(|out| out.status.success())
}

/// A repository holding `COMMITTED` as `notes.md`, whose working copy is
/// then `LOSSY`.
fn repository(name: &str) -> Option<std::path::PathBuf> {
    let root = fixture(name, &[(".ctrm", GRANT), ("notes.md", COMMITTED)])?;
    let _ = std::fs::remove_dir_all(root.join(".git"));
    let init = git(&root, &["init", "-q"]);
    let add = git(&root, &["add", "notes.md"]);
    let id = ["-c", "user.name=t", "-c", "user.email=t@t"];
    let commit = ["commit", "-q", "-m", "x"];
    let args: Vec<&str> = id.iter().chain(&commit).copied().collect();
    let made = git(&root, &args);
    std::fs::write(root.join("notes.md"), LOSSY).ok()?;
    (init && add && made).then_some(root)
}

fn go(root: &Path, mode: Mode) -> (String, u8) {
    let asked = ["notes.md".to_owned()];
    let config = crate::cli::config::discovered(root);
    match run(&config, &asked, Format::Human, mode) {
        Ok(report) => (report.text, report.code),
        Err(why) => (why, 9),
    }
}

fn read(root: &Path) -> String {
    std::fs::read_to_string(root.join("notes.md")).unwrap_or_default()
}

#[test]
fn a_lost_character_is_restored_from_head() {
    let Some(root) = repository("ctrm-fix-git-head") else {
        return;
    };
    let (text, code) = go(
        &root,
        Mode {
            write: true,
            from_git: true,
        },
    );
    assert_eq!((read(&root).as_str(), code), (COMMITTED, 0), "{text}");
    assert!(text.contains("notes.md:2:15"), "{text}");
}

#[test]
fn the_staged_copy_wins_over_head() {
    let Some(root) = repository("ctrm-fix-git-staged") else {
        return;
    };
    let staged = "a\n// hyphenated \u{A9}F node\n";
    let _ = std::fs::write(root.join("notes.md"), staged);
    let _ = git(&root, &["add", "notes.md"]);
    let _ = std::fs::write(root.join("notes.md"), LOSSY);
    let (text, _) = go(
        &root,
        Mode {
            write: true,
            from_git: true,
        },
    );
    assert!(read(&root).contains('\u{A9}'), "{text}");
}

#[test]
fn check_reports_the_restore_and_writes_nothing() {
    let Some(root) = repository("ctrm-fix-git-check") else {
        return;
    };
    let (text, code) = go(
        &root,
        Mode {
            write: false,
            from_git: true,
        },
    );
    assert_eq!((read(&root).as_str(), code), (LOSSY, 1), "{text}");
}

#[test]
fn without_the_flag_the_character_stays_lost_and_is_reported() {
    let Some(root) = repository("ctrm-fix-git-off") else {
        return;
    };
    let (text, code) = go(
        &root,
        Mode {
            write: true,
            from_git: false,
        },
    );
    assert_eq!((read(&root).as_str(), code), (LOSSY, 1), "{text}");
    assert!(text.contains("the original was lost on decoding"), "{text}");
}

#[test]
fn a_file_git_does_not_know_keeps_its_replacement_character() {
    let Some(root) = repository("ctrm-fix-git-unknown") else {
        return;
    };
    let _ = std::fs::write(root.join("other.md"), LOSSY);
    let asked = ["other.md".to_owned()];
    let mode = Mode {
        write: true,
        from_git: true,
    };
    let config = crate::cli::config::discovered(&root);
    let code = run(&config, &asked, Format::Human, mode).map(|r| r.code);
    assert_eq!(code, Ok(1));
}
