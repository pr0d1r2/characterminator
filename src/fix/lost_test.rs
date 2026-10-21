//! The tests of `lost.rs`: what is put back, and what is left alone.

use super::{lost_in, restored};

#[test]
fn a_lost_character_is_put_back_from_the_same_line() {
    let was = "// hyphenated \u{A7}F node cannot be cited\n";
    let now = "// hyphenated \u{FFFD}F node cannot be cited\n";
    assert_eq!(restored(now, was), was);
}

#[test]
fn a_line_that_moved_is_found_when_one_line_matches() {
    let was = "first\nkeep \u{A7} here\n";
    let now = "added\nfirst\nkeep \u{FFFD} here\n";
    assert_eq!(restored(now, was), "added\nfirst\nkeep \u{A7} here\n");
}

#[test]
fn two_different_candidates_leave_the_character_alone() {
    let was = "a\u{A7}b\na\u{B6}b\n";
    let now = "x\ny\nz\na\u{FFFD}b\n";
    assert_eq!(restored(now, was), now);
}

#[test]
fn the_same_number_line_wins_over_a_look_alike() {
    let was = "a\u{A7}b\na\u{B6}b\n";
    let now = "a\u{FFFD}b\na\u{FFFD}b\n";
    assert_eq!(restored(now, was), was);
}

#[test]
fn a_changed_rest_of_the_line_leaves_it_alone() {
    let was = "a\u{A7}b\n";
    assert_eq!(restored("a\u{FFFD}c\n", was), "a\u{FFFD}c\n");
    assert_eq!(restored("a\u{FFFD}bb\n", was), "a\u{FFFD}bb\n");
    assert_eq!(restored("a\u{FFFD}\n", was), "a\u{FFFD}\n");
}

#[test]
fn a_original_that_is_lost_too_restores_nothing() {
    let now = "a\u{FFFD}b\n";
    assert_eq!(restored(now, now), now);
}

#[test]
fn each_lost_character_of_a_line_is_restored() {
    let was = "\u{A7}x\u{2014}\n";
    assert_eq!(restored("\u{FFFD}x\u{FFFD}\n", was), was);
}

#[test]
fn lines_without_the_character_are_untouched() {
    let text = "plain\r\nlines\n";
    assert_eq!(restored(text, "other\n"), text);
    assert_eq!(lost_in("a\u{FFFD}b\u{FFFD}"), 2);
}
