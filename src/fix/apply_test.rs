//! The tests of `apply.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `apply`.

use super::{Cut, Fixed, Pass, Span, check, fix, untouched_bytes_match};
use crate::fix::map::Map;
use crate::fix::{Error, ROOT};
use crate::rules::Origin;
use crate::scan::{Hit, Position};

/// A map in the shape the builtin will have (V26, V31): typography to
/// ASCII, one explicit delete, and one declared sequence whose prefix is
/// also declared.
const MAP: &str = "\
        # em dash, en dash, ellipsis, no-break space, zero width space.\n\
        U+2014 --\n\
        U+2013 -\n\
        U+2026 ...\n\
        U+00A0 U+0020\n\
        U+200B\n\
        # a sequence, its prefix, and a sequence that opens with ASCII.\n\
        U+1F44D+U+1F3FD U+1F44D\n\
        U+1F44D +1\n\
        U+0031+U+20E3 1\n";

/// The inputs the properties below are asserted over. Each one is here
/// for a reason a random generator would not reliably produce:
/// the empty text; a text with nothing to do; a violation in the middle,
/// alone, and doubled with no gap; a declared sequence whose prefix is
/// also declared; that prefix on its own; a sequence whose FIRST code
/// point is allowed, which a per-character check would miss; a violation
/// with no mapping at all (V4); an explicit delete, which shortens the
/// text; a replacement written in code point form; a violation on the
/// second line, which only a position bug would misplace; and a mapped
/// violation between two multi-byte characters that have no mapping,
/// which is where a byte-sloppy copy shows up.
const INPUTS: &[&str] = &[
    "",
    "plain ascii, nothing to do\n",
    "a\u{2014}b",
    "\u{2014}",
    "\u{2014}\u{2013}",
    "a\u{1F44D}\u{1F3FD}b",
    "a\u{1F44D}b",
    "1\u{20E3}",
    "a\u{2764}b",
    "a\u{200B}b",
    "a\u{00A0}b",
    "one\n\u{2014}\ntwo\n",
    "\u{00E9}\u{2014}\u{00E9}",
];

/// What each input becomes when only ASCII is allowed.
const WANT: &[&str] = &[
    "",
    "plain ascii, nothing to do\n",
    "a--b",
    "--",
    "---",
    "a+1b",
    "a+1b",
    "1",
    "a\u{2764}b",
    "ab",
    "a b",
    "one\n--\ntwo\n",
    "\u{00E9}--\u{00E9}",
];

fn ascii(ch: char) -> bool {
    ch.is_ascii()
}

fn ascii_or_em_dash(ch: char) -> bool {
    ch.is_ascii() || ch == '\u{2014}'
}

fn anything(_: char) -> bool {
    true
}

type Predicate = fn(char) -> bool;

const PREDICATES: &[Predicate] = &[ascii, ascii_or_em_dash, anything];

fn map() -> Map {
    Map::parse(MAP, &|line| Origin::Builtin { line }).unwrap_or_default()
}

/// A span standing at `byte`, for the guard tests below. Which character
/// it names does not matter to either guard; the byte length does.
fn span(byte: usize, len: usize, to: &str) -> Span {
    Span {
        hit: Hit {
            position: Position {
                line: 1,
                column: byte.saturating_add(1),
                byte,
            },
            character: '\u{2014}',
        },
        len,
        to: String::from(to),
    }
}

fn pass_of(output: &str, spans: Vec<Span>) -> Pass {
    Pass {
        output: String::from(output),
        spans,
        unmapped: Vec::new(),
    }
}

fn output(text: &str, allowed: &dyn Fn(char) -> bool) -> String {
    fix(text, &map(), allowed)
        .map(|fixed| fixed.output)
        .unwrap_or_else(|_| String::from("<error>"))
}

#[test]
fn the_fixture_map_parses() {
    assert!(Map::parse(MAP, &|line| Origin::Builtin { line }).is_ok());
    assert_eq!(INPUTS.len(), WANT.len());
}

#[test]
fn every_input_is_rewritten_as_declared() {
    for (input, want) in INPUTS.iter().zip(WANT.iter()) {
        assert_eq!(&output(input, &ascii), want, "input {input:?}");
    }
}

#[test]
fn fix_is_idempotent() {
    for input in INPUTS {
        for allowed in PREDICATES {
            let once = output(input, allowed);
            let twice = output(&once, allowed);
            assert_eq!(twice, once, "input {input:?}");
        }
    }
}

#[test]
fn an_allowed_text_keeps_every_byte() {
    for input in INPUTS {
        assert_eq!(&output(input, &anything), input, "input {input:?}");
    }
}

#[test]
fn an_empty_map_keeps_every_byte() {
    for input in INPUTS {
        let fixed = fix(input, &Map::default(), &ascii);
        let text = fixed.map(|done| done.output).unwrap_or_default();
        assert_eq!(&text, input, "input {input:?}");
    }
}

#[test]
fn a_granted_character_is_left_alone() {
    let text = "a\u{2014}\u{2013}b";
    assert_eq!(output(text, &ascii_or_em_dash), "a\u{2014}-b");
}

#[test]
fn a_character_with_no_mapping_is_kept_and_reported() {
    let report = check("a\u{2764}b", &map(), &ascii).unwrap_or_default();
    let found = report.unmapped.first().copied();
    assert_eq!(found.map(|hit| hit.character), Some('\u{2764}'));
    assert_eq!(found.map(|hit| hit.position.column), Some(2));
    assert!(!report.drifted());
}

#[test]
fn the_longest_declared_sequence_wins_and_chains() {
    let text = "a\u{1F44D}\u{1F3FD}b";
    let report = check(text, &map(), &ascii).unwrap_or_default();
    assert_eq!(report.rewrites.len(), 1);
    assert_eq!(
        report.rewrites.first().map(|done| done.to.as_str()),
        Some("+1")
    );
    assert!(report.drifted());
}

#[test]
fn a_sequence_opening_with_an_allowed_character_is_a_violation() {
    let report = check("1\u{20E3}", &map(), &ascii).unwrap_or_default();
    assert_eq!(report.rewrites.len(), 1);
    assert_eq!(report.unmapped.len(), 0);
}

#[test]
fn a_rewrite_is_reported_where_it_was_found() {
    let report =
        check("one\n\u{2014}\ntwo\n", &map(), &ascii).unwrap_or_default();
    let at = report.rewrites.first().map(|done| done.hit.position);
    assert_eq!(
        at.map(|pos| (pos.line, pos.column, pos.byte)),
        Some((2, 1, 4))
    );
}

#[test]
fn a_cyclic_map_is_an_error_rather_than_a_half_rewrite() {
    let source = "U+2014 U+2013\nU+2013 U+2014\n";
    let cyclic = Map::parse(source, &|line| Origin::Builtin { line })
        .unwrap_or_default();
    assert_eq!(fix("\u{2014}", &cyclic, &ascii), Err(Error::MapCycle));
}

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
    let done = fix("a\u{2713}b", &classed("emoji"), &ascii);
    assert_eq!(done.map(|f| f.output).unwrap_or_default(), "a[x]b");
}

#[test]
fn a_class_compresses_into_the_fidelity_family() {
    let done = fix("\u{2713}", &classed("emoji"), &ascii_or_emoji);
    assert_eq!(done.map(|f| f.output).unwrap_or_default(), "\u{2705}");
}

#[test]
fn a_declared_family_takes_part_in_resolution() {
    let done = fix("\u{2713}", &classed("nerd"), &ascii_or_nerd);
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
    let done = fix("\u{2713}", &map, &ascii).unwrap_or_default();
    assert_eq!(done.output, "\u{2713}");
}

#[test]
fn check_reports_without_handing_back_text_to_write() {
    let report = check("a\u{2014}b", &map(), &ascii).unwrap_or_default();
    assert!(report.drifted());
    assert_eq!(report.rewrites.len(), 1);
}

/// The V6 guard has to be able to FAIL, or asserting it proves nothing.
#[test]
fn the_untouched_guard_catches_a_changed_byte() {
    let good = pass_of("a--b", vec![span(1, 3, "--")]);
    assert!(untouched_bytes_match("a\u{2014}b", &good));
    let bad = pass_of("a--B", vec![span(1, 3, "--")]);
    assert!(!untouched_bytes_match("a\u{2014}b", &bad));
}

#[test]
fn a_fix_with_nothing_to_do_reports_nothing() {
    let done = fix("plain", &map(), &ascii).unwrap_or_default();
    assert_eq!(done, Fixed::default_with("plain"));
}

#[test]
fn the_cut_reads_both_sides_in_step() {
    let mut cut = Cut::default();
    assert!(cut.gap_matches("abc", "abCC", 2));
    cut.skip(&span(2, 1, "CC"));
    assert!(cut.tail_matches("abc", "abCC"));
}

/// A map in the shape of the `words` map (V51), plus the explicit
/// delete, so a delete between a word and a letter is exercised.
const WORDS: &str = "\
        word U+22A5 not\n\
        word U+2200 all\n\
        U+2192 ->\n\
        U+200B\n";

/// Each input is here for a boundary a word could fuse across: a
/// letter after, a letter before, both, a digit and an underscore
/// (`\w`), a letter in another script, two words in a row, a word
/// beside punctuation, a word already spaced, a delete standing
/// between the word and the letter, an arrow that is no word at all,
/// and a line break.
const WORD_INPUTS: &[&str] = &[
    "\u{22A5}owns",
    "x\u{22A5}",
    "a\u{22A5}b",
    "1\u{22A5}_x",
    "\u{22A5}\u{017C}\u{00F3}\u{0142}w",
    "\u{22A5}\u{2200}",
    "(\u{22A5})",
    "a \u{22A5} b",
    "\u{22A5}\u{200B}x",
    "a\u{2192}b",
    "\u{22A5}\nx",
];

/// What each becomes when only ASCII is allowed.
const WORD_WANT: &[&str] = &[
    "not owns",
    "x not",
    "a not b",
    "1 not _x",
    "not \u{017C}\u{00F3}\u{0142}w",
    "not all",
    "(not)",
    "a not b",
    "not x",
    "a->b",
    "not\nx",
];

fn words() -> Map {
    Map::parse(WORDS, &|line| Origin::Builtin { line }).unwrap_or_default()
}

fn worded(text: &str, allowed: &dyn Fn(char) -> bool) -> String {
    fix(text, &words(), allowed)
        .map(|fixed| fixed.output)
        .unwrap_or_else(|_| String::from("<error>"))
}

#[test]
fn a_word_is_kept_apart_from_its_neighbours() {
    assert_eq!(WORD_INPUTS.len(), WORD_WANT.len());
    for (input, want) in WORD_INPUTS.iter().zip(WORD_WANT.iter()) {
        assert_eq!(&worded(input, &ascii), want, "input {input:?}");
    }
}

/// V5 and V6 over the word inputs, under every predicate: `fix` runs
/// both guards before it returns, so an `Ok` is the V6 half and the
/// second run is the V5 half.
#[test]
fn a_spaced_word_is_idempotent_and_touches_nothing_else() {
    for input in WORD_INPUTS {
        for allowed in PREDICATES {
            let once = worded(input, allowed);
            assert_ne!(once, "<error>", "input {input:?}");
            assert_eq!(worded(&once, allowed), once, "input {input:?}");
        }
    }
}

#[test]
fn an_allowed_word_source_is_left_alone() {
    for input in WORD_INPUTS {
        assert_eq!(&worded(input, &anything), input, "input {input:?}");
    }
}

/// The inserted space is part of the REPLACEMENT (V51), so the report
/// says what was written where: before the word when the letter came
/// first, after it when the letter follows.
#[test]
fn the_space_is_reported_as_part_of_the_rewrite() {
    let to = |text: &str| {
        let report = check(text, &words(), &ascii).unwrap_or_default();
        report.rewrites.first().map(|done| done.to.clone())
    };
    assert_eq!(to("x\u{22A5}"), Some(String::from(" not")));
    assert_eq!(to("\u{22A5}x"), Some(String::from("not ")));
    assert_eq!(to("a\u{22A5}b"), Some(String::from(" not ")));
}

/// A plain entry is never spaced, whatever it lands next to: a
/// transliteration INSIDE a word (`caf\u{e9}` to `cafe`) must not
/// be split by a rule that exists for words.
#[test]
fn a_plain_entry_is_never_spaced() {
    let map = Map::parse("U+00E9 e\n", &|line| Origin::Builtin { line })
        .unwrap_or_default();
    let done = fix("caf\u{00E9}s", &map, &ascii).unwrap_or_default();
    assert_eq!(done.output, "cafes");
}

/// The builtin map's emoji sequences (V62), run through the engine.
mod sequences {
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
        let allowed =
            |c: char| c.is_ascii() || grant_emoji && EMOJI.contains(c);
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
        for (text, grant) in texts.flat_map(|t| [(t.clone(), false), (t, true)])
        {
            let once = fixed(&text, grant);
            assert!(!once.starts_with('<'), "{text:?}: {once}");
            assert_eq!(fixed(&once, grant), once, "{text:?}");
        }
    }
}

impl Fixed {
    fn default_with(text: &str) -> Self {
        Self {
            output: String::from(text),
            ..Self::default()
        }
    }
}
