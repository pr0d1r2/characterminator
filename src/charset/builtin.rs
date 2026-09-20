//! The builtin sets: one that is CODE, the rest DATA.
//!
//! Every preset but `ascii` ships as a data file written in the
//! `.ctrm-sets` grammar and compiled in with `include_str!` (V22), so the
//! contents a user reads and the contents the tool enforces are the same
//! bytes. `sets.ctrm-sets` is that file. The locale letter presets (V30)
//! are generated into a file of their own and are still to come with T32.
//!
//! `ascii` is the exception on purpose. `src/rules:V21` requires a run with
//! `--no-files --no-builtin-map --no-builtin-sets` to still resolve a
//! default set, so the one set that every path falls back to (`src/rules`
//! V1) cannot itself be a file that the flags remove.

use super::{
    CharRange, CharSet, ParseError, SetCatalog, SetDefinition, SetMember,
    parse_line,
};

/// The name of the intrinsic set.
pub const ASCII: &str = "ascii";

/// Tab and newline: the two control characters ASCII text is made of.
const WHITESPACE: CharRange = CharRange {
    start: '\u{0009}',
    end: '\u{000A}',
};

/// Space through tilde: every printable ASCII character.
const PRINTABLE: CharRange = CharRange {
    start: '\u{0020}',
    end: '\u{007E}',
};

/// `ascii` as a definition, so it composes like any other set (V25).
///
/// Stated in the same shape a data file would state it, so that a rule
/// naming `ascii` alongside a file-declared set takes one path through
/// resolution rather than two.
#[must_use]
pub fn ascii_definition() -> SetDefinition {
    SetDefinition {
        name: ASCII.to_owned(),
        members: vec![
            SetMember::Range(WHITESPACE),
            SetMember::Range(PRINTABLE),
        ],
    }
}

/// `ascii` resolved: printable ASCII plus tab and newline.
///
/// Carriage return is NOT here. It is the separate `cr` set, because a file
/// that grants it is making a decision about line endings rather than about
/// characters.
#[must_use]
pub fn ascii() -> CharSet {
    CharSet::new(ASCII.to_owned(), vec![WHITESPACE, PRINTABLE])
}

/// A catalog declaring the intrinsic set and nothing else.
///
/// The floor of the precedence chain in `src/rules:V19`: the builtin data
/// files, then discovered dotfiles, then flags are inserted over this, each
/// replacing a name the last one declared. Starting from `ascii` rather
/// than from nothing is what makes `--no-builtin-sets` survivable.
#[must_use]
pub fn intrinsic_catalog() -> SetCatalog {
    let mut catalog = SetCatalog::new();
    catalog.insert(ascii_definition());
    catalog
}

/// The preset data file, in the `.ctrm-sets` grammar (V22).
///
/// Public as TEXT because that is what the precedence chain takes: the
/// builtin is the lowest source in `src/rules:V19` and arrives there the
/// same way a dotfile or a `--sets-file` does. One grammar, one parser,
/// and a preset a user overrides by declaring the name again.
pub const SETS: &str = include_str!("sets.ctrm-sets");

/// The presets the data file declares, unresolved.
///
/// Members stay unresolved for the reason [`SetDefinition`] states: a
/// preset here may name another, and `any` would otherwise expand in
/// every catalog that never uses it.
///
/// # Errors
///
/// Returns a [`ParseError`] if a line of the compiled-in file cannot be
/// read. That is a defect in THIS crate, not in anyone's configuration --
/// the test below is what keeps it from shipping -- but it is returned
/// rather than panicked, because a library that kills the process leaves
/// its caller no way to say which set was wrong.
pub fn definitions() -> Result<Vec<SetDefinition>, ParseError> {
    SETS.lines()
        .map(parse_line)
        .filter_map(Result::transpose)
        .collect()
}

/// The intrinsic set plus every preset the data file declares.
///
/// This is the floor of `src/rules:V19` with the builtin source present,
/// as [`intrinsic_catalog`] is the floor with it removed. The data file
/// is inserted OVER `ascii`, which costs nothing today and is the right
/// order the moment the file has anything to say about it.
///
/// # Errors
///
/// As [`definitions`].
pub fn catalog() -> Result<SetCatalog, ParseError> {
    let mut catalog = intrinsic_catalog();
    for definition in definitions()? {
        catalog.insert(definition);
    }
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::{
        ASCII, CharSet, SETS, SetDefinition, SetMember, ascii,
        ascii_definition, catalog, definitions, intrinsic_catalog,
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
    const DECLARED: [&str; 13] = [
        "any",
        "arabic",
        "box",
        "caveman",
        "cr",
        "cyrillic",
        "emoji",
        "greek",
        "latin-ext",
        "latin1",
        "legal",
        "marks",
        "math",
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
    fn is_code_point(member: &SetMember) -> bool {
        match member {
            SetMember::Range(_) => true,
            SetMember::Labelled { member, .. } => is_code_point(member),
            SetMember::Literal(_) | SetMember::Named(_) => false,
        }
    }

    #[test]
    fn every_member_is_written_as_a_code_point() {
        for definition in declared() {
            for member in &definition.members {
                assert!(
                    is_code_point(member),
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
}
