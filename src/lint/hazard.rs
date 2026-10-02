//! Hazard detection (V34): which lint, if any, a character fires.
//!
//! The CODE POINTS are not here. They are the generated data file the
//! charset node compiles in (`src/charset` `hazard.ctrm-sets`), read
//! through its public surface (`src:V39`). What lives here is the part
//! that is this node's: which lint each class is reported under, the
//! order the classes are tried in, and the one cut a set cannot make --
//! a byte order mark at byte 0 is an encoding signature, not a hazard.
//!
//! A joiner inside an RGI ZWJ sequence, and a tag inside an RGI tag
//! sequence, are no hazard (V63). That cut is POSITIONAL, so it needs the
//! text: a caller asks [`Hazards::exempt`] once per text and hands the
//! offsets to [`Hazards::lint_at`]. The list lives in `sequence`.

use crate::charset::{CharSet, builtin};
use crate::lint::Lint;
use crate::lint::sequence::Sequences;
use crate::scan::Hit;

/// The hazard classes in the order they are TRIED, each with the lint it
/// is reported under. The first class holding a character names its lint.
///
/// The classes overlap -- every bidi control, tag and the BOM is also
/// default-ignorable -- so the order is the meaning: the narrow classes
/// come first, and `invisible` is what is left when nothing narrower
/// claims a character. A Trojan Source override therefore reports as
/// `bidi-control`, which is what a reader needs to know about it.
const CLASSES: [(&str, &str); 5] = [
    ("hazard-bidi", "bidi-control"),
    ("hazard-tag", "tag-character"),
    ("hazard-bom", "stray-bom"),
    ("hazard-control", "control-character"),
    ("hazard-invisible", "invisible"),
];

/// The lint whose ONE exemption is positional.
const STRAY_BOM: &str = "stray-bom";

/// The fidelity the classes are resolved at. They carry no labelled
/// member (`src/charset:V41`), so any family answers the same; this is
/// the default one, named rather than imported because a family is an
/// opaque name outside the fix node.
const FAMILY: &str = "text";

/// Every hazard class, resolved once per run, paired with its lint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hazards {
    classes: Vec<(Lint, CharSet)>,
    /// Each compiled-in preset that excuses joiners, and which (V57).
    excusing: Vec<Excuse>,
    /// The RGI sequences whose joiners and tags are no hazard (V63).
    sequences: Sequences,
}

impl Hazards {
    /// The classes as this build ships them.
    ///
    /// Read from the COMPILED-IN file, never from the run's catalog: a
    /// hazard is the one finding no configuration may switch off (V36),
    /// and a catalog is configuration.
    ///
    /// # Errors
    ///
    /// Only a defect in this crate: the data file failing to parse, a
    /// class it does not declare, or a lint the registry does not hold.
    pub fn builtin() -> Result<Self, String> {
        let classes = classes()?;
        let excusing = excusing(&classes)?;
        let sequences = Sequences::builtin();
        Ok(Self {
            classes,
            excusing,
            sequences,
        })
    }

    /// The byte offsets in `text` of every joiner and tag character that
    /// sits inside an RGI emoji sequence (V63), ascending.
    pub fn exempt(&self, text: &str) -> Vec<usize> {
        self.sequences.exempt(text)
    }

    /// [`Hazards::lint_for`], less a hit at an `exempt` offset.
    pub fn lint_at(&self, hit: Hit, exempt: &[usize]) -> Option<Lint> {
        let inside = exempt.binary_search(&hit.position.byte).is_ok();
        self.lint_for(hit).filter(|_| !inside)
    }

    /// Whether a file granted `granted` (a `+`-joined set name, as `check`
    /// names a union) is excused `character` (V57). Only a joiner can be,
    /// and only by a COMPILED-IN preset that grants it and no other hazard:
    /// `persian` excuses ZWNJ, `any` excuses nothing.
    pub fn excuses(&self, granted: &str, character: char) -> bool {
        granted.split('+').any(|name| {
            self.excusing.iter().any(|(set, joiners)| {
                set == name && joiners.contains(&character)
            })
        })
    }

    /// Whether a character is in ANY class, wherever it sits.
    ///
    /// A scan asks this before it knows a position, so a byte order mark
    /// answers yes here and is let off by [`Hazards::lint_for`].
    pub fn contains(&self, character: char) -> bool {
        self.classes.iter().any(|(_, set)| set.contains(character))
    }

    /// The hazard lint one hit fires, or `None` when it is no hazard.
    ///
    /// A BOM at byte 0 is `None`, and stops there: it is not passed on
    /// to `invisible`, which holds it too, because the exemption is for
    /// the CHARACTER at that place rather than for one class's name.
    pub fn lint_for(&self, hit: Hit) -> Option<Lint> {
        let (lint, _) = self
            .classes
            .iter()
            .find(|(_, set)| set.contains(hit.character))?;
        let signature = lint.name == STRAY_BOM && hit.position.byte == 0;
        (!signature).then_some(*lint)
    }
}

/// One hazard class and the lint it fires.
type Class = (Lint, CharSet);

/// Each class of the compiled-in hazard file, paired with its lint.
fn classes() -> Result<Vec<Class>, String> {
    let catalog = builtin::hazard_catalog().map_err(|e| e.to_string())?;
    CLASSES
        .iter()
        .map(|(set, lint)| {
            let lint = Lint::named(lint)
                .ok_or_else(|| format!("no `{lint}` lint registered"))?;
            let set = catalog.resolve(set, FAMILY);
            Ok((lint, set.map_err(|e| e.to_string())?))
        })
        .collect()
}

/// ZERO WIDTH NON-JOINER and ZERO WIDTH JOINER: the only hazards a grant
/// may excuse (V57). Persian needs ZWNJ inside words and Devanagari needs
/// both, so in those scripts they are SPELLING; a bidi control, a tag
/// character or a C0 control is never anybody's spelling.
const JOINERS: [char; 2] = ['\u{200C}', '\u{200D}'];

/// A preset name and the joiners it excuses.
type Excuse = (String, Vec<char>);

/// The compiled-in presets that grant a joiner and NO other hazard, with
/// the joiners each grants. Read from the builtin catalog, never the
/// run's: a `.ctrm-sets` redeclaring `persian` cannot widen this.
fn excusing(classes: &[(Lint, CharSet)]) -> Result<Vec<Excuse>, String> {
    let catalog = builtin::catalog().map_err(|e| e.to_string())?;
    let mut found = Vec::new();
    for name in catalog.names() {
        let set = catalog.resolve(name, FAMILY).map_err(|e| e.to_string())?;
        let joiners: Vec<char> =
            JOINERS.into_iter().filter(|j| set.contains(*j)).collect();
        if !joiners.is_empty() && !grants_other_hazard(&set, classes) {
            found.push((name.to_owned(), joiners));
        }
    }
    Ok(found)
}

/// Whether `set` grants any hazard that is not a joiner. `any` does, so
/// granting everything never excuses anything.
fn grants_other_hazard(set: &CharSet, classes: &[(Lint, CharSet)]) -> bool {
    classes
        .iter()
        .flat_map(|(_, class)| &class.ranges)
        .any(|range| {
            (range.start..=range.end)
                .any(|c| !JOINERS.contains(&c) && set.contains(c))
        })
}

#[cfg(test)]
mod tests {
    use super::Hazards;
    use crate::lint::{Group, Level, Levels, Lint, Target};
    use crate::scan::{Hit, Position};

    fn hazards() -> Hazards {
        let built = Hazards::builtin();
        assert!(built.is_ok(), "{built:?}");
        built.unwrap_or(Hazards {
            classes: Vec::new(),
            excusing: Vec::new(),
            sequences: super::Sequences::default(),
        })
    }

    fn at(byte: usize, character: char) -> Hit {
        let position = Position {
            line: 1,
            column: 1,
            byte,
        };
        Hit {
            position,
            character,
        }
    }

    /// The lint NAME a character fires at byte 5, or `""` for none.
    fn fired(character: char) -> &'static str {
        hazards().lint_for(at(5, character)).map_or("", |l| l.name)
    }

    /// Trojan Source (CVE-2021-42574): the overrides and isolates that
    /// make code display in one order and compile in another.
    #[test]
    fn a_bidi_override_or_isolate_is_a_bidi_control() {
        for point in ['\u{202A}', '\u{202E}', '\u{2066}', '\u{2069}'] {
            assert_eq!(fired(point), "bidi-control", "{point:?}");
        }
        assert_eq!(fired('\u{200F}'), "bidi-control");
    }

    /// ASCII smuggling: U+E0041 is TAG LATIN CAPITAL LETTER A, an `A`
    /// nobody sees and a model reads.
    #[test]
    fn a_tag_letter_is_a_tag_character() {
        assert_eq!(fired('\u{E0041}'), "tag-character");
        assert_eq!(fired('\u{E0001}'), "tag-character");
        assert_eq!(fired('\u{E007F}'), "tag-character");
    }

    #[test]
    fn a_control_other_than_the_layout_three_is_a_control_character() {
        for point in ['\u{0007}', '\u{000B}', '\u{001B}', '\u{007F}'] {
            assert_eq!(fired(point), "control-character", "{point:?}");
        }
        assert_eq!(fired('\u{0085}'), "control-character");
    }

    #[test]
    fn tab_newline_and_carriage_return_are_no_hazard() {
        assert_eq!(fired('\t'), "");
        assert_eq!(fired('\n'), "");
        assert_eq!(fired('\r'), "");
    }

    #[test]
    fn the_rest_of_the_ignorables_are_invisible() {
        for point in ['\u{200B}', '\u{200D}', '\u{00AD}', '\u{FE0F}'] {
            assert_eq!(fired(point), "invisible", "{point:?}");
        }
        assert_eq!(fired('\u{3164}'), "invisible");
        assert_eq!(fired('\u{E0100}'), "invisible");
    }

    #[test]
    fn ordinary_text_is_no_hazard() {
        for point in ['A', ' ', '\u{00E9}', '\u{2014}', '\u{1F600}'] {
            assert_eq!(fired(point), "", "{point:?}");
        }
        assert!(!hazards().contains('A'));
    }

    /// The positional cut: a signature at byte 0, a stray anywhere else
    /// -- and the one at byte 0 is not handed on to `invisible` either.
    #[test]
    fn a_bom_is_a_hazard_everywhere_but_byte_zero() {
        let found = hazards();
        assert_eq!(found.lint_for(at(0, '\u{FEFF}')), None);
        let stray = found.lint_for(at(3, '\u{FEFF}')).map(|l| l.name);
        assert_eq!(stray, Some("stray-bom"));
        assert!(found.contains('\u{FEFF}'));
    }

    /// Only byte 0 is exempt, and only for the BOM: another hazard at the
    /// start of a file is as much a hazard as anywhere.
    #[test]
    fn byte_zero_lets_off_the_bom_and_nothing_else() {
        let first = hazards().lint_for(at(0, '\u{202E}')).map(|l| l.name);
        assert_eq!(first, Some("bidi-control"));
    }

    /// Every lint a class names is a registered hazard, so it forbids.
    #[test]
    fn every_class_fires_a_hazard_lint_at_forbid() {
        for (lint, _) in &hazards().classes {
            assert_eq!(lint.group, Group::Hazard, "{}", lint.name);
            assert_eq!(lint.default_level(), Level::Forbid, "{}", lint.name);
        }
    }

    /// V36: `!hazard=allow`, and the lint named on its own, move nothing.
    #[test]
    fn no_rule_lowers_a_real_hazard_lint() {
        let lint = hazards().lint_for(at(5, '\u{202E}'));
        let lint = lint.unwrap_or(Lint::new("bidi-control", Group::Hazard));
        let mut levels = Levels::new();
        levels.set(Target::Group(Group::Hazard), Level::Allow);
        levels.set(Target::Lint(lint), Level::Warn);
        levels.set_charset(Level::Allow);
        assert_eq!(levels.level_of(lint), Level::Forbid);
    }
}
