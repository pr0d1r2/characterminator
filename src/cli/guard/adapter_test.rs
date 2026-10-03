//! The tests of `adapter.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `adapter`.
//!
//! This file holds the payload fixtures, the `Read` of a text file,
//! and malformed input. Two subjects are child modules in files of
//! their own: a `Read` of bytes that are not plain text (`bytes`) and
//! tool output (`output`).

use super::run;
use crate::render::json_string;
use std::path::{Path, PathBuf};

#[path = "adapter_bytes_test.rs"]
mod bytes;
#[path = "adapter_output_test.rs"]
mod output;

/// A tree of its own under `target/`, untracked by construction, or
/// `None` when the disk refuses.
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

/// The PreToolUse payload Claude Code sends for a `Read`.
fn read_payload(cwd: &Path, file: &str) -> String {
    let cwd = json_string(&cwd.to_string_lossy());
    let path = json_string(file);
    format!(
        "{{\"session_id\":\"s\",\"transcript_path\":\"/t.jsonl\",\
             \"cwd\":{cwd},\"permission_mode\":\"default\",\
             \"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Read\",\
             \"tool_input\":{{\"file_path\":{path}}},\
             \"tool_use_id\":\"toolu_1\"}}"
    )
}

/// The PostToolUse payload, with `response` spliced in as raw JSON.
fn output_payload(tool: &str, response: &str) -> String {
    format!(
        "{{\"session_id\":\"s\",\"cwd\":\"/nowhere\",\
             \"hook_event_name\":\"PostToolUse\",\"tool_name\":\"{tool}\",\
             \"tool_input\":{{\"url\":\"https://example.com\"}},\
             \"tool_response\":{response}}}"
    )
}

/// What `guard` prints for one payload, which must be a payload.
fn answer(stdin: &str) -> String {
    answer_in(stdin, Path::new("."))
}

fn answer_in(stdin: &str, root: &Path) -> String {
    let got = run(stdin, root);
    assert!(got.is_ok(), "{got:?}");
    got.unwrap_or_default()
}

/// The answer to a `Read` of one file in a tree of its own. A disk
/// that refuses FAILS the caller's assertions rather than skipping
/// it: a guard test that silently stopped running would read as a
/// guard that passed.
fn read_of(name: &str, files: &[(&str, &str)], file: &str) -> String {
    let Some(root) = tree(name, files) else {
        return String::from("(disk refused)");
    };
    let full = root.join(file);
    answer_in(&read_payload(&root, &full.to_string_lossy()), &root)
}

#[test]
fn a_read_of_a_clean_file_passes_in_silence() {
    let got = read_of("ctrm-guard-clean", &[("a.md", "plain\n")], "a.md");
    assert_eq!(got, "");
}

/// Trojan Source: the read is DENIED, and the reason is the row
/// `check` would print -- path, line, column, code point, lint.
#[test]
fn a_read_of_a_bidi_control_is_denied_by_name() {
    let files = [("trojan.rs", "ok\nx\u{202E}y\u{2066}\n")];
    let got = read_of("ctrm-guard-bidi", &files, "trojan.rs");
    assert!(got.contains(r#""permissionDecision":"deny""#), "{got}");
    let row = "trojan.rs:2:2 U+202E bidi-control -- 2 bidi override or tag";
    assert!(got.contains(row), "{got}");
    assert!(got.is_ascii(), "{got}");
}

/// V111: a denial says how to get past it -- `fix` the file, then Read
/// again -- so the agent neither retries the same Read nor quits.
#[test]
fn a_denied_read_names_the_next_step() {
    let files = [("t.md", "a\u{E0041}\n")];
    let got = read_of("ctrm-guard-next-step", &files, "t.md");
    let step = "Read denied: run `ctrm fix ";
    assert!(got.contains(step), "{got}");
    assert!(
        got.contains("t.md` to remove them, then Read again."),
        "{got}"
    );
}

/// V110: a soft hyphen, terminal colour and a form feed are hazards
/// `check` forbids, and ordinary content: the read goes ahead, with a
/// note that says what they are and asks for no change.
#[test]
fn a_read_of_ordinary_hazards_passes_with_a_note() {
    let files = [("a.c", "Donau\u{00AD}dampf \u{1B}[1m\u{0C}\n")];
    let got = read_of("ctrm-guard-tier-note", &files, "a.c");
    assert!(!got.contains("permissionDecision"), "{got}");
    let note = "a.c:1:6 U+00AD invisible -- 3 invisible formatting or \
                control character(s)";
    assert!(got.contains(note), "{got}");
    assert!(got.contains("do not change the file unless asked"), "{got}");
}

/// An em dash is outside `ascii` and no hazard: the read goes ahead,
/// with a note naming the lint and the count (V35).
#[test]
fn a_read_of_an_em_dash_passes_with_a_note() {
    let files = [("a.md", "a\u{2014}b\u{2014}c\n")];
    let got = read_of("ctrm-guard-dash", &files, "a.md");
    assert!(got.contains(r#""additionalContext":"#), "{got}");
    assert!(got.contains("(outside-set: 2)"), "{got}");
    assert!(!got.contains("permissionDecision"), "{got}");
}

/// The tree's own `.ctrm` decides what a read is judged against.
#[test]
fn a_read_is_judged_against_the_rules_in_cwd() {
    let files = [(".ctrm", "*.md ascii+typography\n"), ("a.md", "-\n")];
    let got = read_of("ctrm-guard-granted", &files, "a.md");
    assert_eq!(got, "");
    let files = [(".ctrm", "*.md any\n"), ("a.md", "\u{2014}\n")];
    let got = read_of("ctrm-guard-granted-any", &files, "a.md");
    assert_eq!(got, "");
}

/// The note follows the level (V35): a finding at `warn` is noted like
/// one at `deny`, and one at `allow` is no finding, so the read passes in
/// silence.
#[test]
fn the_note_fires_at_warn_and_is_silent_at_allow() {
    let files = [(".ctrm", "* ascii !warn\n"), ("a.md", "a\u{2014}b\n")];
    let got = read_of("ctrm-guard-note-warn", &files, "a.md");
    assert!(got.contains("(outside-set: 1)"), "{got}");
    let files = [(".ctrm", "* ascii !allow\n"), ("a.md", "a\u{2014}b\n")];
    let got = read_of("ctrm-guard-note-allow", &files, "a.md");
    assert_eq!(got, "");
}

/// A `.ctrm` that cannot be applied leaves the hazards judged: they
/// are compiled in, and a typo must not open the door (V53).
#[test]
fn a_broken_rules_file_still_blocks_a_hazard() {
    let files = [(".ctrm", "*.md nosuchset\n"), ("a.md", "a\u{202E}\n")];
    let got = read_of("ctrm-guard-broken", &files, "a.md");
    assert!(got.contains("a.md:1:2 U+202E bidi-control"), "{got}");
    let files = [(".ctrm", "*.md nosuchset\n"), ("b.md", "fine\n")];
    let got = read_of("ctrm-guard-broken-clean", &files, "b.md");
    assert!(got.contains("judged for hazards only"), "{got}");
}

#[test]
fn a_read_of_a_missing_file_passes() {
    let got = read_of("ctrm-guard-missing", &[], "absent.md");
    assert_eq!(got, "");
}

/// No `cwd` in the payload: the process directory stands in, and a
/// relative path resolves against it.
#[test]
fn a_payload_without_cwd_resolves_against_the_root() {
    let files = [("t.rs", "\u{202E}\n")];
    let root = tree("ctrm-guard-nocwd", &files).unwrap_or_default();
    let stdin = r#"{"hook_event_name":"PreToolUse","tool_name":"Read",
            "tool_input":{"file_path":"t.rs"}}"#;
    let got = answer_in(stdin, &root);
    assert!(got.contains("t.rs:1:1 U+202E bidi-control"), "{got}");
}

/// Malformed input is a NAMED error, which the cli turns into exit 1
/// -- never a decision, and never a guess about which call it was.
#[test]
fn malformed_input_is_a_named_error() {
    for bad in ["", "{", "not json", r#"{"tool_name":"Read"}"#] {
        let why = run(bad, Path::new(".")).err().unwrap_or_default();
        assert!(why.starts_with("hook input"), "{bad:?} -> {why}");
    }
}
