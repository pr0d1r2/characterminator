//! `fix --from-git` at the process boundary: the word on the command line
//! reaches the verb (`src/fix:V106`). What the verb then does is
//! `src/cli/fix_git_test.rs`'s.

use std::path::{Path, PathBuf};
use std::process::Command;

const COMMITTED: &str = "// hyphenated \u{A7}F node\n";
const LOSSY: &str = "// hyphenated \u{FFFD}F node\n";

fn git(root: &Path, args: &[&str]) -> bool {
    let ran = Command::new("git").arg("-C").arg(root).args(args).output();
    ran.is_ok_and(|out| out.status.success())
}

/// A repository holding `COMMITTED`, whose working copy is then `LOSSY`.
fn repository() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ctrm-from-git");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).ok()?;
    std::fs::write(root.join(".ctrm"), "* ascii+caveman\n").ok()?;
    std::fs::write(root.join("notes.md"), COMMITTED).ok()?;
    let id = ["-c", "user.name=t", "-c", "user.email=t@t"];
    let commit = ["commit", "-q", "-m", "x"];
    let args: Vec<&str> = id.iter().chain(&commit).copied().collect();
    let made = git(&root, &["init", "-q"])
        && git(&root, &["add", "notes.md"])
        && git(&root, &args);
    std::fs::write(root.join("notes.md"), LOSSY).ok()?;
    made.then_some(root)
}

#[test]
fn the_flag_puts_the_character_back() {
    let Some(root) = repository() else {
        return;
    };
    let mut ctrm = Command::new(env!("CARGO_BIN_EXE_ctrm"));
    let ran = ctrm
        .args(["fix", "--from-git", "notes.md"])
        .current_dir(&root);
    let code = ran.output().ok().and_then(|o| o.status.code());
    let text = std::fs::read_to_string(root.join("notes.md"));
    assert_eq!(code, Some(0));
    assert_eq!(text.unwrap_or_default(), COMMITTED);
}
