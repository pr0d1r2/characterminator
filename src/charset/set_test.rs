//! V86: `contains` searches, and answers exactly as the linear walk it
//! replaced. Kept apart from `set.rs` so the module reads as code.

use crate::charset::adopt_all;
use crate::charset::builtin::catalog;
use crate::charset::range::normalize;
use crate::charset::{CharRange, CharSet};

/// The walk `contains` used to be: the reference the search must agree
/// with on every probe.
fn linear(set: &CharSet, point: char) -> bool {
    set.ranges.iter().any(|range| range.contains(point))
}

/// Every code point where an answer can change: each range's ends and
/// their neighbours, the ASCII edge, and both ends of the code space.
fn probes(set: &CharSet) -> Vec<char> {
    let edges = set
        .ranges
        .iter()
        .flat_map(|r| [r.start as u32, r.end as u32]);
    let near = edges.flat_map(|p| [p.wrapping_sub(1), p, p.saturating_add(1)]);
    let fixed = [0, 0x09, 0x7E, 0x7F, 0x80, 0xD7FF, 0xE000, 0x10_FFFF];
    near.chain(fixed).filter_map(char::from_u32).collect()
}

fn agrees(set: &CharSet) -> bool {
    probes(set)
        .into_iter()
        .all(|p| set.contains(p) == linear(set, p))
}

/// A deterministic xorshift, so a failing set can be rebuilt by seed.
fn next(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

/// A canonical set of up to 64 ranges scattered over the planes, ASCII
/// included, as `new` would build it from config.
fn generated(seed: u32) -> CharSet {
    let mut state = seed;
    let ranges = (0..next(&mut state) % 64).filter_map(|_| {
        let start = next(&mut state) % 0x3_0000;
        let end = start.saturating_add(next(&mut state) % 300);
        CharRange::new(char::from_u32(start)?, char::from_u32(end)?)
    });
    CharSet::new(seed.to_string(), ranges.collect())
}

#[test]
fn search_agrees_with_the_walk_on_generated_sets() {
    let seeds = 1..=500;
    assert!(seeds.map(generated).all(|set| agrees(&set)));
}

#[test]
fn search_agrees_at_the_edges_of_the_code_space() {
    let top = CharRange::single(char::MAX);
    let low = CharRange::single('\0');
    let edges = CharSet::new("edges".to_owned(), vec![top, low]);
    assert!(edges.contains(char::MAX) && edges.contains('\0'));
    assert!(!edges.contains('\u{1}') && !edges.contains('\u{10FFFE}'));
    let empty = CharSet::new("empty".to_owned(), Vec::new());
    assert!(!empty.contains('a') && !empty.contains('\u{4E00}'));
    assert!(agrees(&edges) && agrees(&empty));
}

#[test]
fn an_ascii_point_past_every_leading_range_is_refused() {
    let wide = CharRange::new('\u{0100}', '\u{0200}')
        .unwrap_or(CharRange::single('\u{0100}'));
    let set =
        CharSet::new("high".to_owned(), vec![CharRange::single('a'), wide]);
    assert!(set.contains('a'));
    assert!(!set.contains('b') && !set.contains('\u{7F}'));
    assert!(agrees(&set));
}

/// V86's runner over the real data: every builtin preset and every CLDR
/// locale resolves canonical, and the search agrees with the walk on it.
#[test]
fn every_shipped_set_is_canonical_and_searched_correctly() {
    let mut sets = catalog().unwrap_or_default();
    assert!(adopt_all(&mut sets).is_ok());
    let names: Vec<String> = sets.names().map(String::from).collect();
    assert!(names.iter().any(|name| name == "zh"));
    for name in names {
        let set = sets.resolve(&name, "text");
        assert!(set.is_ok(), "{name} must resolve");
        let set = set.unwrap_or_else(|_| CharSet::new(name, Vec::new()));
        assert_eq!(normalize(set.ranges.clone()), set.ranges, "{}", set.name);
        assert!(agrees(&set), "{}", set.name);
    }
}
