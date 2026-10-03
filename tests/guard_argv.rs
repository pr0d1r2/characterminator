//! `guard` ignores argv (`src/cli/guard:V93`), pinned at the process
//! boundary: the only place argv exists. A unit test of the adapter never
//! sees a command line, so it could not catch a dispatch that started
//! parsing one -- and a parsed `--bogus` would be exit 2, which Claude Code
//! reads as "block" (`src/cli/guard:V53`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// What one `ctrm guard` run printed, and its exit code.
struct Answer {
    stdout: String,
    code: Option<i32>,
}

/// The built binary as a hook would start it, from `dir`.
fn spawned(dir: &Path, argv: &[&str]) -> Option<Child> {
    Command::new(env!("CARGO_BIN_EXE_ctrm"))
        .arg("guard")
        .args(argv)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

/// Run `ctrm guard <argv>` from `dir` with `stdin`. A process that does
/// not start answers with no code, which fails the caller's assertions.
fn guard(dir: &Path, argv: &[&str], stdin: &str) -> Answer {
    let mut child = spawned(dir, argv);
    if let Some(mut input) = child.as_mut().and_then(|c| c.stdin.take()) {
        let _ = input.write_all(stdin.as_bytes());
    }
    let out = child.and_then(|c| c.wait_with_output().ok());
    Answer {
        stdout: out.as_ref().map_or_else(String::new, |o| {
            String::from_utf8_lossy(&o.stdout).into_owned()
        }),
        code: out.and_then(|o| o.status.code()),
    }
}

/// A tree of its own under `target/`, or `None` when the disk refuses.
fn tree(name: &str, files: &[(&str, &str)]) -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(name);
    std::fs::create_dir_all(&root).ok()?;
    for (file, text) in files {
        std::fs::write(root.join(file), text).ok()?;
    }
    Some(root)
}

/// A flag no verb knows, a format guard does not take and a stray path:
/// any one of them stops every other verb with exit 2. Guard decides
/// anyway -- here a bidi override in fetched text, blocked, exit 0.
#[test]
fn guard_with_a_bogus_command_line_still_decides() {
    let payload = format!(
        "{{\"hook_event_name\":\"PostToolUse\",\"tool_name\":\"WebFetch\",\
         \"cwd\":\"/nowhere\",\"tool_response\":\"a{}b\"}}",
        '\u{202E}'
    );
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let argv = ["--bogus", "--format", "sarif", "stray.md"];
    let got = guard(here, &argv, &payload);
    assert_eq!(got.code, Some(0), "{}", got.stdout);
    let block = r#""decision":"block""#;
    assert!(got.stdout.contains(block), "{}", got.stdout);
}

/// The rules come from the dotfiles at the payload's `cwd`, never from
/// argv or the process directory: `-C`, `--rule '* any'` and a `* any`
/// beside the process would each silence the em dash note; none does.
#[test]
fn guard_reads_its_rules_from_the_payload_cwd_only() {
    let files = [(".ctrm", "* ascii\n"), ("a.md", "a\u{2014}b\n")];
    let rules = tree("ctrm-guard-argv-rules", &files).unwrap_or_default();
    let wide = tree("ctrm-guard-argv-wide", &[(".ctrm", "* any\n")]);
    let wide = wide.unwrap_or_default();
    let at = rules.to_string_lossy().replace('\\', "\\\\");
    let payload = format!(
        "{{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Read\",\
         \"cwd\":\"{at}\",\"tool_input\":{{\"file_path\":\"a.md\"}}}}"
    );
    let wide_dir = wide.to_string_lossy().into_owned();
    let argv = ["-C", wide_dir.as_str(), "--rule", "* any"];
    let got = guard(&wide, &argv, &payload);
    assert_eq!(got.code, Some(0), "{}", got.stdout);
    assert!(got.stdout.contains("outside-set: 1"), "{}", got.stdout);
}
