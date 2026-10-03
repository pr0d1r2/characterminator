//! The level a lint speaks at.

/// The rustc and clippy levels, in the order of increasing severity.
///
/// `Forbid` is the one that cannot be lowered by a later rule or flag, and
/// it exists so the hazard group cannot be argued down to a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Level {
    Allow,
    Warn,
    Deny,
    Forbid,
}

impl Level {
    /// Every level, quietest first, so a caller can list or walk them
    /// without knowing how many there are.
    pub const ALL: &'static [Self] =
        &[Self::Allow, Self::Warn, Self::Deny, Self::Forbid];

    /// The level a rule line names, spelled the way rustc and clippy spell
    /// it. Unknown words are `None` rather than a default, so a typo in a
    /// config file is reported instead of silently meaning `allow`.
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "allow" => Some(Self::Allow),
            "warn" => Some(Self::Warn),
            "deny" => Some(Self::Deny),
            "forbid" => Some(Self::Forbid),
            _ => None,
        }
    }

    /// The word a rule line and the json output carry for this level. It is
    /// the inverse of `named`, so what is printed can be fed back in.
    pub fn name(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::Forbid => "forbid",
        }
    }

    /// Whether a finding at this level fails the run: deny and forbid do,
    /// allow and warn do not.
    pub fn is_failure(self) -> bool {
        matches!(self, Self::Deny | Self::Forbid)
    }

    /// Strict mode, the clippy `-D warnings` shape: warn counts as deny.
    ///
    /// It RAISES only. Deny and forbid are already at least as loud, and
    /// allow stays silent: a mode that made every allowed lint fire would
    /// be a different request than making warnings count.
    pub fn under_strict(self) -> Self {
        if self == Self::Warn { Self::Deny } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::Level;

    #[test]
    fn every_level_survives_a_round_trip_through_its_name() {
        for level in Level::ALL {
            assert_eq!(Level::named(level.name()), Some(*level));
        }
    }

    #[test]
    fn an_unknown_word_is_not_a_level() {
        assert_eq!(Level::named("error"), None);
        assert_eq!(Level::named("Deny"), None);
        assert_eq!(Level::named(""), None);
    }

    #[test]
    fn deny_and_forbid_fail_the_run_and_the_quiet_levels_do_not() {
        assert!(!Level::Allow.is_failure());
        assert!(!Level::Warn.is_failure());
        assert!(Level::Deny.is_failure());
        assert!(Level::Forbid.is_failure());
    }

    #[test]
    fn strict_raises_warn_to_deny_and_moves_nothing_else() {
        assert_eq!(Level::Warn.under_strict(), Level::Deny);
        assert_eq!(Level::Allow.under_strict(), Level::Allow);
        assert_eq!(Level::Deny.under_strict(), Level::Deny);
        assert_eq!(Level::Forbid.under_strict(), Level::Forbid);
    }

    #[test]
    fn the_levels_are_ordered_by_severity() {
        assert!(Level::Allow < Level::Warn);
        assert!(Level::Warn < Level::Deny);
        assert!(Level::Deny < Level::Forbid);
    }
}
