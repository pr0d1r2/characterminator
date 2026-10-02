//! The pedantic lints: silent until asked, one claim per character, and
//! the fixtures where each one is right to be allowed (V37, V58). A
//! child of the `checker` tests, for their helpers.

use super::{any, findings, hazards, lint, report, summary};
use crate::charset::{CharSet, builtin};
use crate::cli::checker::{Judge, Looked, inspect};
use crate::lint::{Group, Level, Levels, Target};

/// One finding as `(lint name, level, byte)`.
type Fired = (&'static str, Level, usize);

/// What `check` finds in `text` under `set`, with pedantic at `warn`
/// and the charset findings at `charset`, sorted by byte.
fn pedantic(text: &str, set: &CharSet, charset: Level) -> Vec<Fired> {
    let mut levels = Levels::new();
    levels.set(Target::Group(Group::Pedantic), Level::Warn);
    levels.set_charset(charset);
    let hazards = hazards();
    let judge = Judge::new(set, &hazards, lint());
    let mut all = match inspect(text.as_bytes(), &judge, &levels) {
        Looked::Findings(all) => all.iter().map(summary).collect(),
        Looked::Unread(_) => Vec::new(),
    };
    all.sort_by_key(|(_, _, byte)| *byte);
    all
}

/// V37: the group is `allow`, so a text with every pedantic shape in
/// it says nothing until a run asks.
#[test]
fn pedantic_lints_are_silent_until_asked() {
    assert_eq!(findings("a \r\nb\u{a0}c", &any()), vec![]);
}

/// Asked, each of the four fires at the character it points at.
#[test]
fn asked_for_every_pedantic_shape_fires_once() {
    let fired = pedantic("a \r\nb\u{a0}c", &any(), Level::Deny);
    let warn = Level::Warn;
    let expected = vec![
        ("trailing-whitespace", warn, 1),
        ("crlf", warn, 2),
        ("unicode-space", warn, 5),
        ("final-newline", warn, 7),
    ];
    assert_eq!(fired, expected);
}

/// A no-break space the set does not grant is ONE finding: the set's,
/// which is the stronger claim. Allow that one, and pedantic speaks.
#[test]
fn a_space_outside_the_set_is_reported_once() {
    let ascii = builtin::ascii();
    let denied = pedantic("a\u{a0}b\n", &ascii, Level::Deny);
    assert_eq!(denied, vec![("outside-set", Level::Deny, 1)]);
    let allowed = pedantic("a\u{a0}b\n", &ascii, Level::Allow);
    assert_eq!(allowed, vec![("unicode-space", Level::Warn, 1)]);
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

/// Under plain `ascii` the CR is not granted, so a CR LF is already
/// `outside-set`; asking for pedantic does not say it twice.
#[test]
fn a_cr_the_set_refuses_is_one_finding() {
    let fired = pedantic("hi\r\n", &builtin::ascii(), Level::Deny);
    assert_eq!(fired, vec![("outside-set", Level::Deny, 2)]);
    let quiet = pedantic("hi\r\n", &builtin::ascii(), Level::Allow);
    assert_eq!(quiet, vec![("crlf", Level::Warn, 2)]);
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

/// B20: one claim per character across the walks (`src/lint:V55`). A
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

/// Asked for nothing, none of the four fires, and a character the
/// set refuses is one `outside-set` finding however many lints it
/// would also fire. The `e` the accent decomposes from is in the set, so
/// `not-nfc` still names it.
#[test]
fn the_unicode_lints_are_silent_until_asked_and_claim_once() {
    let text = "cafe\u{301} p\u{430}y \u{ff21}\n";
    assert_eq!(findings(text, &any()), vec![]);
    let ascii = builtin::ascii();
    let fired = pedantic(text, &ascii, Level::Deny);
    let deny = Level::Deny;
    let expected = vec![
        ("not-nfc", Level::Warn, 3),
        ("outside-set", deny, 4),
        ("outside-set", deny, 8),
        ("outside-set", deny, 12),
    ];
    assert_eq!(fired, expected);
}
