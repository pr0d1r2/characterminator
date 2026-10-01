//! The harness mapping, and the ONLY file that knows one: Claude Code's
//! hook payloads in, its decision documents out.
//!
//! itok's `guard` has the same shape and the same rule: everything about
//! a harness lives in one file, so a second harness is a second mapping
//! rather than a fork of the verb, and `guard` itself only ever sees a
//! [`Call`] and answers with a [`Verdict`].
//!
//! The contract, as the Claude Code hooks reference documents it
//! (code.claude.com/docs/en/hooks, read 2026-10-01):
//!
//! ```text
//! stdin   {"cwd","hook_event_name":"PreToolUse","tool_name",
//!          "tool_input":{"file_path",...}, ...}
//!         {"cwd","hook_event_name":"PostToolUse","tool_name",
//!          "tool_input":{...},"tool_response":<any JSON>, ...}
//! deny    {"hookSpecificOutput":{"hookEventName":"PreToolUse",
//!          "permissionDecision":"deny","permissionDecisionReason":"..."}}
//! block   {"decision":"block","reason":"..."}       (PostToolUse)
//! note    {"hookSpecificOutput":{"hookEventName":"...",
//!          "additionalContext":"..."}}
//! ```
//!
//! A PostToolUse `block` cannot un-run the tool. What it does is put the
//! reason in front of the model beside the output, which is the point:
//! the model is told the content is tainted before it acts on it.
//!
//! Unknown fields are IGNORED, so a harness adding one breaks nothing. A
//! payload with no `hook_event_name` is refused, because without it the
//! answer's shape is unknowable; an event this file does not map is a
//! silent pass, because the harness's matcher, not this file, decides
//! which calls arrive.

use super::json::{self, Value};
use crate::render::json_string;

/// What `guard` is asked about, with the harness spelled out of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Call {
    /// Before a tool runs on a file: judge the file first. Any tool naming
    /// `tool_input.file_path` qualifies; `Read` is the one a settings
    /// matcher normally sends.
    Read { cwd: Option<String>, path: String },
    /// After a tool ran: every string in its output, each labelled with
    /// where it sat, because the output's shape varies by tool and a
    /// hazard is a hazard in any field of it.
    Output {
        tool: String,
        texts: Vec<(String, String)>,
    },
    /// An event, or a call, with nothing to judge.
    Other,
}

/// Which hook a verdict answers. The two spell a block differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Event {
    Before,
    After,
}

impl Event {
    const fn name(self) -> &'static str {
        match self {
            Self::Before => "PreToolUse",
            Self::After => "PostToolUse",
        }
    }
}

/// What `guard` decided (V35).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Nothing to say: the harness carries on, and stdout stays empty.
    Pass,
    /// Carry on, with a note for the model: findings that are not hazards.
    Note(String),
    /// A hazard: deny the read, or flag the output as tainted.
    Block(String),
}

/// Map one payload to a call.
///
/// # Errors
///
/// Input that is not JSON, or JSON that names no hook event.
pub(super) fn call(stdin: &str) -> Result<Call, String> {
    let payload = json::parse(stdin)
        .map_err(|why| format!("hook input is not JSON: {why}"))?;
    let event = payload.get("hook_event_name").and_then(Value::text);
    match event {
        Some("PreToolUse") => Ok(before(&payload)),
        Some("PostToolUse") => Ok(after(&payload)),
        Some(_) => Ok(Call::Other),
        None => Err(String::from("hook input names no `hook_event_name`")),
    }
}

/// A call that names no file -- `Bash`, say -- has nothing to read, so
/// nothing to judge before it runs.
fn before(payload: &Value) -> Call {
    let input = payload.get("tool_input");
    let path = input.and_then(|i| i.get("file_path")).and_then(Value::text);
    path.map_or(Call::Other, |path| Call::Read {
        cwd: text_of(payload, "cwd"),
        path: path.to_owned(),
    })
}

fn after(payload: &Value) -> Call {
    let response = payload.get("tool_response");
    let texts = response.map(|r| r.strings("tool_response"));
    Call::Output {
        tool: text_of(payload, "tool_name").unwrap_or_default(),
        texts: texts
            .unwrap_or_default()
            .into_iter()
            .map(|(at, text)| (at, text.to_owned()))
            .collect(),
    }
}

fn text_of(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::text).map(str::to_owned)
}

/// The decision document, or the empty string for a silent pass.
///
/// One line of pure ASCII: every reason is a json string literal from
/// `src/render`, so a path holding a quote, or a hazard named in a reason,
/// cannot make a document no parser accepts.
pub(super) fn response(event: Event, verdict: &Verdict) -> String {
    match (event, verdict) {
        (_, Verdict::Pass) => String::new(),
        (Event::Before, Verdict::Block(why)) => denial(why),
        (Event::After, Verdict::Block(why)) => {
            format!(
                "{{\"decision\":\"block\",\"reason\":{}}}",
                json_string(why)
            )
        }
        (_, Verdict::Note(note)) => context(event, note),
    }
}

/// A PreToolUse deny: the read does not happen, and the model is told why.
fn denial(why: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":{}}}}}",
        json_string(why)
    )
}

/// A pass that carries a note. No `permissionDecision` is given, so the
/// harness's own permission flow runs as it would without the hook.
fn context(event: Event, note: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"{}\",\
         \"additionalContext\":{}}}}}",
        event.name(),
        json_string(note)
    )
}

#[cfg(test)]
mod tests {
    use super::{Call, Event, Verdict, call, response};

    fn mapped(stdin: &str) -> Call {
        let got = call(stdin);
        assert!(got.is_ok(), "{got:?}");
        got.unwrap_or(Call::Other)
    }

    #[test]
    fn a_read_maps_to_its_file_and_directory() {
        let got = mapped(
            r#"{"session_id":"s","cwd":"/repo","hook_event_name":"PreToolUse",
            "tool_name":"Read","tool_input":{"file_path":"/repo/a.md"}}"#,
        );
        let want = Call::Read {
            cwd: Some(String::from("/repo")),
            path: String::from("/repo/a.md"),
        };
        assert_eq!(got, want);
    }

    #[test]
    fn a_call_naming_no_file_has_nothing_to_judge() {
        let bash = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash",
            "tool_input":{"command":"ls"}}"#;
        assert_eq!(mapped(bash), Call::Other);
    }

    #[test]
    fn an_unmapped_event_passes() {
        let stop = r#"{"hook_event_name":"Stop","cwd":"/repo"}"#;
        assert_eq!(mapped(stop), Call::Other);
    }

    #[test]
    fn output_maps_to_every_string_in_the_response() {
        let got = mapped(
            r#"{"hook_event_name":"PostToolUse","tool_name":"Bash",
            "tool_response":{"stdout":"ok","interrupted":false}}"#,
        );
        let pair = |at: &str, text: &str| (at.to_owned(), text.to_owned());
        let texts = vec![
            pair("tool_response.stdout (key)", "stdout"),
            pair("tool_response.stdout", "ok"),
            pair("tool_response.interrupted (key)", "interrupted"),
        ];
        let tool = String::from("Bash");
        assert_eq!(got, Call::Output { tool, texts });
    }

    #[test]
    fn a_payload_with_no_event_or_no_json_is_refused_by_name() {
        let why = call(r#"{"tool_name":"Read"}"#).err().unwrap_or_default();
        assert!(why.contains("hook_event_name"), "{why}");
        let why = call("not json").err().unwrap_or_default();
        assert!(why.starts_with("hook input is not JSON"), "{why}");
    }

    #[test]
    fn a_pass_prints_nothing() {
        assert_eq!(response(Event::Before, &Verdict::Pass), "");
        assert_eq!(response(Event::After, &Verdict::Pass), "");
    }

    #[test]
    fn a_blocked_read_is_a_permission_denial() {
        let got = response(Event::Before, &Verdict::Block(String::from("x")));
        let want = r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"x"}}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn blocked_output_is_a_block_decision_with_its_reason() {
        let got = response(Event::After, &Verdict::Block(String::from("y")));
        assert_eq!(got, r#"{"decision":"block","reason":"y"}"#);
    }

    #[test]
    fn a_note_is_additional_context_for_its_own_event() {
        let note = Verdict::Note(String::from("z"));
        let before = r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"z"}}"#;
        assert_eq!(response(Event::Before, &note), before);
        let after = response(Event::After, &note);
        assert!(
            after.contains(r#""hookEventName":"PostToolUse""#),
            "{after}"
        );
    }

    /// A reason is escaped and ASCII: a quote in a path, or the hazard
    /// itself echoed in a reason, cannot break the document.
    #[test]
    fn a_reason_is_escaped_and_ascii() {
        let why = Verdict::Block(String::from("a \"q\" \u{202E}"));
        let got = response(Event::After, &why);
        assert_eq!(got, r#"{"decision":"block","reason":"a \"q\" \u202e"}"#);
    }
}
