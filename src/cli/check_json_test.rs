//! The json contract through the verbs (`src/render:V95`, V122): one
//! violation shape in `check`, `fix --check`'s `unmapped` and its
//! `rewrites`, each saying what `fix` would write. A child of
//! `check_test.rs`; `super::super` is `check`.

use super::super::super::config::discovered;
use super::super::super::fix;
use super::super::run;
use crate::cli::testkit::fixture;
use crate::render::Format;

/// An em dash the map covers, then U+2261, which it does not.
const TEXT: &str = "a\u{2014}\u{2261}\n";

/// The keys every violation ends with, after `codepoint` and `character`.
fn tail(lint: &str, level: &str, replacement: &str, fixable: bool) -> String {
    format!(
        r#""lint":"{lint}","level":"{level}","replacement":{replacement},"fixable":{fixable}"#
    )
}

/// V122: `replacement` is what `fix` would write, `null` when nothing
/// would be; `fixable` says whether the next `check` would still see it.
#[test]
fn a_check_violation_says_what_fix_would_write() {
    let Some(root) = fixture("ctrm-json-remedy", &[("n.md", TEXT)]) else {
        return;
    };
    let asked = [String::from("n.md")];
    let report = run(&discovered(&root), &asked, Format::Json);
    let text = report.map(|r| r.text).unwrap_or_default();
    let mapped = tail("outside-set", "deny", r#""--""#, true);
    let kept = tail("outside-set", "deny", "null", false);
    assert!(text.contains(&mapped), "{text}");
    assert!(text.contains(&kept), "{text}");
}

/// V95: a rewrite carries the violation's keys, judged as `check` judged
/// the character -- a hazard names the `hazard` set and its own lint.
#[test]
fn a_rewrite_carries_the_violation_shape() {
    let files = [("notes.md", "a\u{2014}\u{202E}b\n")];
    let Some(root) = fixture("ctrm-json-rewrite", &files) else {
        return;
    };
    let asked = [String::from("notes.md")];
    let report = fix::run(&discovered(&root), &asked, Format::Json, false);
    let text = report.map(|r| r.text).unwrap_or_default();
    let dash = tail("outside-set", "deny", r#""--""#, true);
    let bidi = tail("bidi-control", "forbid", r#""""#, true);
    assert!(text.contains(&format!(r#""to":"--","set":"ascii",{dash}"#)));
    assert!(text.contains(&format!(r#""to":"","set":"hazard",{bidi}"#)));
}
