//! The judging, end to end through `check`: a real tree, a real `.ctrm`, and
//! the report a reader sees. A child of the `check` tests, for their fixture.
//! What a finding IS, without the tree, is tested beside the checker in
//! `src/judge`; these hold the wiring between the two -- the hazards no
//! configuration lowers (`src/lint:V36`), the joiners only a compiled-in preset
//! excuses (`src/lint/hazard:V57`, B19), and the pedantic fixtures exempted per
//! path (`src/lint/pedantic:V37`, `src/lint/pedantic:V58`, B20).

use super::super::run;
use crate::cli::testkit::fixture;
use crate::render::Format;

/// The human report for `files` under `ctrm`, with every file that is
/// not a dotfile named on the command line.
fn report(name: &str, ctrm: &str, files: &[(&str, &str)]) -> String {
    let mut tree = vec![(".ctrm", ctrm)];
    tree.extend_from_slice(files);
    let Some(root) = fixture(name, &tree) else {
        return String::from("(fixture not written)");
    };
    let paths: Vec<String> = files
        .iter()
        .filter(|(path, _)| !path.starts_with('.'))
        .map(|(path, _)| String::from(*path))
        .collect();
    let config = crate::cli::config::discovered(&root);
    let found = run(&config, &paths, Format::Human);
    found.map(|r| r.text).unwrap_or_else(|why| why)
}

/// The lines that try to talk a hazard down -- allowing the group or a
/// lint, redeclaring a class as harmless -- are REFUSED at their origin
/// (`src/rules:V117`, `src/judge:V116`), where they used to be accepted
/// and ignored, which read as a lowering that worked (B68).
#[test]
fn a_line_lowering_a_hazard_is_refused_at_its_origin() {
    let lowering = [
        ["--rule", "* any !hazard=allow"],
        ["--rule", "* any !invisible=warn"],
        ["--rule", "* any !bidi-control=deny"],
        ["--set", "hazard-invisible U+0041"],
    ];
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for [flag, value] in lowering {
        let words = crate::cli::testkit::argv(&["check", flag, value]);
        let config = crate::cli::config::from_argv(here, &words);
        let why = config.and_then(|c| c.validate()).err().unwrap_or_default();
        assert!(why.contains("argv[3]"), "{value}: {why}");
    }
}

/// The whole path, through a real `.ctrm`: a rule that grants `any`
/// and allows the charset findings does not lower the forbid (V36), and
/// the run fails.
#[test]
fn no_configuration_talks_a_hazard_down() {
    let files = [
        (".ctrm", "* any !hazard=forbid !allow\n"),
        ("smuggled.txt", "fine\u{200B}\n"),
    ];
    let Some(root) = fixture("ctrm-hazard-forbid-fixture", &files) else {
        return;
    };
    let paths = [String::from("smuggled.txt")];
    let config = crate::cli::config::discovered(&root);
    let report = run(&config, &paths, Format::Json);
    let report = report.unwrap_or_else(|why| unreachable!("{why}"));
    assert_eq!(report.code, 1, "{}", report.text);
    assert!(report.text.contains("\"invisible\""), "{}", report.text);
    assert!(report.text.contains("\"forbid\""), "{}", report.text);
}

/// `src/lint/hazard:V57`: a Persian word spelled with ZWNJ (mi-khaham, "I
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
/// ZWNJ it grants is still a hazard (`src/lint/hazard:V57`).
#[test]
fn a_redeclared_preset_excuses_no_joiner() {
    let sets = (".ctrm-sets", "hindi ascii U+200C U+200D\n");
    let files = [sets, ("a.md", "a\u{200C}b\n")];
    let got = report("ctrm-joiner-redeclared", "* hindi\n", &files);
    assert!(got.contains("U+200C"), "{got}");
}

/// V37 fixture, `crlf`: a Windows batch file, which `cmd.exe` reads
/// with CR LF endings, in a tree that grants `cr`. The lint cannot
/// know that; a later rule naming the path allows the lint there. The
/// line names no set, so it moves the level and leaves the grant alone
/// (`src/rules:V56`) -- before B5 it narrowed `.bat` back to `ascii`.
#[test]
fn a_batch_file_needs_crlf_and_says_so_per_path() {
    let files = [("build.bat", "@echo off\r\n"), ("notes.txt", "hi\r\n")];
    let base = "* ascii+cr !pedantic=warn\n";
    let on = report("ctrm-crlf-on", base, &files);
    let both = "build.bat:1:10 U+000D crlf\nnotes.txt:1:3 U+000D crlf";
    assert_eq!(on, both);
    let ctrm = format!("{base}*.bat !crlf=allow\n");
    let off = report("ctrm-crlf-off", &ctrm, &files);
    assert_eq!(off, "notes.txt:1:3 U+000D crlf");
}

/// V37 fixture, `trailing-whitespace`: two trailing spaces are a
/// Markdown HARD LINE BREAK, which is content, not noise.
#[test]
fn a_markdown_hard_break_is_trailing_whitespace_on_purpose() {
    let files = [("a.md", "one  \ntwo\n"), ("a.rs", "fn f() {} \n")];
    let base = "* ascii !pedantic=warn\n";
    let on = report("ctrm-trailing-on", base, &files);
    let rs = "a.rs:1:10 U+0020 trailing-whitespace";
    assert_eq!(on, format!("a.md:1:4 U+0020 trailing-whitespace\n{rs}"));
    let ctrm = format!("{base}*.md ascii !trailing-whitespace=allow\n");
    assert_eq!(report("ctrm-trailing-off", &ctrm, &files), rs);
}

/// V37 fixture, `final-newline`: a golden file holding this tool's
/// own human output, which ends with NO newline, byte for byte. An
/// empty file is not a finding at all.
#[test]
fn a_byte_exact_golden_file_ends_without_a_newline() {
    let golden = ("want.out", "a.rs:1:1 U+2014 ascii");
    let files = [golden, ("lib.rs", "fn f() {}"), ("empty.txt", "")];
    let base = "* ascii !pedantic=warn\n";
    let on = report("ctrm-final-on", base, &files);
    let rs = "lib.rs:1:9 U+007D final-newline";
    assert_eq!(on, format!("{rs}\nwant.out:1:21 U+0069 final-newline"));
    let ctrm = format!("{base}*.out ascii !final-newline=allow\n");
    assert_eq!(report("ctrm-final-off", &ctrm, &files), rs);
}

/// B20: one claim per character across the walks (`src/lint/pedantic:V55`). A
/// trailing space that is also the last character is one finding, and
/// so is a last character that also mixes scripts.
#[test]
fn a_character_two_walks_point_at_is_claimed_once() {
    let trailing = [("f.txt", "a ")];
    let got = report("ctrm-claim-trail", "* ascii !pedantic=warn\n", &trailing);
    assert_eq!(got, "f.txt:1:2 U+0020 trailing-whitespace");
    let mixed = [("f.txt", "a\u{03BB}")];
    let got = report("ctrm-claim-mixed", "* any !pedantic=warn\n", &mixed);
    assert_eq!(got, "f.txt:1:2 U+03BB final-newline");
}

/// V37 fixture, `unicode-space`: French typography puts a no-break
/// space before `:` and a narrow one before `!`. A project grants them
/// in a set, and allows the lint where the language wants them.
#[test]
fn french_spacing_is_a_unicode_space_on_purpose() {
    let fr = ("fr.md", "Prix\u{a0}: 5\nOui\u{202f}!\n");
    let sets = (".ctrm-sets", "french U+00A0 U+202F\n");
    let files = [sets, fr, ("a.md", "a\u{a0}b\n")];
    let base = "* ascii+french !pedantic=warn\n";
    let one = "a.md:1:2 U+00A0 unicode-space";
    let on = report("ctrm-space-on", base, &files);
    assert!(
        on.starts_with(one) && on.contains("fr.md:2:4 U+202F"),
        "{on}"
    );
    let ctrm = format!("{base}fr.md ascii+french !unicode-space=allow\n");
    assert_eq!(report("ctrm-space-off", &ctrm, &files), one);
}

/// One V58 fixture: `files` under `* any !pedantic=warn` fires `on`,
/// and adding `exempt` (a later rule naming no set) leaves `off`.
fn exempted(name: &str, files: &[(&str, &str)], exempt: &str) -> [String; 2] {
    let base = "* any !pedantic=warn\n";
    let on = report(&format!("{name}-on"), base, files);
    let ctrm = format!("{base}{exempt}\n");
    [on, report(&format!("{name}-off"), &ctrm, files)]
}

/// V58 fixture, `not-nfc`: a listing captured on macOS, whose file
/// system hands names back DECOMPOSED. The golden file must keep the
/// bytes it was given; prose elsewhere is still held to NFC.
#[test]
fn a_macos_listing_is_decomposed_on_purpose() {
    let files = [("ls.out", "cafe\u{301}.txt\n"), ("a.md", "cafe\u{301}\n")];
    let [on, off] = exempted("ctrm-nfc", &files, "*.out !not-nfc=allow");
    let md = "a.md:1:4 U+0065 not-nfc";
    assert_eq!(on, format!("{md}\nls.out:1:4 U+0065 not-nfc"));
    assert_eq!(off, md);
}

/// V58 fixture, `nfkc-compat`: SQUARE METRES are written with a
/// superscript two, and `m2` is not the same text to a reader.
#[test]
fn a_unit_superscript_is_compatibility_on_purpose() {
    let files = [("area.md", "50 m\u{b2}\n"), ("a.md", "x\u{fb01}\n")];
    let exempt = "area.md !nfkc-compat=allow";
    let [on, off] = exempted("ctrm-nfkc", &files, exempt);
    let md = "a.md:1:2 U+FB01 nfkc-compat";
    assert_eq!(on, format!("{md}\narea.md:1:5 U+00B2 nfkc-compat"));
    assert_eq!(off, md);
}

/// V58 fixture, `mixed-script`: a micrometre is Greek mu then Latin
/// m, one word in two scripts by definition. The finding points at
/// the letter that emptied the word's scripts: the `m`.
#[test]
fn a_micrometre_mixes_scripts_on_purpose() {
    let files = [("lab.md", "5 \u{3bc}m\n"), ("a.md", "p\u{3bb}y\n")];
    let exempt = "lab.md !mixed-script=allow";
    let [on, off] = exempted("ctrm-mixed", &files, exempt);
    let md = "a.md:1:2 U+03BB mixed-script";
    assert_eq!(on, format!("{md}\nlab.md:1:4 U+006D mixed-script"));
    assert_eq!(off, md);
}

/// V58 fixture, `confusable`: Russian prose. Half its alphabet is
/// drawn like Latin, so every such letter is a lookalike, and a
/// Russian file says so once, per path.
#[test]
fn russian_prose_is_confusable_on_purpose() {
    let mir = "\u{43c}\u{438}\u{440}\n";
    let files = [("ru.md", mir), ("a.md", "\u{440}\n")];
    let exempt = "ru.md !confusable=allow";
    let [on, off] = exempted("ctrm-confusable", &files, exempt);
    let md = "a.md:1:1 U+0440 confusable";
    assert_eq!(on, format!("{md}\nru.md:1:3 U+0440 confusable"));
    assert_eq!(off, md);
}
