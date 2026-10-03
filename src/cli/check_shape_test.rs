//! Bounded `check` output through the verb (`src/render:V124`):
//! `--summary`, `--max` and the tally. A child of `check_test.rs`;
//! `super::super` is `check`.

use super::super::super::config::discovered;
use super::super::{Report, shaped};
use crate::cli::testkit::fixture;
use crate::render::{Format, Shape};

/// Two em dashes and an U+2261 in one file, one em dash in another.
fn ran(name: &str, format: Format, shape: Shape) -> Report {
    let files = [
        ("a.md", "\u{2014}\u{2014}\u{2261}\n"),
        ("b.md", "\u{2014}\n"),
    ];
    let none = Report {
        text: String::new(),
        code: 9,
        note: String::new(),
    };
    let Some(root) = fixture(name, &files) else {
        return none;
    };
    let asked = [String::from("a.md"), String::from("b.md")];
    shaped(&discovered(&root), &asked, format, shape).unwrap_or(none)
}

const SUMMARY: Shape = Shape {
    summary: true,
    max: None,
};

/// One row per (file, code point): first place, count, replacement.
#[test]
fn a_summary_folds_each_file_by_code_point() {
    let said = ran("ctrm-shape-summary", Format::Human, SUMMARY);
    let want = concat!(
        "a.md:1:1 U+2014 ascii x2 -> \"--\"\n",
        "a.md:1:3 U+2261 ascii x1\n",
        "b.md:1:1 U+2014 ascii x1 -> \"--\""
    );
    assert_eq!((said.text.as_str(), said.code), (want, 1));
}

/// The json summary is a document of its own: `groups`, each a
/// violation plus `count`.
#[test]
fn a_json_summary_carries_groups_with_counts() {
    let said = ran("ctrm-shape-json", Format::Json, SUMMARY);
    let head = r#"{"schema":1,"verb":"check","groups":[{"path":"a.md","#;
    let first = r#""replacement":"--","fixable":true,"count":2}"#;
    assert!(said.text.starts_with(head), "{}", said.text);
    assert!(said.text.contains(first), "{}", said.text);
    assert_eq!(said.note, "", "the tally is human only");
}

/// `--max` cuts the rows and says how many it hid; the tally on stderr
/// still counts every finding.
#[test]
fn max_cuts_the_rows_and_the_tally_counts_them_all() {
    let shape = Shape {
        summary: false,
        max: Some(2),
    };
    let said = ran("ctrm-shape-max", Format::Human, shape);
    let want = "a.md:1:1 U+2014 ascii\na.md:1:2 U+2014 ascii\n... 2 more";
    assert_eq!(said.text, want);
    let tally = "4 findings in 2 files; most: U+2014 x3, U+2261 x1 -- see \
                 'ctrm sets --containing U+2014' or 'ctrm init'";
    assert_eq!(said.note, tally);
}
