//! The tests of `splice.rs`, in a file of their own (sherd V50).

use super::{current, replace, sample};

const DOC: &str = "intro\n<!-- BEGIN x -->\nold\n<!-- END x -->\noutro\n";

#[test]
fn replace_swaps_the_body_and_keeps_the_markers() {
    assert_eq!(
        replace(DOC, "x", "new\n"),
        Ok("intro\n<!-- BEGIN x -->\nnew\n<!-- END x -->\noutro\n".to_owned())
    );
}

#[test]
fn current_returns_the_body_byte_for_byte() {
    assert_eq!(current(DOC, "x"), Ok("old\n"));
}

#[test]
fn splicing_twice_is_splicing_once() {
    let once = replace(DOC, "x", "new\n").unwrap_or_default();
    assert_eq!(replace(&once, "x", "new\n"), Ok(once.clone()));
}

#[test]
fn a_marker_name_is_exact_not_a_prefix() {
    let doc = "<!-- BEGIN xy -->\nkeep\n<!-- END xy -->\n";
    assert!(replace(doc, "x", "new\n").is_err());
}

#[test]
fn a_marker_is_a_whole_line() {
    let doc = "see <!-- BEGIN x --> here\n<!-- END x -->\n";
    assert!(replace(doc, "x", "new\n").is_err());
}

#[test]
fn a_missing_pair_is_an_error_naming_the_markers() {
    let err = replace("no markers\n", "x", "new\n")
        .err()
        .unwrap_or_default();
    assert!(err.contains("<!-- BEGIN x -->"), "{err}");
}

#[test]
fn an_end_before_its_begin_is_an_error() {
    let doc = "<!-- END x -->\n<!-- BEGIN x -->\n";
    let err = replace(doc, "x", "new\n").err().unwrap_or_default();
    assert!(err.contains("comes before"), "{err}");
}

#[test]
fn a_duplicated_pair_is_an_error() {
    let doc = format!("{DOC}{DOC}");
    let err = replace(&doc, "x", "new\n").err().unwrap_or_default();
    assert!(err.contains("more than once"), "{err}");
}

#[test]
fn a_crlf_document_still_finds_its_markers() {
    let doc = "<!-- BEGIN x -->\r\nold\r\n<!-- END x -->\r\n";
    assert_eq!(current(doc, "x"), Ok("old\r\n"));
}

#[test]
fn sample_names_lines_each_side_lacks() {
    assert_eq!(
        sample("a\nb\n", "a\nc\n"),
        vec!["want: b".to_owned(), "have: c".to_owned()]
    );
}

#[test]
fn sample_caps_each_side_at_three_lines() {
    assert_eq!(sample("1\n2\n3\n4\n5\n", "").len(), 3);
}
