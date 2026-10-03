//! V104: a hazard is rewritten whatever the set grants, and V5 and V6
//! still hold. Property-tested over a generated corpus: every text drawn
//! from an alphabet of plain letters, mapped characters, a declared
//! sequence, the hazards `fix` deletes and the one class it may not.

use super::{PREDICATES, anything, ascii, map};
use crate::fix::apply::fix_under;
use crate::fix::{Hazard, Law};

/// The hazards this test's law reports. Which ones are hazards is the
/// judge's answer in the binary; here it is a fixed list, so the engine
/// is tested on its own.
const DELETED: &[char] = &['\u{202E}', '\u{200B}', '\u{E0041}', '\u{2066}'];
const CONTROL: char = '\u{7}';

const ALPHABET: &[char] = &[
    'a',
    'b',
    ' ',
    '\n',
    '\u{2014}',
    '\u{1F44D}',
    '\u{1F3FD}',
    '\u{00E9}',
    '\u{2764}',
    '\u{202E}',
    '\u{200B}',
    '\u{E0041}',
    '\u{2066}',
    CONTROL,
];

fn hazards(text: &str) -> Vec<Hazard> {
    let hazard = |(byte, ch): (usize, char)| {
        let delete = DELETED.contains(&ch);
        (delete || ch == CONTROL).then_some(Hazard { byte, delete })
    };
    text.char_indices().filter_map(hazard).collect()
}

fn fixed(text: &str, allowed: &dyn Fn(char) -> bool) -> String {
    let law = Law {
        allowed,
        hazards: &hazards,
    };
    fix_under(text, &map(), law)
        .map(|fixed| fixed.output)
        .unwrap_or_else(|_| String::from("<error>"))
}

/// A deterministic corpus: a linear congruential walk over the alphabet,
/// so a failure names an input that reproduces.
fn corpus() -> Vec<String> {
    let mut seed: u64 = 104;
    let mut next = move |below: usize| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let drawn = usize::try_from(seed >> 33).unwrap_or_default();
        drawn.checked_rem(below).unwrap_or_default()
    };
    let mut text = || {
        let len = next(12);
        let pick = |_| ALPHABET.get(next(ALPHABET.len())).copied();
        (0..len).filter_map(pick).collect()
    };
    (0..400).map(|_| text()).collect()
}

/// V5: a second fix under the same law changes nothing.
#[test]
fn fix_under_a_law_with_hazards_is_idempotent() {
    for text in corpus() {
        for allowed in PREDICATES {
            let once = fixed(&text, allowed);
            assert_eq!(fixed(&once, allowed), once, "input {text:?}");
        }
    }
}

/// V6 restated: where the set grants every character, the ONLY change is
/// the hazards that draw nothing, gone. Every other byte, the control
/// character included, is where it was.
#[test]
fn a_granted_text_loses_its_hazards_and_nothing_else() {
    for text in corpus() {
        let want: String =
            text.chars().filter(|c| !DELETED.contains(c)).collect();
        assert_eq!(fixed(&text, &anything), want, "input {text:?}");
    }
}

/// Under any set, no hazard that draws nothing survives, and the control
/// character -- which `ascii` grants and no map entry covers -- is kept.
#[test]
fn no_deleted_hazard_survives_and_the_control_stays() {
    for text in corpus() {
        let out = fixed(&text, &ascii);
        assert!(!out.chars().any(|c| DELETED.contains(&c)), "{text:?}");
        let bells = |s: &str| s.chars().filter(|c| *c == CONTROL).count();
        assert_eq!(bells(&out), bells(&text), "input {text:?}");
    }
}

/// A hazard the map covers is rewritten BY the map, granted or not: the
/// fixture maps the zero width space to a delete, and a `.ctrm-map` line
/// for a control character is how a project removes one.
#[test]
fn a_mapped_hazard_follows_the_map_even_where_granted() {
    let bell = crate::fix::Map::parse("U+0007 !\n", &|line| {
        crate::rules::Origin::Builtin { line }
    })
    .unwrap_or_default();
    let law = Law {
        allowed: &anything,
        hazards: &hazards,
    };
    let out = fix_under("a\u{7}b", &bell, law).map(|f| f.output);
    assert_eq!(out.unwrap_or_default(), "a!b");
}

/// The plain `fix` knows no hazard: its predicate is the whole law, as
/// before V104, so a library caller that grants a character keeps it.
#[test]
fn the_plain_fix_knows_no_hazard() {
    let text = "a\u{202E}b";
    let out = super::fix(text, &map(), anything).map(|f| f.output);
    assert_eq!(out.unwrap_or_default(), text);
}
