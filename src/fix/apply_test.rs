//! The tests of `apply.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `apply`.
//!
//! This file holds the shared fixtures, the properties every input is
//! held to (V5, V6), and the guards. Three subjects are child modules
//! of this one, each in its own file and reaching these fixtures as
//! `super`: rewriting through classes and families, the `words` map's
//! spacing (V51), and the builtin emoji sequences with the multi-pass
//! fix they need (V62, V65).

use super::{Cut, Fixed, Pass, Span, check, fix, untouched_bytes_match};
use crate::fix::Error;
use crate::fix::map::Map;
use crate::rules::Origin;
use crate::scan::{Hit, Position};

#[path = "apply_class_test.rs"]
mod classes;
#[path = "apply_sequences_test.rs"]
mod sequences;
#[path = "apply_words_test.rs"]
mod words;

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

/// B40: a target outside the set leaves the character unmapped (V4),
/// and it did not -- the replacement was never judged, so `fix` wrote
/// a disallowed character, reported nothing, and `check` then failed it.
#[test]
fn a_target_outside_the_set_keeps_the_character_and_reports_it() {
    let lone = Map::parse("U+2261 U+2295\n", &|line| Origin::Builtin { line });
    let done = fix("c\u{2261}d", &lone.unwrap_or_default(), &ascii);
    let done = done.unwrap_or_default();
    assert_eq!(done.output, "c\u{2261}d");
    assert!(!done.report.drifted());
    let kept = done.report.unmapped.first().map(|hit| hit.character);
    assert_eq!(kept, Some('\u{2261}'));
}

/// The judgement is of the WHOLE chain: a target outside the set that the
/// map rewrites again, into the set, is still a rewrite (V5).
#[test]
fn a_chain_that_ends_inside_the_set_still_rewrites() {
    let source = "U+2261 U+2295\nU+2295 x\n";
    let chain = Map::parse(source, &|line| Origin::Builtin { line });
    let done = fix("c\u{2261}d", &chain.unwrap_or_default(), &ascii);
    assert_eq!(done.map(|fixed| fixed.output), Ok(String::from("cxd")));
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

/// `Layer::back` is a binary search over the spans; this is the walk it
/// replaced, asked of every output byte of a layer that grows, shrinks
/// and deletes, back to back.
#[test]
fn a_layer_maps_every_output_byte_back_as_the_walk_did() {
    let spans = vec![span(1, 3, "--"), span(4, 3, ""), span(7, 2, "xyz")];
    let layer = super::Layer::of(&spans);
    for byte in 0..16 {
        assert_eq!(layer.back(byte), walked(byte, &spans), "byte {byte}");
    }
}

/// The per-span walk `Layer::back` replaced, kept as its oracle.
fn walked(byte: usize, layer: &[Span]) -> usize {
    let mut cut = Cut::default();
    for span in layer {
        let start = cut.start_of(span);
        if byte < start {
            break;
        }
        if byte < start.saturating_add(span.to.len()) {
            return span.hit.position.byte;
        }
        cut.skip(span);
    }
    cut.input.saturating_add(byte.saturating_sub(cut.output))
}

impl Fixed {
    fn default_with(text: &str) -> Self {
        Self {
            output: String::from(text),
            ..Self::default()
        }
    }
}
