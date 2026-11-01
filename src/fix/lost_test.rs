//! The tests of `lost.rs`, in a file of their own (sherd V50).

use super::{recover, to_lossy};
use crate::fix::Rewrite;
use crate::scan::{Hit, Position};

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
fn a_lossy_copy_gives_way_to_the_next_one() {
    let working = "\u{FFFD}F here\n";
    let (output, rewrites) = recover(working, &[working, "\u{A7}F here\n"]);
    assert_eq!((output.as_str(), rewrites.len()), ("\u{A7}F here\n", 1));
}

#[test]
fn a_lossy_line_is_no_rival_for_a_line_that_moved() {
    let committed = "\u{FFFD}F here\n\u{A7}F here\n";
    let (output, _) = recover("x\ny\n\u{FFFD}F here\n", &[committed]);
    assert_eq!(output, "x\ny\n\u{A7}F here\n");
}

#[test]
fn a_line_restored_only_in_part_falls_to_the_next_copy() {
    let working = "\u{FFFD} \u{FFFD}\n";
    let references = ["\u{A7} \u{FFFD}\n", "\u{A9} \u{AE}\n"];
    let (output, _) = recover(working, &references);
    assert_eq!(output, "\u{A9} \u{AE}\n");
}

#[test]
fn the_first_reference_that_shows_the_line_wins() {
    let working = "\u{FFFD}x\n";
    let (output, _) = recover(working, &["nope\n", "\u{A7}x\n", "\u{A9}x\n"]);
    assert_eq!(output, "\u{A7}x\n");
}

/// Where `byte` of the recovered text sits in the lossy one.
fn moved(byte: usize, restored: &[Rewrite]) -> usize {
    let (line, column, character) = (1, 1, 'x');
    let position = Position { line, column, byte };
    let mut hit = Hit {
        position,
        character,
    };
    to_lossy(&mut hit, restored);
    hit.position.byte
}

/// Each character of the recovered text, moved back, lands on the byte its
/// twin has in the lossy text, past originals shorter and longer than `U+FFFD`.
#[test]
fn a_hit_is_moved_back_to_the_lossy_text() {
    let lossy = "\u{FFFD}a\u{FFFD}b\u{2014}\u{FFFD}\n";
    let original = "\u{A7}a\u{1F600}b\u{2014}\u{A7}\n";
    let (recovered, rewrites) = recover(lossy, &[original]);
    let bytes = recovered.char_indices().map(|at| moved(at.0, &rewrites));
    let got: Vec<usize> = bytes.collect();
    let want: Vec<usize> = lossy.char_indices().map(|at| at.0).collect();
    assert_eq!(got, want);
}
