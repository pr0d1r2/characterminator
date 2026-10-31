//! The tests of `lost.rs`, in a file of their own (sherd V50).

use super::recover;

#[test]
fn a_lost_character_is_put_back_from_the_same_line() {
    let committed = "a\n// hyphenated \u{A7}F node\nz\n";
    let working = "a\n// hyphenated \u{FFFD}F node\nz\n";
    let (output, rewrites) = recover(working, &[committed]);
    assert_eq!(output, committed);
    let at = rewrites.first().map(|r| (r.hit.position, r.to.as_str()));
    let position = at.map(|(p, _)| (p.line, p.column, p.byte));
    assert_eq!(position, Some((2, 15, 16)));
    assert_eq!(at.map(|(_, to)| to), Some("\u{A7}"));
}

#[test]
fn a_line_that_moved_is_found_when_it_is_the_only_match() {
    let committed = "x\n\u{A7}F here\n";
    let (output, _) = recover("\u{FFFD}F here\n", &[committed]);
    assert_eq!(output, "\u{A7}F here\n");
}

#[test]
fn an_ambiguous_line_is_left_alone() {
    let committed = "\u{A7}F here\n\u{A9}F here\n";
    let working = "x\ny\n\u{FFFD}F here\n";
    let (output, rewrites) = recover(working, &[committed]);
    assert_eq!((output.as_str(), rewrites.len()), (working, 0));
}

#[test]
fn a_line_that_differs_elsewhere_is_left_alone() {
    let working = "\u{FFFD}F node\n";
    let (output, rewrites) = recover(working, &["\u{A7}F nodes\n"]);
    assert_eq!((output.as_str(), rewrites.len()), (working, 0));
}

#[test]
fn a_reference_that_is_lossy_itself_restores_nothing() {
    let working = "a \u{FFFD} b\n";
    let (output, rewrites) = recover(working, &[working]);
    assert_eq!((output.as_str(), rewrites.len()), (working, 0));
}

#[test]
fn the_first_reference_that_shows_the_line_wins() {
    let working = "\u{FFFD}x\n";
    let (output, _) = recover(working, &["nope\n", "\u{A7}x\n", "\u{A9}x\n"]);
    assert_eq!(output, "\u{A7}x\n");
}
