//! The builtin map's emoji sequences (V62), run through the engine,
//! and the multi-pass fix they need (V65). A child of the `apply` tests.

use super::{Hit, Map, Origin, fix};
use crate::charset::{CharSet, builtin};
use std::sync::LazyLock;

/// Built once: every test here asks the same map and preset.
static MAP: LazyLock<Map> = LazyLock::new(builtin_map);
static EMOJI: LazyLock<CharSet> = LazyLock::new(emoji);

fn builtin_map() -> Map {
    Map::parse(crate::fix::BUILTIN, &|line| Origin::Builtin { line })
        .unwrap_or_default()
}

/// The `emoji` preset, which a file grants on top of ASCII.
fn emoji() -> CharSet {
    let found = builtin::catalog()
        .ok()
        .and_then(|c| c.resolve("emoji", "emoji").ok());
    assert!(found.is_some());
    found.unwrap_or_else(builtin::ascii)
}

/// `text` fixed by the builtin map, with `emoji` granted or not.
fn fixed(text: &str, grant_emoji: bool) -> String {
    let allowed = |c: char| c.is_ascii() || grant_emoji && EMOJI.contains(c);
    fix(text, &MAP, &allowed)
        .map(|done| done.output)
        .unwrap_or_else(|why| format!("<{why}>"))
}

/// England: U+1F3F4, the tag letters `gbeng`, the cancel tag.
const ENGLAND: &str =
    "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";
/// A gendered, skin-toned couple with heart.
const COUPLE: &str = "\u{1F469}\u{1F3FB}\u{200D}\u{2764}\u{FE0F}\
        \u{200D}\u{1F468}\u{1F3FF}";
/// Kiss: man, man. (The untoned neutral kiss is U+1F48F itself.)
const KISS: &str = "\u{1F468}\u{200D}\u{2764}\u{FE0F}\u{200D}\
        \u{1F48B}\u{200D}\u{1F468}";
/// Man, woman, girl.
const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";

/// One fixture per target kind. The ASCII targets are asked under
/// `ascii`; the emoji ones under `emoji`, where they settle.
const CASES: [(&str, &str, bool); 10] = [
    ("1\u{FE0F}\u{20E3}", "1", false),
    ("#\u{FE0F}\u{20E3}", "#", false),
    ("*\u{FE0F}\u{20E3}", "*", false),
    ("\u{1F1F5}\u{1F1F1}", " PL ", true),
    (ENGLAND, " GB-ENG ", true),
    (FAMILY, "\u{1F46A}", true),
    (COUPLE, "\u{1F491}", true),
    (KISS, "\u{1F48F}", true),
    ("\u{1F468}\u{1F3FD}\u{200D}\u{1F4BB}", "\u{1F468}", true),
    ("\u{1F3F3}\u{FE0F}\u{200D}\u{1F308}", "\u{1F3F3}", true),
];

#[test]
fn each_kind_of_sequence_lands_on_its_target() {
    for (text, want, grant) in CASES {
        let got = fixed(&format!("a{text}b"), grant);
        assert_eq!(got, format!("a{want}b"), "{text:?}");
    }
}

/// B34: mixed-tone holding hands and handshake land on their one
/// code point (UTS #51 section 2.6), as the same-tone forms do via
/// V60; people holding hands has none, so keeps its first emoji.
const HANDS: [(&str, &str); 5] = [
    (
        "\u{1F468}\u{1F3FB}\u{200D}\u{1F91D}\u{200D}\u{1F468}\u{1F3FC}",
        "\u{1F46C}",
    ),
    (
        "\u{1F469}\u{1F3FB}\u{200D}\u{1F91D}\u{200D}\u{1F469}\u{1F3FC}",
        "\u{1F46D}",
    ),
    (
        "\u{1F469}\u{1F3FB}\u{200D}\u{1F91D}\u{200D}\u{1F468}\u{1F3FC}",
        "\u{1F46B}",
    ),
    ("\u{1FAF1}\u{1F3FB}\u{200D}\u{1FAF2}\u{1F3FC}", "\u{1F91D}"),
    (
        "\u{1F9D1}\u{1F3FB}\u{200D}\u{1F91D}\u{200D}\u{1F9D1}\u{1F3FC}",
        "\u{1F9D1}",
    ),
];

/// V76, B35: regional indicators pair from the start of their
/// run (`XU` then a lone `S`, never `US`), and a flag's code is
/// a word, kept off a letter or another code beside it.
const FLAGS: [(&str, &str); 5] = [
    ("\u{1F1FD}\u{1F1FA}\u{1F1F8}", "\u{1F1FD}\u{1F1FA}\u{1F1F8}"),
    (
        "\u{1F1FD}\u{1F1FA}\u{1F1FA}\u{1F1F8}",
        "\u{1F1FD}\u{1F1FA}US",
    ),
    ("\u{1F1F5}\u{1F1F1}\u{1F1E9}\u{1F1EA}", "PL DE"),
    ("A\u{1F1F5}\u{1F1F1}B", "A PL B"),
    ("x \u{1F1F5}\u{1F1F1}.", "x PL."),
];

/// Each under `ascii` and under `emoji`: `fix` returns, so V6
/// held, the bytes around the flag are as they were, and a second
/// fix changes nothing (V5).
#[test]
fn flags_pair_from_the_run_start_and_stay_apart() {
    let runs = FLAGS.iter().flat_map(|c| [(c, false), (c, true)]);
    for ((text, want), grant) in runs {
        let once = fixed(text, grant);
        assert_eq!(once, *want, "{text:?}");
        assert_eq!(fixed(&once, grant), once, "{text:?}");
    }
}

#[test]
fn holding_hands_lands_on_its_grouping() {
    for (text, want) in HANDS {
        assert_eq!(fixed(text, true), want, "{text:?}");
    }
    assert_eq!(fixed("\u{1F46C}\u{1F3FB}", true), "\u{1F46C}");
}

/// Under `ascii` a ZWJ sequence still compresses: one finding is
/// left where there were several, since the target has no ASCII.
#[test]
fn under_ascii_a_zwj_sequence_leaves_one_emoji() {
    assert_eq!(fixed(FAMILY, false), "\u{1F46A}");
}

/// A joiner in no RGI sequence is not a sequence: it stays, and
/// is reported (V4), while the emoji around it are left alone.
#[test]
fn a_joiner_outside_any_listed_sequence_stays_reported() {
    let set = emoji();
    let text = "\u{1F600}\u{200D}\u{1F600}";
    let done = fix(text, &builtin_map(), &|c| set.contains(c));
    let done = done.unwrap_or_default();
    assert_eq!(done.output, text);
    let unmapped = done.report.unmapped.first().map(|h| h.character);
    assert_eq!(unmapped, Some('\u{200D}'));
}

/// A file granting every code point of a sequence keeps it (V6).
#[test]
fn a_granted_sequence_is_left_alone() {
    let text = format!("x{COUPLE}y");
    let all = fix(&text, &builtin_map(), &|_| true);
    assert_eq!(all.map(|done| done.output).ok(), Some(text));
}

/// The V26 and V60 entries answer as before.
#[test]
fn the_single_code_point_entries_are_unchanged() {
    assert_eq!(fixed("a\u{2014}b", false), "a--b");
    assert_eq!(fixed("\u{201C}q\u{201D}", false), "\"q\"");
    assert_eq!(fixed("\u{1F44D}\u{1F3FD}", true), "\u{1F44D}");
    assert_eq!(fixed("\u{2764}\u{FE0F}", true), "\u{2764}");
}

/// B14: a deletion that REVEALS a sequence the scan walked past
/// settles in a later pass (V65) instead of being refused as
/// unsettled (V5).
#[test]
fn a_sequence_revealed_by_a_deletion_still_settles() {
    let stray = "\u{1F469}\u{FE0F}\u{200D}\u{1F4BB}";
    assert_eq!(fixed(stray, true), "\u{1F469}");
    let toned = "\u{1F468}\u{1F3FB}\u{200D}\u{1F469}\u{1F3FB}\
            \u{200D}\u{1F467}";
    assert_eq!(fixed(toned, true), "\u{1F46A}");
}

/// B24: a later pass reads text an earlier one shortened, and its
/// rows are still reported where they sit in the ORIGINAL: every
/// rewrite and every kept character resolves in the file on disk,
/// at its byte, line and column, in one byte-ordered list.
#[test]
fn every_row_of_a_multi_pass_fix_resolves_in_the_original() {
    let text = "ab\u{2014}\u{1F469}\u{FE0F}\u{200D}\u{1F4BB}\u{2014}\
            \u{1F1F5}\u{FE0F}\u{1F1F1} z\n";
    let done = fix(text, &MAP, &|c: char| c.is_ascii());
    assert!(done.is_ok());
    let report = done.unwrap_or_default().report;
    let rows = report.rewrites.iter().map(|r| r.hit);
    assert!(rows.clone().is_sorted_by_key(|h| h.position.byte));
    let hits: Vec<Hit> = rows.chain(report.unmapped).collect();
    assert!(hits.len() > 4, "{hits:?}");
    let truth: Vec<Hit> = crate::scan::located(text).collect();
    for hit in &hits {
        assert!(truth.contains(hit), "{hit:?} is not in the file");
    }
}

/// Every listed sequence, under `ascii` and under `emoji`: `fix`
/// returns, so V6 and V5 held, and a second fix changes nothing.
#[test]
fn every_listed_sequence_settles_and_is_idempotent() {
    let map = &*MAP;
    let long = |e: &&crate::fix::MapEntry| e.from.chars().nth(1).is_some();
    let texts = map.entries().iter().filter(long);
    let texts = texts.map(|entry| format!("a{}b", entry.from));
    for (text, grant) in texts.flat_map(|t| [(t.clone(), false), (t, true)]) {
        let once = fixed(&text, grant);
        assert!(!once.starts_with('<'), "{text:?}: {once}");
        assert_eq!(fixed(&once, grant), once, "{text:?}");
    }
}

/// The rows of a multi-pass fix are relocated in ONE walk of the
/// original, sorted by byte, not one walk per row (a large file went
/// quadratic). Many lines, each needing a second pass: every row still
/// lands on its own line, at a byte, line and column the file has.
#[test]
fn many_multi_pass_rows_each_resolve_on_their_own_line() {
    let line = "a\u{2014} \u{1F469}\u{FE0F}\u{200D}\u{1F4BB} \u{2261}\n";
    let text = line.repeat(40);
    let done = fix(&text, &MAP, &|c: char| c.is_ascii()).unwrap_or_default();
    let rows = done.report.rewrites.iter().map(|r| r.hit);
    let hits: Vec<Hit> = rows.chain(done.report.unmapped).collect();
    let truth: Vec<Hit> = crate::scan::located(&text).collect();
    for hit in &hits {
        assert!(truth.contains(hit), "{hit:?} is not in the file");
    }
    for at in 1..=40 {
        let here = hits.iter().filter(|h| h.position.line == at).count();
        assert!(here >= 3, "line {at}: {here} rows in {hits:?}");
    }
}
