//! The tests of `adapter.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `adapter`.

use super::run;
use crate::render::json_string;
use std::path::{Path, PathBuf};

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
    let row = "trojan.rs:2:2 U+202E bidi-control -- 2 hazard";
    assert!(got.contains(row), "{got}");
    assert!(got.is_ascii(), "{got}");
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

/// The answer to a `Read` of one file holding raw `bytes`.
fn read_of_bytes(name: &str, ctrm: &str, bytes: &[u8]) -> String {
    let Some(root) = tree(name, &[(".ctrm", ctrm)]) else {
        return String::from("(disk refused)");
    };
    let full = root.join("f.rs");
    if std::fs::write(&full, bytes).is_err() {
        return String::from("(disk refused)");
    }
    answer_in(&read_payload(&root, &full.to_string_lossy()), &root)
}

/// B17: a file that is not text is still shown to the model, lossily
/// decoded, so a hazard in it denies the read (V66) -- after a stray
/// `\xff`, after a NUL, and under a `.ctrm` that cannot be applied.
#[test]
fn a_hazard_in_a_file_that_is_not_text_is_denied() {
    let utf8 = b"ok\nlet x = \"\xe2\x80\xae\";\n\xff\n";
    let got = read_of_bytes("ctrm-guard-notutf8", "", utf8);
    assert!(got.contains("f.rs:2:10 U+202E bidi-control"), "{got}");
    let nul = b"a\0\xe2\x80\xae";
    let broken = "*.rs nosuchset";
    for (name, ctrm) in [("ctrm-guard-nul", ""), ("ctrm-guard-bad", broken)] {
        let got = read_of_bytes(name, ctrm, nul);
        assert!(got.contains("f.rs:1:3 U+202E bidi-control"), "{got}");
    }
}

/// No hazard but the NUL and the bytes a binary is made of: the read
/// goes ahead, with a note that no rule could apply.
#[test]
fn a_binary_file_without_a_smuggling_hazard_passes_with_a_note() {
    let got =
        read_of_bytes("ctrm-guard-binary", "", b"\x89PNG\r\n\x1a\n\0\0\xff");
    assert!(got.contains("is not text"), "{got}");
    assert!(!got.contains("permissionDecision"), "{got}");
}

/// A SPARSE file: `head` at byte 0, `tail` at `at`, nothing (NUL, no
/// disk) between. Its read answer, under a `.ctrm` of `* ascii`.
fn read_of_sparse(name: &str, head: &str, at: u64, tail: &str) -> String {
    use std::io::{Seek, SeekFrom, Write};
    let Some(root) = tree(name, &[(".ctrm", "* ascii\n")]) else {
        return String::from("(disk refused)");
    };
    let full = root.join("big.log");
    let written = std::fs::File::create(&full).and_then(|mut f| {
        f.write_all(head.as_bytes())?;
        f.seek(SeekFrom::Start(at))?;
        f.write_all(tail.as_bytes())
    });
    if written.is_err() {
        return String::from("(disk refused)");
    }
    answer_in(&read_payload(&root, &full.to_string_lossy()), &root)
}

/// V102: past the 16 MiB cap only a prefix is judged. A hazard OUTSIDE
/// it is not found -- read whole, the NUL gap and the U+202E would deny
/// -- so the read passes, never in silence: the note says the rest was
/// not judged.
#[test]
fn a_file_over_the_cap_is_judged_by_its_prefix_and_says_so() {
    let past = super::CAP.saturating_add(10);
    let got =
        read_of_sparse("ctrm-guard-cap-tail", "plain\n", past, "\u{202E}\n");
    assert!(!got.contains("permissionDecision"), "{got}");
    assert!(got.contains("16 MiB cap"), "{got}");
    assert!(got.contains("the rest was NOT"), "{got}");
}

/// A hazard INSIDE the prefix still denies: the cap bounds memory, it
/// opens no door.
#[test]
fn a_hazard_in_the_prefix_of_a_capped_file_still_denies() {
    let past = super::CAP.saturating_add(10);
    let got =
        read_of_sparse("ctrm-guard-cap-head", "a\u{202E}b\n", past, "x\n");
    assert!(got.contains(r#""permissionDecision":"deny""#), "{got}");
    assert!(got.contains("big.log:1:2 U+202E"), "{got}");
}

/// The cut lands after the last newline inside the cap, else after the
/// last whole UTF-8 character, so it never splits one; a file within
/// the cap is read whole.
#[test]
fn the_prefix_is_cut_at_a_line_or_a_whole_character() {
    use super::cut;
    assert_eq!(cut(b"ab\ncd\nef", 7), 6);
    // U+2014 is 3 bytes at 1..4; a cap of 3 would split it.
    assert_eq!(cut("a\u{2014}b".as_bytes(), 3), 1);
    assert_eq!(cut(b"abcdef", 4), 4);
    let root = tree("ctrm-guard-cap-small", &[("s.md", "ab\ncd\n")]);
    let file = root.unwrap_or_default().join("s.md");
    let whole = super::loaded(&file, 6).map(|(b, w)| (b.len(), w));
    assert_eq!(whole, Some((6, true)));
    let capped = super::loaded(&file, 5).map(|(b, w)| (b.len(), w));
    assert_eq!(capped, Some((3, false)));
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

/// ASCII smuggling in a WebFetch result, sent the way a JSON encoder
/// may send it -- as escaped surrogate pairs -- and raw. Both decode
/// to the tag letters, and both block.
#[test]
fn tag_smuggling_in_a_web_fetch_result_is_tainted() {
    let first = "U+E0041 tag-character at tool_response.result line 1 \
                     column 6";
    for result in [
        r#"{"result":"Hello\udb40\udc41\udb40\udc42","code":200}"#,
        "{\"result\":\"Hello\u{E0041}\u{E0042}\",\"code\":200}",
    ] {
        let got = answer(&output_payload("WebFetch", result));
        let block = r#"{"decision":"block","reason":"#;
        assert!(got.starts_with(block), "{got}");
        assert!(got.contains(first), "{got}");
        assert!(got.contains("holds 2 hazard"), "{got}");
    }
}

/// `src/lint:V63`: the joiners of an RGI family and the tags of the
/// England flag are no hazard in tool output; a joiner between two
/// emoji that form no listed sequence, and tags after U+1F3F4 that
/// spell no listed subdivision, still block.
#[test]
fn only_a_listed_emoji_sequence_lets_its_joiners_and_tags_off() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    let england =
        "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";
    let ok = format!("{{\"result\":\"{family} {england}\"}}");
    assert_eq!(answer(&output_payload("WebFetch", &ok)), "");
    let smuggled = "\u{1F3F4}\u{E0069}\u{E0067}\u{E006E}\u{E007F}";
    for bad in ["\u{1F600}\u{200D}\u{1F600}", smuggled] {
        let body = format!("{{\"result\":\"{bad}\"}}");
        let got = answer(&output_payload("WebFetch", &body));
        assert!(got.starts_with(r#"{"decision":"block""#), "{got}");
    }
}

/// An MCP tool's result: the hazard is three levels down, and the
/// reason says exactly where.
#[test]
fn a_hazard_deep_in_a_nested_response_is_found_where_it_sits() {
    let response = r#"{"content":[{"type":"text","text":"ok"},
            {"type":"text","text":"line one\nsee \u202ehere"}],
            "isError":false}"#;
    let got = answer(&output_payload("mcp__docs__fetch", response));
    let at = "U+202E bidi-control at tool_response.content[1].text \
                  line 2 column 5";
    assert!(got.contains(at), "{got}");
    assert!(got.contains("The mcp__docs__fetch output"), "{got}");
}

/// Output is judged for hazards ONLY: an em dash in a web page is the
/// web's business, and a note on every page would be noise.
#[test]
fn clean_or_merely_non_ascii_output_passes_in_silence() {
    let bash = r#"{"stdout":"caf\u00e9 \u2014 ok","interrupted":false}"#;
    assert_eq!(answer(&output_payload("Bash", bash)), "");
    let plain = r#""PASS: 42 tests passed\n""#;
    assert_eq!(answer(&output_payload("Bash", plain)), "");
    let none = r#"{"hook_event_name":"PostToolUse","tool_name":"X"}"#;
    assert_eq!(answer(none), "");
}

/// B16: tool output has no file start, so a BOM opening a string of it
/// is a stray, not an encoding signature (V53).
#[test]
fn a_bom_opening_a_string_of_output_is_tainted() {
    for result in [r#"{"result":"\ufeffhi"}"#, "\"\u{FEFF}hi\""] {
        let got = answer(&output_payload("WebFetch", result));
        assert!(got.starts_with(r#"{"decision":"block""#), "{got}");
        assert!(got.contains("U+FEFF stray-bom"), "{got}");
    }
}

/// A hazard in a member NAME is still in front of the model.
#[test]
fn a_hazard_in_a_key_is_found() {
    let got = answer(&output_payload("WebSearch", r#"{"ti\u200btle":1}"#));
    let at = "U+200B invisible at tool_response.ti<U+200B>tle (key) \
                  line 1 column 3";
    assert!(got.contains(at), "{got}");
}

/// The path in a reason is spelled out too: a file NAMED with a
/// hazard must not put that hazard in front of the model.
#[test]
fn a_hazard_in_a_file_name_is_spelled_out() {
    let name = "x\u{202E}.rs";
    let got = read_of("ctrm-guard-name", &[(name, "\u{200B}\n")], name);
    assert!(got.contains("x<U+202E>.rs:1:1 U+200B invisible"), "{got}");
}

/// B18: an MCP result nested past the reader's depth bound used to be
/// an adapter error, exit 1, and so a pass. Its raw text is judged
/// instead, escapes decoded, pairs included (V67).
#[test]
fn a_hazard_too_deep_to_parse_is_still_tainted() {
    let pair = format!("{}udb40{}udc41", '\\', '\\');
    for hazard in ["\u{202E}", &pair] {
        let got = answer(&too_deep(&format!("\"x{hazard}\"")));
        assert!(got.starts_with(r#"{"decision":"block""#), "{got}");
    }
    let got = run(&too_deep("1"), Path::new("."));
    let why = got.err().unwrap_or_default();
    assert!(why.contains("nested too deep"), "{why}");
}

/// A PostToolUse payload with `inner` 300 arrays down.
fn too_deep(inner: &str) -> String {
    let (open, close) = ("[".repeat(300), "]".repeat(300));
    output_payload("mcp__x__y", &format!("{open}{inner}{close}"))
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
