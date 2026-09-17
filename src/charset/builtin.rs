//! The sets that are CODE rather than data.
//!
//! Only `ascii` lives here. Every other builtin preset ships as a data file
//! written in the `.ctrm-sets` grammar and compiled in with `include_str!`
//! (V22) -- that is T22, T23 and T32, a later wave. Nothing here invents
//! their contents: a preset whose members were guessed would be a grant
//! nobody reviewed.
//!
//! `ascii` is the exception on purpose. `src/rules:V21` requires a run with
//! `--no-files --no-builtin-map --no-builtin-sets` to still resolve a
//! default set, so the one set that every path falls back to (`src/rules`
//! V1) cannot itself be a file that the flags remove.

use super::{CharRange, CharSet, SetCatalog, SetDefinition, SetMember};

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

#[cfg(test)]
mod tests {
    use super::{ASCII, ascii, ascii_definition, intrinsic_catalog};

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
        assert_eq!(intrinsic_catalog().resolve(ASCII), Ok(ascii()));
    }

    #[test]
    fn the_intrinsic_catalog_declares_only_ascii() {
        let catalog = intrinsic_catalog();
        let listed = catalog.names().collect::<Vec<_>>();
        assert_eq!(listed, vec![ASCII]);
    }
}
