//! The `words` map (V51): a word is spaced off the letters beside it,
//! and nothing else is. A child of the `apply` tests, for their
//! fixtures and predicates.

use super::{PREDICATES, anything, ascii, check, fix};
use crate::fix::map::Map;
use crate::rules::Origin;

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
