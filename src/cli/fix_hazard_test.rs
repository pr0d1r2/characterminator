//! `src/fix:V104` through the verb: a hazard is rewritten even where the
//! file's set grants it, judged EXACTLY as `check` judges it -- so what
//! `check` lets off (a BOM at byte 0, a joiner inside an RGI sequence, a
//! joiner the file's own script preset spells with) is left byte for byte,
//! and what `check` fires on is gone after one bare `fix`.

use super::{ran, read};
use crate::cli::testkit::fixture;

/// A bare `fix` over one `notes.md` under `ctrm`: the file it leaves, the
/// report, and the exit code.
fn fixed(name: &str, ctrm: &str, text: &str) -> (String, String, u8) {
    let files = [(".ctrm", ctrm), ("notes.md", text)];
    let Some(root) = fixture(name, &files) else {
        return (String::new(), String::new(), 9);
    };
    let (out, code) = ran(&root, true);
    (read(&root), out, code)
}

/// Under `any`, every hazard class that draws nothing is deleted: a bidi
/// override, a stray BOM, a zero width space, a tag letter, a lone
/// joiner. The BOM at byte 0 is a signature, not a hazard, and stays.
#[test]
fn every_hazard_that_draws_nothing_is_deleted_under_any() {
    let text = "\u{FEFF}a\u{202E}b\u{FEFF}c\u{200B}d\u{E0041}e\u{200D}f\n";
    let (left, out, code) = fixed("ctrm-fixhz-any", "* any\n", text);
    assert_eq!(left, "\u{FEFF}abcdef\n", "{out}");
    assert_eq!(code, 0, "{out}");
}

/// A control character may mean something, so without a map entry it is
/// kept and reported; WITH one the map rewrites it, granted or not.
#[test]
fn a_control_character_is_rewritten_only_through_the_map() {
    let text = "a\u{7}b\n";
    let (left, out, code) = fixed("ctrm-fixhz-bell", "* any\n", text);
    assert_eq!((left.as_str(), code), (text, 1), "{out}");
    assert_eq!(out, "left: notes.md:1:2 U+0007 hazard");
    let files = [
        (".ctrm", "* any\n"),
        (".ctrm-map", "U+0007 !\n"),
        ("notes.md", text),
    ];
    let Some(root) = fixture("ctrm-fixhz-bell-mapped", &files) else {
        return;
    };
    let (out, code) = ran(&root, true);
    assert_eq!((read(&root).as_str(), code), ("a!b\n", 0), "{out}");
}

/// What `check` lets off is left alone: the joiners of an RGI ZWJ
/// sequence (`src/lint/hazard:V63`) and the ZWNJ Persian spells with
/// (`src/lint/hazard:V57`). Neither is a hazard IN THAT FILE.
#[test]
fn what_check_lets_off_is_left_alone() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\n";
    let (left, out, code) = fixed("ctrm-fixhz-rgi", "* any\n", family);
    assert_eq!((left.as_str(), code), (family, 0), "{out}");
    let word = "\u{0645}\u{06CC}\u{200C}\u{062E}\u{0648}\u{0627}\u{0647}\n";
    let persian = "* ascii+persian\n";
    let (left, out, code) = fixed("ctrm-fixhz-fa", persian, word);
    assert_eq!((left.as_str(), code), (word, 0), "{out}");
    let (left, _, _) = fixed("ctrm-fixhz-fa-any", "* any\n", word);
    assert_eq!(left, word.replace('\u{200C}', ""));
}

/// V5 through the verb: a second bare `fix` changes nothing and reports
/// nothing, and `check` would now pass the file.
#[test]
fn a_second_fix_finds_nothing_left() {
    let text = "x\u{2066}y\u{2069}z\u{00AD}w\n";
    let files = [(".ctrm", "* any\n"), ("notes.md", text)];
    let Some(root) = fixture("ctrm-fixhz-twice", &files) else {
        return;
    };
    let (_, code) = ran(&root, true);
    assert_eq!((read(&root).as_str(), code), ("xyzw\n", 0));
    let (out, code) = ran(&root, true);
    assert_eq!((out.as_str(), code), ("", 0));
}

/// `src/rules:V139`: a path granted `hazard:tags` keeps its tag characters
/// through a bare `fix` -- the artefact the grant exists for -- while any
/// other hazard in it is still deleted.
#[test]
fn a_granted_tag_character_survives_fix_and_the_rest_does_not() {
    let text = "a\u{E0041}b\u{200B}c\n";
    let ctrm = "notes.md ascii+hazard:tags\n";
    let (left, out, code) = fixed("ctrm-fixhz-granted", ctrm, text);
    assert_eq!(left, "a\u{E0041}bc\n", "{out}");
    assert_eq!(code, 0, "{out}");
}

/// `src/lint/hazard:V142`: a bare `fix` re-decodes a double-encoded run to the
/// character it was, losslessly -- in a file granted everything too -- and a
/// second run changes nothing.
#[test]
fn a_double_encoded_run_is_restored_by_fix() {
    let text = "must \u{e2}\u{8a}\u{a5} share \u{e2}\u{80}\u{94} ok\n";
    let (left, out, code) = fixed("ctrm-fixhz-mojibake", "* any\n", text);
    assert_eq!(left, "must \u{22A5} share \u{2014} ok\n", "{out}");
    assert_eq!(code, 0, "{out}");
}

/// The restored character is judged like any other: under `ascii` the dash
/// goes on to the map, and a character nothing maps is KEPT and reported, not
/// left damaged.
#[test]
fn a_restored_character_is_then_judged_by_the_map_and_the_set() {
    let text = "a \u{e2}\u{80}\u{94} b \u{e2}\u{8a}\u{a5}\n";
    let (left, out, code) =
        fixed("ctrm-fixhz-mojibake-ascii", "* ascii\n", text);
    assert_eq!(left, "a -- b \u{22A5}\n", "{out}");
    assert_eq!(code, 1, "{out}");
    assert!(out.ends_with("left: notes.md:1:8 U+22A5 ascii"), "{out}");
}
