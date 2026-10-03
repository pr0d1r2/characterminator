//! The tests of `lookup.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `lookup`.

use super::{LOCALES, adopt, adopt_all, line_of};
use crate::charset::builtin::{SETS, ascii, catalog};
use crate::charset::{CharSet, SetCatalog, parse_line};

const TEXT: &str = "text";

fn builtin() -> SetCatalog {
    let built = catalog();
    assert!(built.is_ok(), "the compiled-in presets must parse");
    built.unwrap_or_default()
}

/// The builtin catalog plus the locales `wanted` reaches.
fn adopted(wanted: &[&str]) -> SetCatalog {
    let mut sets = builtin();
    let names = wanted.iter().map(|name| (*name).to_owned());
    assert!(adopt(&mut sets, names).is_ok());
    sets
}

fn everything() -> SetCatalog {
    let mut sets = builtin();
    assert!(adopt_all(&mut sets).is_ok());
    sets
}

fn resolved(sets: &SetCatalog, name: &str) -> CharSet {
    let set = sets.resolve(name, TEXT);
    assert!(set.is_ok(), "{name} must resolve");
    set.unwrap_or_else(|_| CharSet::new(name.to_owned(), Vec::new()))
}

fn locale(name: &str) -> CharSet {
    resolved(&adopted(&[name]), name)
}

fn locale_names() -> Vec<String> {
    let parsed = LOCALES.lines().map(parse_line);
    let names = parsed.filter_map(|line| line.ok().flatten());
    names.map(|definition| definition.name).collect()
}

/// V61's LAZY half: the eager builtin text holds no locale, and a run
/// naming none adds none.
#[test]
fn a_run_naming_no_locale_parses_none() {
    assert!(!SETS.contains("\npl "));
    let before: Vec<String> = builtin().names().map(String::from).collect();
    let sets = adopted(&["ascii", "caveman", "hazard", "nonesuch"]);
    let after: Vec<String> = sets.names().map(String::from).collect();
    assert_eq!(before, after);
}

/// An alias brings itself and nothing else: its parent is inlined.
#[test]
fn a_named_locale_brings_only_itself() {
    let before = builtin().names().count();
    let sets = adopted(&["pt-BR"]);
    assert!(sets.get("pt-BR").is_some() && sets.get("pt").is_none());
    assert!(sets.get("pl").is_none());
    assert_eq!(sets.names().count(), before.saturating_add(1));
}

#[test]
fn a_variant_resolves_through_its_parent() {
    assert_eq!(locale("pt-BR").ranges, locale("pt").ranges);
    assert_eq!(locale("es-MX").ranges, locale("es").ranges);
    assert!(locale("pt-BR").contains('\u{00E7}'));
    assert!(locale("sr-Latn").contains('\u{0107}'));
    assert!(locale("sr").contains('\u{0436}'));
}

/// A user set named like a locale wins: only a miss is looked up.
#[test]
fn a_declared_set_wins_over_the_locale() {
    let mut sets = builtin();
    let mine = parse_line("pl U+2261").ok().flatten();
    mine.into_iter()
        .for_each(|definition| sets.insert(definition));
    assert!(adopt(&mut sets, ["pl".to_owned()]).is_ok());
    assert!(resolved(&sets, "pl").contains('\u{2261}'));
    assert!(!resolved(&sets, "pl").contains('\u{0105}'));
}

/// B33: a user set named like an alias's parent does not stand in for
/// it. `pt-BR` stays CLDR's `pt` and `en` stays the intrinsic `ascii`.
#[test]
fn an_alias_reads_its_parent_from_the_compiled_in_data() {
    let mut sets = builtin();
    for line in ["pt U+0161", "ascii U+0161"] {
        let mine = parse_line(line).ok().flatten();
        mine.into_iter().for_each(|d| sets.insert(d));
    }
    let names = ["pt-BR".to_owned(), "en".to_owned()];
    assert!(adopt(&mut sets, names).is_ok());
    assert!(resolved(&sets, "pt-BR").contains('\u{00E7}'));
    assert!(!resolved(&sets, "pt-BR").contains('\u{0161}'));
    assert_eq!(resolved(&sets, "en").ranges, ascii().ranges);
    assert!(resolved(&sets, "pt").contains('\u{0161}'));
}

/// V61: CJK, Indic and RTL are in, as are the 1B Latin locales; and a
/// loan letter sits in `-aux`, not in the main set.
const HELD: [(&str, char); 11] = [
    ("ja", '\u{3042}'),
    ("zh", '\u{4E2D}'),
    ("zh-Hant", '\u{570B}'),
    ("ko", '\u{AC00}'),
    ("hi", '\u{0915}'),
    ("ar", '\u{0628}'),
    ("he", '\u{05D0}'),
    ("pl", '\u{0104}'),
    ("de", '\u{00DF}'),
    ("tr", '\u{0130}'),
    ("pl-aux", '\u{00E4}'),
];

#[test]
fn every_script_is_covered() {
    for (name, letter) in HELD {
        assert!(locale(name).contains(letter), "{name} {letter}");
    }
    assert!(!locale("pl").contains('\u{00E4}'));
}

/// Every line resolves, and grants either letters beyond ASCII or,
/// for an all-ASCII locale, exactly `ascii` (V59).
#[test]
fn every_locale_resolves_to_letters_or_to_ascii() {
    let sets = everything();
    let names = locale_names();
    assert_eq!(names.len(), 1831);
    for name in names {
        let set = resolved(&sets, &name);
        let beyond = set.ranges.iter().all(|r| r.start > '\u{7F}');
        let plain = set.ranges == ascii().ranges;
        assert!(!set.is_empty() && (beyond || plain), "{name}");
    }
}

/// B32: an all-ASCII locale, its regional variants and its CLDR
/// default-content codes resolve, and grant ASCII and nothing more.
const PLAIN: [&str; 9] = [
    "en", "en-GB", "en-US", "en-150", "id", "ms", "ceb", "ceb-PH", "id-ID",
];

#[test]
fn an_all_ascii_locale_resolves_to_ascii() {
    for name in PLAIN {
        assert_eq!(locale(name).ranges, ascii().ranges, "{name}");
    }
    assert!(locale("en-aux").contains('\u{00E9}'));
}

/// The generator subtracts the hazard file (V59); this says it did,
/// for every locale, range against range.
#[test]
fn no_locale_holds_a_hazard() {
    let sets = everything();
    let hazard = resolved(&sets, "hazard");
    for name in locale_names() {
        let set = resolved(&sets, &name);
        let hit = set.ranges.iter().any(|r| {
            hazard
                .ranges
                .iter()
                .any(|h| r.start <= h.end && h.start <= r.end)
        });
        assert!(!hit, "{name} holds a hazard");
    }
}

/// A locale never shadows a preset, which would make `greek` mean
/// two things depending on whether a run had read the locale file.
#[test]
fn no_locale_is_named_like_a_preset() {
    let presets = builtin();
    for name in locale_names() {
        assert!(presets.get(&name).is_none(), "{name} is a preset");
    }
}

#[test]
fn the_locale_file_is_ascii_and_out_of_the_eager_text() {
    assert!(LOCALES.is_ascii());
    assert!(!SETS.contains(LOCALES));
}

/// The forward scan `line_of` used to be: the reference the index must
/// agree with (V88).
fn scanned(name: &str) -> Option<&'static str> {
    LOCALES
        .lines()
        .find(|line| line.split_once(' ').is_some_and(|(head, _)| head == name))
}

/// V88: the index finds, for every first token in the file and for the
/// names it must miss, exactly the line the scan found.
#[test]
fn the_index_finds_the_line_the_scan_found() {
    let heads = LOCALES.lines().filter_map(|l| l.split_once(' '));
    let names: Vec<&str> = heads.map(|(head, _)| head).collect();
    assert!(names.len() > 1_500, "every locale line is a probe");
    let misses = ["", "#", "nonesuch", "PL", "pt-br", "ascii", "pl "];
    for name in names.into_iter().chain(misses) {
        assert_eq!(line_of(name), scanned(name), "{name:?}");
    }
}

/// Lazy still: adopting through the index brings the same sets as
/// before, alias parents inlined, for a name reached twice over.
#[test]
fn adopting_twice_through_the_index_is_stable() {
    let once = resolved(&adopted(&["pt-BR"]), "pt-BR");
    let twice = resolved(&adopted(&["pt-BR", "pt", "pt-BR"]), "pt-BR");
    assert_eq!(once, twice);
    assert_eq!(once.ranges, locale("pt").ranges);
}
