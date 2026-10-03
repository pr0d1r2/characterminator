//! Rewriting through CLASSES rather than entries: the fallback path to
//! ASCII, the fidelity's own family, and a family the map declares. A
//! child of the `apply` tests, for their fixtures.

use super::{ascii, check, fix};
use crate::fix::family::ROOT;
use crate::fix::map::Map;
use crate::rules::Origin;

/// A map that rewrites through CLASSES rather than entries, including a
/// family the builtin tree does not carry.
const CLASSES: &str = "\
        family nerd emoji\n\
        = tick ascii:[x] text:U+2713 emoji:U+2705 nerd:U+F00C\n\
        = hand emoji:U+1F44D+U+1F3FD,U+1F44D ascii:+1\n";

fn classed(fidelity: &str) -> Map {
    Map::parse(CLASSES, &|line| Origin::Builtin { line })
        .and_then(|map| map.with_fidelity(fidelity))
        .unwrap_or_default()
}

fn ascii_or_emoji(ch: char) -> bool {
    ch.is_ascii() || ch == '\u{2705}' || ch == '\u{1F44D}'
}

fn ascii_or_nerd(ch: char) -> bool {
    ch.is_ascii() || ch == '\u{F00C}'
}

#[test]
fn a_class_falls_back_along_the_path_to_ascii() {
    let done = fix("a\u{2713}b", &classed("emoji"), ascii);
    assert_eq!(done.map(|f| f.output).unwrap_or_default(), "a[x]b");
}

#[test]
fn a_class_compresses_into_the_fidelity_family() {
    let done = fix("\u{2713}", &classed("emoji"), ascii_or_emoji);
    assert_eq!(done.map(|f| f.output).unwrap_or_default(), "\u{2705}");
}

#[test]
fn a_declared_family_takes_part_in_resolution() {
    let done = fix("\u{2713}", &classed("nerd"), ascii_or_nerd);
    assert_eq!(done.map(|f| f.output).unwrap_or_default(), "\u{F00C}");
}

#[test]
fn the_longest_class_member_wins() {
    let text = "\u{1F44D}\u{1F3FD}";
    let report = check(text, &classed(ROOT), &ascii).unwrap_or_default();
    assert_eq!(report.rewrites.len(), 1);
    let first = report.rewrites.first().map(|done| done.to.clone());
    assert_eq!(first, Some(String::from("+1")));
}

#[test]
fn a_class_with_no_allowed_member_is_kept_and_reported() {
    let source = "= tick text:U+2713 emoji:U+2705\n";
    let map = Map::parse(source, &|line| Origin::Builtin { line })
        .unwrap_or_default();
    let report = check("\u{2713}", &map, &ascii).unwrap_or_default();
    assert_eq!(report.unmapped.len(), 1);
    assert!(!report.drifted());
    let done = fix("\u{2713}", &map, ascii).unwrap_or_default();
    assert_eq!(done.output, "\u{2713}");
}
