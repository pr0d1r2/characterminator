//! The hazards (V34, V36) and what exempts a character from one: an RGI
//! emoji sequence (`src/lint:V63`) and a script preset's own joiner
//! (`src/lint:V57`). A child of the `checker` tests, for their helpers;
//! the same through a real tree is `src/cli` `check_judged_test.rs`.

use super::{any, findings};
use crate::charset::{CharSet, builtin};
use crate::lint::Level;

/// Trojan Source (CVE-2021-42574) in a file granted EVERYTHING: a
/// RIGHT-TO-LEFT OVERRIDE and an isolate pair around a condition. The
/// set grants them; the level is what fires (V34).
#[test]
fn a_trojan_source_override_fires_under_any() {
    let text = "x = \"\u{202E} }\u{2066}if ok\u{2069} {\";\n";
    let fired = findings(text, &any());
    assert_eq!(fired.len(), 3);
    for (name, level, _) in fired {
        assert_eq!((name, level), ("bidi-control", Level::Forbid));
    }
}

/// ASCII smuggling: TAG LATIN CAPITAL LETTER A and B, invisible to a
/// reader and legible to a model, in a file granted everything.
#[test]
fn tag_letters_fire_under_any() {
    let fired = findings("hi\u{E0041}\u{E0042}\n", &any());
    let tag = ("tag-character", Level::Forbid);
    assert_eq!(fired, vec![(tag.0, tag.1, 2), (tag.0, tag.1, 6)]);
}

/// The other three classes, one each, all under `any`.
#[test]
fn a_control_a_stray_bom_and_an_invisible_fire_under_any() {
    let fired = findings("a\u{001B}b\u{FEFF}c\u{200B}\n", &any());
    let names: Vec<&str> = fired.iter().map(|(n, _, _)| *n).collect();
    assert_eq!(names, ["control-character", "stray-bom", "invisible"]);
    assert!(fired.iter().all(|(_, level, _)| *level == Level::Forbid));
}

/// At byte 0 the BOM is a signature and no hazard: under `any` it is
/// nothing at all, and under `ascii` it is the ordinary charset
/// finding, at the ordinary level.
#[test]
fn a_bom_at_the_start_is_judged_by_the_set_alone() {
    assert_eq!(findings("\u{FEFF}hello\n", &any()), vec![]);
    let ascii = findings("\u{FEFF}hello\n", &builtin::ascii());
    assert_eq!(ascii, vec![("outside-set", Level::Deny, 0)]);
}

/// A character that is both a hazard and outside the set is reported
/// ONCE, as the hazard, at the louder level.
#[test]
fn a_hazard_outside_the_set_is_reported_once_as_the_hazard() {
    let fired = findings("a\u{200B}\n", &builtin::ascii());
    assert_eq!(fired, vec![("invisible", Level::Forbid, 1)]);
}

fn emoji() -> CharSet {
    builtin::catalog()
        .ok()
        .and_then(|c| c.resolve("emoji", "emoji").ok())
        .unwrap_or_else(builtin::ascii)
}

/// `src/lint:V63`: a joiner inside an RGI ZWJ sequence is no hazard,
/// so under `emoji` (which withholds it) it is `outside-set` and the
/// sequence is `fix`'s to compress. A joiner in no listed sequence,
/// and tags after U+1F3F4 that spell no listed flag, still fire.
#[test]
fn a_listed_sequence_turns_its_joiner_into_an_outside_set_finding() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    let fired = findings(family, &emoji());
    let outside = vec![
        ("outside-set", Level::Deny, 4),
        ("outside-set", Level::Deny, 11),
    ];
    assert_eq!(fired, outside);
}

#[test]
fn a_joiner_or_tag_outside_a_listed_sequence_still_fires() {
    let emoji = emoji();
    let stray = findings("\u{1F600}\u{200D}\u{1F600}", &emoji);
    assert_eq!(stray, vec![("invisible", Level::Forbid, 4)]);
    let smuggled = findings("\u{1F3F4}\u{E0069}\u{E007F}", &emoji);
    let tag = ("tag-character", Level::Forbid, 4);
    assert_eq!(smuggled, vec![tag, ("tag-character", Level::Forbid, 8)]);
}
