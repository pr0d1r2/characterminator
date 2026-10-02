//! The named checks this build knows.
//!
//! One table, so `ctrm` can list its lints, a rule line can address one by
//! name, and a name nobody registered is an error instead of a directive
//! that silently matches nothing.
//!
//! HOW A HAZARD LINT ARRIVED (T36): a row here with `group:
//! Group::Hazard`, and nothing else. The group forbids and the resolver
//! refuses to lower a forbid; the detection is `hazard.rs`, and the code
//! points it reads are the charset node's generated data file -- no table
//! of code points is shipped from this node.

use crate::lint::{Group, Level, pedantic};

/// A named check. The name is what a rule line and the json output carry,
/// so it is the stable identifier rather than the message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lint {
    pub name: &'static str,
    pub group: Group,
}

/// Every lint this build knows.
///
/// Names are lowercase and hyphenated, the way clippy and rustc spell
/// theirs, and no name here may equal a group name: `!<lint|group>=<level>`
/// takes one token for both, so a collision would make a rule ambiguous.
/// The tests below enforce that.
///
/// The pedantic rows ship their NAMES whether or not their detection has
/// landed. V37 requires the group to stay `allow`, so a registered
/// pedantic lint reports nothing until a run asks for it; registering the
/// names up front is what lets `--pedantic` and `!not-nfc=warn` be parsed,
/// explained and rejected-on-typo before any of them can fire.
///
/// Every pedantic row is the constant `pedantic.rs` detects under, so the
/// name is spelled once: four need no data (V55), four read Unicode's
/// tables (V58).
pub const LINTS: &[Lint] = &[
    // V34's classes, one lint each, in the order `hazard.rs` tries them.
    // One lint per class rather than one `hazard` lint: the json names
    // the lint, and "this is a bidi override" is the sentence a reader of
    // a Trojan Source finding needs, where "this is a hazard" is not.
    Lint::new("bidi-control", Group::Hazard),
    Lint::new("tag-character", Group::Hazard),
    Lint::new("stray-bom", Group::Hazard),
    Lint::new("control-character", Group::Hazard),
    Lint::new("invisible", Group::Hazard),
    // The tool's ordinary violation: a character outside the set the rules
    // granted this path. Named for what is true of the character rather
    // than for its group, because `charset` is already the group's name.
    Lint::new("outside-set", Group::Charset),
    // V37's candidates, in the order that spec lists them.
    pedantic::NOT_NFC,
    pedantic::NFKC_COMPAT,
    pedantic::UNICODE_SPACE,
    pedantic::MIXED_SCRIPT,
    pedantic::CONFUSABLE,
    pedantic::CRLF,
    pedantic::TRAILING_WHITESPACE,
    pedantic::FINAL_NEWLINE,
];

impl Lint {
    /// One row of the table. It exists so the table reads one lint to a
    /// line: the struct literal spelled out is four lines per lint, and a
    /// registry nobody can read in one screen is a registry that grows
    /// duplicates.
    pub const fn new(name: &'static str, group: Group) -> Self {
        Self { name, group }
    }

    /// The lint a rule line names. An unregistered name is `None`, so a
    /// misspelled lint is reported rather than being a directive that
    /// matches nothing for the rest of the run.
    pub fn named(name: &str) -> Option<Self> {
        LINTS.iter().copied().find(|lint| lint.name == name)
    }

    /// The level this lint starts at, before any rule speaks: its group's
    /// default.
    pub fn default_level(self) -> Level {
        self.group.default_level()
    }
}

#[cfg(test)]
mod tests {
    use super::{LINTS, Lint};
    use crate::lint::{Group, Level};

    #[test]
    fn every_registered_lint_resolves_back_to_itself() {
        for lint in LINTS {
            assert_eq!(Lint::named(lint.name), Some(*lint));
        }
    }

    #[test]
    fn an_unregistered_name_is_not_a_lint() {
        assert_eq!(Lint::named("no-such-lint"), None);
        assert_eq!(Lint::named(""), None);
    }

    #[test]
    fn no_lint_name_is_registered_twice() {
        let mut seen: Vec<&str> = Vec::new();
        for lint in LINTS {
            assert!(!seen.contains(&lint.name), "duplicate: {}", lint.name);
            seen.push(lint.name);
        }
    }

    /// The token in `!<lint|group>=<level>` is one word for both kinds, so
    /// a lint sharing a group's name would make a rule line ambiguous.
    #[test]
    fn no_lint_is_named_after_a_group() {
        for lint in LINTS {
            assert_eq!(Group::named(lint.name), None, "{}", lint.name);
        }
    }

    #[test]
    fn a_lint_starts_at_its_groups_level() {
        let outside = Lint::named("outside-set");
        assert_eq!(outside.map(Lint::default_level), Some(Level::Deny));
        let nfc = Lint::named("not-nfc");
        assert_eq!(nfc.map(Lint::default_level), Some(Level::Allow));
    }

    #[test]
    fn every_hazard_lint_forbids_before_any_rule_speaks() {
        let hazards = LINTS.iter().filter(|l| l.group == Group::Hazard);
        assert_eq!(hazards.clone().count(), 5);
        for lint in hazards {
            assert_eq!(lint.default_level(), Level::Forbid, "{}", lint.name);
        }
    }

    #[test]
    fn every_pedantic_lint_is_silent_until_asked() {
        let pedantic = LINTS.iter().filter(|l| l.group == Group::Pedantic);
        for lint in pedantic {
            assert_eq!(lint.default_level(), Level::Allow, "{}", lint.name);
        }
    }
}
