//! PostToolUse: tool output is judged for hazards only (V35), down to
//! a key, a file name and past the parser's depth bound (V67, B16,
//! B18). A child of `adapter_test.rs`, reaching its fixtures as `super`.

use super::{answer, output_payload, read_of, run};
use std::path::Path;

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

/// `src/lint/hazard:V63`: the joiners of an RGI family and the tags of the
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
