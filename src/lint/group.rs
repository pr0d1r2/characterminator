//! The group a lint belongs to, and the default level it carries.

use crate::lint::Level;

/// The groups a lint can belong to. The group carries the default level:
/// hazard forbids, charset denies, pedantic allows until asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Group {
    Hazard,
    Charset,
    Pedantic,
}

impl Group {
    /// Every group, so a caller can list them without knowing how many
    /// there are. The order is loudest first, which is the order a
    /// `ctrm` listing wants.
    pub const ALL: &'static [Self] =
        &[Self::Hazard, Self::Charset, Self::Pedantic];

    /// The group a rule line names in `!<lint|group>=<level>`.
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "hazard" => Some(Self::Hazard),
            "charset" => Some(Self::Charset),
            "pedantic" => Some(Self::Pedantic),
            _ => None,
        }
    }

    /// The word a rule line and the json output carry for this group.
    pub fn name(self) -> &'static str {
        match self {
            Self::Hazard => "hazard",
            Self::Charset => "charset",
            Self::Pedantic => "pedantic",
        }
    }

    /// The level every lint in the group starts at, before any rule
    /// speaks.
    ///
    /// Hazard FORBIDS: an invisible character is a hazard whatever the
    /// file is for, and forbid is the level a later rule cannot lower.
    /// Charset DENIES: a character outside the declared set is the tool's
    /// ordinary violation, and it is what a run exits 1 for. Pedantic
    /// ALLOWS: those lints flag legitimate text often enough that firing
    /// them uninvited would teach a reader to ignore the output.
    pub fn default_level(self) -> Level {
        match self {
            Self::Hazard => Level::Forbid,
            Self::Charset => Level::Deny,
            Self::Pedantic => Level::Allow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Group;
    use crate::lint::Level;

    #[test]
    fn every_group_survives_a_round_trip_through_its_name() {
        for group in Group::ALL {
            assert_eq!(Group::named(group.name()), Some(*group));
        }
    }

    #[test]
    fn an_unknown_word_is_not_a_group() {
        assert_eq!(Group::named("hazardous"), None);
        assert_eq!(Group::named("Hazard"), None);
    }

    #[test]
    fn the_groups_carry_the_defaults_the_spec_states() {
        assert_eq!(Group::Hazard.default_level(), Level::Forbid);
        assert_eq!(Group::Charset.default_level(), Level::Deny);
        assert_eq!(Group::Pedantic.default_level(), Level::Allow);
    }

    #[test]
    fn a_hazard_starts_out_failing_the_run_and_a_pedantic_lint_does_not() {
        assert!(Group::Hazard.default_level().is_failure());
        assert!(Group::Charset.default_level().is_failure());
        assert!(!Group::Pedantic.default_level().is_failure());
    }
}
