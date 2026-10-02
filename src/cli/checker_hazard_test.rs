//! The hazards (V34, V36) and what exempts a character from one: an RGI
//! emoji sequence (`src/lint:V63`) and a script preset's own joiner
//! (`src/lint:V57`). A child of the `checker` tests, for their helpers.

use super::{any, findings, fixture, report};
use crate::charset::{CharSet, builtin};
use crate::cli::check::run;
use crate::cli::config::Config;
use crate::lint::Level;
use crate::render::Format;

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

/// The whole path, through a real `.ctrm`: a rule that grants `any`
/// and tries to allow the group, the lint, and the charset findings,
/// plus a `.ctrm-sets` that redeclares the class as harmless. None of
/// it lowers the forbid (V36), and the run fails.
#[test]
fn no_configuration_talks_a_hazard_down() {
    let files = [
        (".ctrm", "* any !hazard=allow !invisible=allow !allow\n"),
        (".ctrm-sets", "hazard-invisible U+0041\n"),
        ("smuggled.txt", "fine\u{200B}\n"),
    ];
    let Some(root) = fixture("ctrm-hazard-fixture", &files) else {
        return;
    };
    let paths = [String::from("smuggled.txt")];
    let report = run(&Config::discovered(&root), &paths, Format::Json);
    let report = report.unwrap_or_else(|why| unreachable!("{why}"));
    assert_eq!(report.code, 1, "{}", report.text);
    assert!(report.text.contains("\"invisible\""), "{}", report.text);
    assert!(report.text.contains("\"forbid\""), "{}", report.text);
}

/// `src/lint:V57`: a Persian word spelled with ZWNJ (mi-khaham, "I
/// want") is clean where the rule names `persian`, and the same bytes
/// under `any` still fire -- granting everything excuses nothing.
#[test]
fn a_script_preset_excuses_its_own_joiner_and_any_does_not() {
    let word =
        "\u{0645}\u{06CC}\u{200C}\u{062E}\u{0648}\u{0627}\u{0647}\u{0645}\n";
    let files = [("fa.md", word)];
    let fa = report("ctrm-joiner-fa", "* ascii+persian\n", &files);
    assert_eq!(fa, "");
    let any = report("ctrm-joiner-any", "* any\n", &files);
    assert!(any.contains("U+200C"), "{any}");
}

/// The excuse is the joiner the preset grants and nothing else: a bidi
/// control in a Persian file still fires, and `persian` does not
/// excuse the ZERO WIDTH JOINER that `hindi` does.
#[test]
fn a_script_preset_excuses_nothing_but_its_joiners() {
    let bidi = [("fa.md", "\u{0645}\u{202E}\u{0645}\n")];
    let fired = report("ctrm-joiner-bidi", "* ascii+persian\n", &bidi);
    assert!(fired.contains("U+202E"), "{fired}");
    let conjunct = [("hi.md", "\u{0915}\u{094D}\u{200D}\u{0937}\n")];
    let hi = report("ctrm-joiner-hi", "* ascii+hindi\n", &conjunct);
    assert_eq!(hi, "");
    let fa = report("ctrm-joiner-zwj", "* ascii+persian\n", &conjunct);
    assert!(fa.contains("U+200D"), "{fa}");
}

/// B19: a `.ctrm-sets` line NAMED `hindi` is not the preset, so the
/// ZWNJ it grants is still a hazard (`src/lint:V57`).
#[test]
fn a_redeclared_preset_excuses_no_joiner() {
    let sets = (".ctrm-sets", "hindi ascii U+200C U+200D\n");
    let files = [sets, ("a.md", "a\u{200C}b\n")];
    let got = report("ctrm-joiner-redeclared", "* hindi\n", &files);
    assert!(got.contains("U+200C"), "{got}");
}
