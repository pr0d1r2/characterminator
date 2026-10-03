//! The binary end to end, in a fixture repository under the temp dir:
//! root discovery from the working directory, the recorded-answer
//! environment, and a missing tool reported as one (`dev:V137`).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FILES: [(&str, &str); 6] = [
    (".git/HEAD", "ref: refs/heads/main\n"),
    ("SPEC.md", "# SPEC\n"),
    ("Cargo.toml", "[package]\nname = \"demo\"\n"),
    (".coverage", "lines 90.0\n"),
    (
        "docs/THIRD-PARTY-NOTICES.md",
        "<!-- BEGIN closure -->\n<!-- END closure -->\n",
    ),
    ("tree.txt", "demo v0.1.0 (/x)|MIT\nmemchr v2.8.3|MIT\n"),
];

fn fixture(name: &str) -> PathBuf {
    let id = std::process::id();
    let root = std::env::temp_dir().join(format!("ctrm-dev-cli-{id}-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    for (file, text) in FILES {
        let path = root.join(file);
        let dir = path.parent().map(PathBuf::from).unwrap_or_default();
        assert!(std::fs::create_dir_all(dir).is_ok());
        assert!(std::fs::write(path, text).is_ok());
    }
    root
}

fn dev(root: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_characterminator-dev"));
    cmd.current_dir(root.join("docs"));
    cmd
}

/// The exit code and stderr, or `(None, "")` when it could not run.
fn result(out: std::io::Result<Output>) -> (Option<i32>, String) {
    out.map(|o| {
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        (o.status.code(), err)
    })
    .unwrap_or_default()
}

#[test]
fn the_binary_finds_the_root_above_its_working_directory() {
    let root = fixture("root");
    let tree = root.join("tree.txt");
    let out = dev(&root)
        .args(["notices"])
        .env("CTRM_DEV_CARGO_TREE", tree)
        .output();
    assert_eq!(result(out), (Some(0), String::new()));
    let doc = std::fs::read_to_string(root.join("docs/THIRD-PARTY-NOTICES.md"));
    assert!(
        doc.unwrap_or_default()
            .contains("| `memchr` | 2.8.3 | MIT |")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_cargo_that_cannot_spawn_is_a_missing_tool() {
    let root = fixture("tool");
    let out = dev(&root)
        .args(["notices", "--check"])
        .env_remove("CTRM_DEV_CARGO_TREE")
        .env("CARGO", root.join("no-such-cargo"))
        .output();
    let (code, err) = result(out);
    assert_eq!(code, Some(1));
    assert!(err.contains("a MISSING TOOL, not a finding"), "{err}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn outside_any_repository_is_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_characterminator-dev"))
        .arg("readme")
        .current_dir(std::env::temp_dir())
        .output();
    assert_eq!(result(out).0, Some(2));
}
