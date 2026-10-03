//! The tests of `builtin.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `builtin`.

use super::{
    ASCII, CharSet, HAZARD, SETS, SetDefinition, SetMember, ascii,
    ascii_definition, catalog, definitions, hazard_catalog, intrinsic_catalog,
};

#[test]
fn ascii_grants_printable_text() {
    let set = ascii();
    assert!(set.contains('\u{0020}'));
    assert!(set.contains('\u{0041}'));
    assert!(set.contains('\u{007E}'));
}

#[test]
fn ascii_grants_tab_and_newline() {
    let set = ascii();
    assert!(set.contains('\u{0009}'));
    assert!(set.contains('\u{000A}'));
}

#[test]
fn ascii_withholds_carriage_return_and_other_controls() {
    let set = ascii();
    assert!(!set.contains('\u{000D}'));
    assert!(!set.contains('\u{0000}'));
    assert!(!set.contains('\u{001F}'));
    assert!(!set.contains('\u{007F}'));
}

#[test]
fn ascii_withholds_everything_above_ascii() {
    let set = ascii();
    assert!(!set.contains('\u{00A0}'));
    assert!(!set.contains('\u{2014}'));
    assert!(!set.contains('\u{1F600}'));
}

#[test]
fn the_definition_resolves_to_the_same_set() {
    assert_eq!(ascii_definition().name, ASCII);
    assert_eq!(ascii().ranges.len(), 2);
}

#[test]
fn the_intrinsic_catalog_composes_the_same_set() {
    assert_eq!(intrinsic_catalog().resolve(ASCII, TEXT), Ok(ascii()));
}

#[test]
fn the_intrinsic_catalog_declares_only_ascii() {
    let catalog = intrinsic_catalog();
    let listed = catalog.names().collect::<Vec<_>>();
    assert_eq!(listed, vec![ASCII]);
}

/// The default fidelity (`src/rules:V29`), spelled rather than
/// imported: a family is an opaque name to this node (V41).
const TEXT: &str = "text";

/// The presets `src/charset/SPEC.md` INTERFACES names, which this
/// file ships.
///
/// Written out rather than read back from the catalog: a test that
/// asked the data file what it declares would pass just as happily
/// after a preset was deleted from it.
const DECLARED: [&str; 16] = [
    "any",
    "arabic",
    "box",
    "caveman",
    "cr",
    "cyrillic",
    "emoji",
    "greek",
    "hazard",
    "hindi",
    "latin-ext",
    "latin1",
    "legal",
    "marks",
    "math",
    "persian",
    // `typography` is tested by name below rather than listed here:
    // the array length is the count this file promises, and a preset
    // added without a test is what that count is for.
];

fn declared() -> Vec<SetDefinition> {
    let parsed = definitions();
    assert!(parsed.is_ok(), "the compiled-in data file must parse");
    parsed.unwrap_or_default()
}

fn preset(name: &str) -> CharSet {
    preset_at(name, TEXT)
}

/// The same, at a fidelity the caller names (V41).
fn preset_at(name: &str, family: &str) -> CharSet {
    let resolved = catalog()
        .ok()
        .and_then(|sets| sets.resolve(name, family).ok());
    assert!(resolved.is_some(), "{name} must resolve");
    resolved.unwrap_or_else(|| CharSet::new(name.to_owned(), Vec::new()))
}

#[test]
fn the_data_file_parses() {
    assert!(definitions().is_ok());
}

#[test]
fn the_data_file_is_pure_ascii() {
    assert!(SETS.is_ascii());
}

/// V22: every member is `U+XXXX`, which is what keeps the file ASCII.
/// A fidelity label is allowed in front of one (V41) and carries no
/// character of its own, so it is followed rather than rejected.
///
/// So is a member NAMING another builtin set (V25), which is how the
/// generated `hazard` is the union of its classes rather than a second
/// copy of their ranges. Only a name the builtin text itself declares:
/// one reaching outside it would make a preset depend on whatever a
/// user happened to declare.
fn is_code_point(member: &SetMember, builtin: &[String]) -> bool {
    match member {
        SetMember::Range(_) => true,
        SetMember::Labelled { member, .. } => is_code_point(member, builtin),
        SetMember::Named(name) => builtin.contains(name),
        SetMember::Literal(_) => false,
    }
}

#[test]
fn every_member_is_written_as_a_code_point() {
    let names: Vec<String> = declared().into_iter().map(|d| d.name).collect();
    for definition in declared() {
        for member in &definition.members {
            assert!(
                is_code_point(member, &names),
                "{} holds a member that is not U+XXXX",
                definition.name
            );
        }
    }
}

#[test]
fn every_preset_the_spec_names_is_declared_and_grants_something() {
    for name in DECLARED {
        assert!(!preset(name).is_empty(), "{name} grants nothing");
    }
    assert!(!preset("typography").is_empty());
}

#[test]
fn the_data_file_declares_no_set_twice() {
    let mut names: Vec<String> =
        declared().into_iter().map(|set| set.name).collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total);
}

#[test]
fn the_data_file_does_not_declare_ascii() {
    let ascii_declared = declared().iter().any(|set| set.name == ASCII);
    assert!(!ascii_declared);
}

#[test]
fn the_catalog_carries_ascii_as_well_as_the_presets() {
    assert_eq!(preset(ASCII), ascii());
    assert!(catalog().is_ok_and(|sets| sets.get("caveman").is_some()));
}

#[test]
fn caveman_grants_the_format_symbols() {
    let set = preset("caveman");
    assert!(set.contains('\u{2192}'));
    assert!(set.contains('\u{22A5}'));
    assert!(set.contains('\u{00A7}'));
}

/// R3: stopping at FORMAT.md's own list misses 32% of the files.
#[test]
fn caveman_grants_the_measured_symbols_too() {
    let set = preset("caveman");
    assert!(set.contains('\u{21D2}'));
    assert!(set.contains('\u{2235}'));
    assert!(set.contains('\u{00B7}'));
    assert!(!set.contains('\u{1F600}'));
}

#[test]
fn box_grants_both_drawing_blocks() {
    let set = preset("box");
    assert!(set.contains('\u{250C}'));
    assert!(set.contains('\u{257F}'));
    assert!(set.contains('\u{25A0}'));
    assert!(!set.contains('\u{2580}'));
}

#[test]
fn typography_grants_what_the_builtin_map_targets() {
    let set = preset("typography");
    assert!(set.contains('\u{2014}'));
    assert!(set.contains('\u{2019}'));
    assert!(set.contains('\u{201D}'));
    assert!(set.contains('\u{2026}'));
    assert!(set.contains('\u{00A0}'));
    assert!(set.contains('\u{2212}'));
}

#[test]
fn emoji_grants_single_code_points_and_withholds_the_joiners() {
    let set = preset("emoji");
    assert!(set.contains('\u{1F600}'));
    assert!(set.contains('\u{2600}'));
    assert!(!set.contains('\u{200D}'));
    assert!(!set.contains('\u{FE0F}'));
    assert!(!set.contains('\u{1F3FB}'));
    assert!(!set.contains('\u{20E3}'));
}

#[test]
fn cr_grants_only_the_carriage_return() {
    let set = preset("cr");
    assert!(set.contains('\u{000D}'));
    assert_eq!(set.ranges.len(), 1);
}

#[test]
fn a_coarse_block_grants_its_whole_block() {
    assert!(preset("latin1").contains('\u{00E9}'));
    assert!(preset("latin-ext").contains('\u{0142}'));
    assert!(preset("cyrillic").contains('\u{0416}'));
    assert!(preset("greek").contains('\u{03B1}'));
    assert!(preset("arabic").contains('\u{0645}'));
}

#[test]
fn any_grants_every_code_point_there_is() {
    let set = preset("any");
    assert!(set.contains('\u{0000}'));
    assert!(set.contains('\u{E000}'));
    assert!(set.contains('\u{10FFFF}'));
}

#[test]
fn a_preset_is_a_handful_of_ranges() {
    for name in DECLARED {
        let ranges = preset(name).ranges.len();
        assert!(ranges <= 32, "{name} holds {ranges} ranges");
    }
}
/// The preset that needed V41: one name, two spellings, and only the
/// fidelity in force is granted.
#[test]
fn marks_grants_the_text_spelling_at_text_fidelity() {
    let set = preset("marks");
    // CHECK MARK and BALLOT X, the `text` members.
    assert!(set.contains('\u{2713}'));
    assert!(set.contains('\u{2717}'));
    // WHITE HEAVY CHECK MARK and CROSS MARK, which are `emoji`.
    assert!(!set.contains('\u{2705}'));
    assert!(!set.contains('\u{274C}'));
}

#[test]
fn marks_grants_the_emoji_spelling_at_emoji_fidelity() {
    let set = preset_at("marks", "emoji");
    assert!(set.contains('\u{2705}'));
    assert!(set.contains('\u{274C}'));
    assert!(!set.contains('\u{2713}'));
    assert!(!set.contains('\u{2717}'));
}

/// An unlabelled member belongs to every fidelity: WARNING SIGN is the
/// same code point in both spellings, so labelling it twice would say
/// there were two of it.
#[test]
fn an_unlabelled_member_survives_every_fidelity() {
    assert!(preset("marks").contains('\u{26A0}'));
    assert!(preset_at("marks", "emoji").contains('\u{26A0}'));
    assert!(preset_at("marks", "nerd").contains('\u{26A0}'));
}

/// An UNKNOWN family grants the unlabelled members and nothing else.
/// No walk up the family tree happens here (V41): that tree lives in
/// the map, and a set that followed it would make this node depend on
/// `src/fix`.
#[test]
fn an_unknown_family_grants_only_what_carries_no_label() {
    let set = preset_at("marks", "nerd");
    assert_eq!(set.ranges.len(), 1);
    assert!(!set.contains('\u{2713}'));
    assert!(!set.contains('\u{2705}'));
}

/// The classes the generated hazard file declares, in the order the
/// lint node reads them. Spelled out for the reason [`DECLARED`] is.
const HAZARD_CLASSES: [&str; 5] = [
    "hazard-bidi",
    "hazard-tag",
    "hazard-bom",
    "hazard-control",
    "hazard-invisible",
];

fn hazard_class(name: &str) -> CharSet {
    let resolved = hazard_catalog()
        .ok()
        .and_then(|sets| sets.resolve(name, TEXT).ok());
    assert!(resolved.is_some(), "{name} must resolve on its own");
    resolved.unwrap_or_else(|| CharSet::new(name.to_owned(), Vec::new()))
}

/// How many code points a set holds, counted rather than computed so
/// no arithmetic is needed to say it.
fn size(set: &CharSet) -> usize {
    set.ranges.iter().map(|r| (r.start..=r.end).count()).sum()
}

#[test]
fn the_hazard_file_is_ascii_and_reads_without_the_presets() {
    assert!(HAZARD.is_ascii());
    assert!(SETS.ends_with(HAZARD));
    for name in HAZARD_CLASSES {
        assert!(!hazard_class(name).is_empty(), "{name} is empty");
    }
    let listed = hazard_catalog().map(|c| c.names().count());
    assert_eq!(listed, Ok(6));
}

/// The generated line is checked against the figure upstream prints
/// beneath it ("Total code points: 4174" in DerivedCoreProperties
/// 18.0.0), so a member lost or mistyped in a regeneration shows here.
#[test]
fn the_ignorable_class_holds_exactly_what_unicode_counts() {
    assert_eq!(size(&hazard_class("hazard-invisible")), 4174);
}

/// V34's members, at least one per class it names: ZWSP, ZWJ, soft
/// hyphen, VS16, a Hangul filler, an invisible math operator; RLO and
/// LRI; TAG A; BEL, ESC, DELETE and a C1 control; the BOM.
const V34_MEMBERS: &str = "\u{200B}\u{200D}\u{00AD}\u{FE0F}\u{3164}\
        \u{2061}\u{202E}\u{2066}\u{E0041}\u{0007}\u{001B}\u{007F}\
        \u{009B}\u{FEFF}";

/// ... and the three controls it keeps.
#[test]
fn hazard_holds_every_class_v34_names_and_keeps_the_layout_controls() {
    let set = preset("hazard");
    for point in V34_MEMBERS.chars() {
        assert!(set.contains(point), "U+{:04X}", u32::from(point));
    }
    for point in ['\t', '\n', '\r', ' ', 'A', '\u{00A0}', '\u{1F3FD}'] {
        assert!(!set.contains(point), "U+{:04X}", u32::from(point));
    }
}

/// The catalog's `hazard` and the one the lint node reads are the same
/// bytes, so `ctrm sets` lists what actually fires.
#[test]
fn the_catalog_hazard_is_the_hazard_the_lint_reads() {
    assert_eq!(preset("hazard"), hazard_class("hazard"));
}

/// A bidi override is classed as bidi and not only as invisible: the
/// narrow classes are what name the lint, so each must hold its own.
#[test]
fn each_hazard_class_holds_its_own_members() {
    assert!(hazard_class("hazard-bidi").contains('\u{202E}'));
    assert!(hazard_class("hazard-bidi").contains('\u{200F}'));
    assert!(!hazard_class("hazard-bidi").contains('\u{200B}'));
    assert_eq!(size(&hazard_class("hazard-tag")), 128);
    assert_eq!(size(&hazard_class("hazard-bom")), 1);
    assert_eq!(size(&hazard_class("hazard-control")), 62);
}

/// `SetCatalog::builtin` is infallible only because both halves of it
/// parse: the presets and every locale. Its fallback is never taken.
#[test]
fn the_library_catalog_holds_presets_and_locales() {
    let mut built = catalog().unwrap_or_default();
    assert!(super::super::locale::adopt_all(&mut built).is_ok());
    let library = super::SetCatalog::builtin();
    assert_eq!(library, built);
    assert!(library.resolve("caveman", "text").is_ok());
    assert!(library.resolve("pl", "text").is_ok());
}
