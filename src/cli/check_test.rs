//! The tests of `check.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `check`.
//!
//! What the VERB owns: a path matched as it is shown (`src/judge:V71`),
//! the exit code a skip earns, and the shape a finding is reported in.
//! What a finding IS -- the judging -- is tested in `src/judge`; the
//! judging through a real tree is the `judged` child below.

use super::super::config::discovered;
use super::run;
use crate::cli::testkit::fixture;
use crate::lint::Group;
use crate::render::Format;

#[path = "check_judged_test.rs"]
mod judged;

/// `src/judge:V71` through the verb: an anchored rule reaches a file named the
/// long way round, so it is judged by that rule and not by `ascii`.
#[test]
fn an_anchored_rule_reaches_a_path_spelled_with_dot_dot() {
    let Some(root) = fixture("ctrm-dotdot", &[(".ctrm", "sub/c.md any\n")])
    else {
        return;
    };
    let wrote = std::fs::create_dir_all(root.join("sub"))
        .and_then(|()| std::fs::write(root.join("sub/c.md"), "\u{2014}\n"));
    assert!(wrote.is_ok());
    let asked = [String::from("sub/../sub/c.md")];
    let found = run(&discovered(&root), &asked, Format::Human);
    assert_eq!(found.map(|r| (r.text, r.code)), Ok((String::new(), 0)));
}

/// The exit code `check` gives one file of raw `bytes`, per format.
fn code_for(name: &str, bytes: &[u8]) -> Vec<u8> {
    let Some(root) = fixture(name, &[]) else {
        return vec![];
    };
    if std::fs::write(root.join("f.txt"), bytes).is_err() {
        return vec![];
    }
    let config = discovered(&root);
    let asked = [String::from("f.txt")];
    [Format::Human, Format::Json, Format::Sarif]
        .into_iter()
        .map(|form| run(&config, &asked, form).map_or(9, |r| r.code))
        .collect()
}

/// B22: invalid UTF-8 is an error, exit 1, in every format
/// (`src/scan:V8`); a binary skip is named and stays exit 0.
#[test]
fn invalid_utf8_fails_and_a_binary_skip_does_not() {
    assert_eq!(code_for("ctrm-not-utf8", b"ab\xffcd\n"), vec![1, 1, 1]);
    assert_eq!(code_for("ctrm-binary-exit", b"ab\0cd\n"), vec![0, 0, 0]);
}

/// The human row names the `hazard` set, not the file's: the set did
/// not decide a hazard, and `any` in that column would read as though
/// the override fell outside it.
#[test]
fn a_hazard_row_names_the_hazard_set() {
    let files = [(".ctrm", "* any\n"), ("trojan.rs", "a\u{202E}b\n")];
    let Some(root) = fixture("ctrm-hazard-row-fixture", &files) else {
        return;
    };
    let paths = [String::from("trojan.rs")];
    let report = run(&discovered(&root), &paths, Format::Human);
    let text = report.map(|r| r.text).unwrap_or_else(|why| why);
    assert_eq!(text, "trojan.rs:1:2 U+202E hazard");
    assert_eq!(Group::Hazard.name(), "hazard");
}

/// The json contract is unchanged by a pedantic finding: the same keys,
/// `set` the set in force, `lint` saying which, asserted whole (V11).
#[test]
fn a_pedantic_finding_keeps_the_json_contract() {
    let ctrm = "* ascii+cr !crlf=deny\n";
    let files = [(".ctrm", ctrm), ("n.txt", "hi\r\n")];
    let Some(root) = fixture("ctrm-crlf-json", &files) else {
        return;
    };
    let paths = [String::from("n.txt")];
    let report = run(&discovered(&root), &paths, Format::Json);
    let report = report.unwrap_or_else(|why| unreachable!("{why}"));
    let expected = concat!(
        r#"{"verb":"check","violations":[{"path":"n.txt","line":1,"#,
        r#""column":3,"byte":2,"codepoint":"U+000D","character":"\r","#,
        r#""set":"ascii+cr","lint":"crlf","level":"deny"}],"skipped":[]}"#
    );
    assert_eq!((report.text.as_str(), report.code), (expected, 1));
}

/// SARIF: the lint is the `ruleId`, and the region is the one
/// character the finding points at.
#[test]
fn a_pedantic_finding_is_a_sarif_result_on_one_character() {
    let ctrm = "* ascii !final-newline=warn\n";
    let files = [(".ctrm", ctrm), ("n.txt", "ab")];
    let Some(root) = fixture("ctrm-final-sarif", &files) else {
        return;
    };
    let paths = [String::from("n.txt")];
    let found = run(&discovered(&root), &paths, Format::Sarif);
    let log = found.map(|r| r.text).unwrap_or_else(|why| why);
    let result = concat!(
        r#"{"ruleId":"final-newline","level":"warning","#,
        r#""message":{"text":"U+0062 (set in force: ascii)"},"#
    );
    let region = r#""startLine":1,"startColumn":2,"endColumn":3}"#;
    assert!(log.contains(result) && log.contains(region), "{log}");
}
