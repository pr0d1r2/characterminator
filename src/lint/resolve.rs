//! Level resolution: how loud each lint ended up being.
//!
//! A run collects level directives -- one per rule suffix that named a
//! level, in the order the rules matched -- and then asks this table what
//! a given lint's level is. Two things decide the answer: the LAST
//! directive that covers the lint wins, the way a later `.ctrm` line wins
//! (`src/rules:V2`), and a `forbid` is a FLOOR that nothing after it can
//! lower.

use crate::lint::{Finding, Level, Lint, Target};
use crate::scan::Hit;

/// The level directives a run collected, plus whether strict mode is on.
///
/// Directives are kept in order rather than folded into a map as they
/// arrive, because folding would have to decide the forbid question at
/// insert time and would lose the sequence `explain` needs to show.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Levels {
    directives: Vec<(Target, Level)>,
    strict: bool,
}

impl Levels {
    /// An empty table: every lint sits at its group's default level.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record a `!<lint|group>=<level>` suffix. Call order is rule order,
    /// and the later call wins wherever both cover the same lint.
    pub(crate) fn set(&mut self, target: Target, level: Level) {
        self.directives.push((target, level));
    }

    /// Record a bare `!<level>` suffix, which moves the charset group.
    pub(crate) fn set_charset(&mut self, level: Level) {
        self.set(Target::BARE, level);
    }

    /// Turn strict mode on: warn counts as deny, so a warned lint starts
    /// failing the run. It raises only, and it cannot reach a forbid.
    pub(crate) fn set_strict(&mut self, strict: bool) {
        self.strict = strict;
    }

    /// The level that applies to one lint, all directives and strict mode
    /// taken into account.
    pub(crate) fn level_of(&self, lint: Lint) -> Level {
        let stated = self.stated(lint);
        if self.strict {
            stated.under_strict()
        } else {
            stated
        }
    }

    /// What one hit is worth: the hit, the lint that found it, and the
    /// level this table resolved for that lint.
    pub(crate) fn finding(&self, hit: Hit, lint: Lint) -> Finding {
        Finding {
            hit,
            lint,
            level: self.level_of(lint),
        }
    }

    /// What the rules said, before strict mode is applied.
    ///
    /// THE FORBID FLOOR IS THIS LOOP'S `break`. Once the level in hand is
    /// forbid -- whether the group started there or a rule raised it --
    /// no later directive is even read, so a relaxed configuration written
    /// afterwards cannot talk a hazard down.
    fn stated(&self, lint: Lint) -> Level {
        let mut level = lint.default_level();
        for (target, stated) in &self.directives {
            if level == Level::Forbid {
                break;
            }
            if target.covers(lint) {
                level = *stated;
            }
        }
        level
    }
}

#[cfg(test)]
mod tests {
    use super::Levels;
    use crate::lint::{Group, Level, Lint, Target};
    use crate::scan::{Hit, Position};

    /// A registered hazard lint (T36). The group is what forbids, so no
    /// Unicode data is needed to test the floor.
    fn hazard() -> Lint {
        Lint::new("invisible", Group::Hazard)
    }

    fn charset() -> Lint {
        crate::lint::OUTSIDE_SET
    }

    fn pedantic() -> Lint {
        Lint::new("crlf", Group::Pedantic)
    }

    fn hit() -> Hit {
        let position = Position {
            line: 1,
            column: 1,
            byte: 0,
        };
        Hit {
            position,
            character: 'x',
        }
    }

    #[test]
    fn the_test_lints_are_the_registered_ones() {
        assert_eq!(Lint::named("outside-set"), Some(charset()));
        assert_eq!(Lint::named("crlf"), Some(pedantic()));
        assert_eq!(Lint::named("invisible"), Some(hazard()));
    }

    #[test]
    fn with_no_directive_a_lint_sits_at_its_groups_default() {
        let levels = Levels::new();
        assert_eq!(levels.level_of(hazard()), Level::Forbid);
        assert_eq!(levels.level_of(charset()), Level::Deny);
        assert_eq!(levels.level_of(pedantic()), Level::Allow);
    }

    #[test]
    fn the_later_directive_wins() {
        let mut levels = Levels::new();
        levels.set_charset(Level::Warn);
        levels.set_charset(Level::Allow);
        assert_eq!(levels.level_of(charset()), Level::Allow);
        levels.set_charset(Level::Deny);
        assert_eq!(levels.level_of(charset()), Level::Deny);
    }

    #[test]
    fn a_lint_directive_and_a_group_directive_race_by_order() {
        let mut levels = Levels::new();
        levels.set(Target::Lint(pedantic()), Level::Deny);
        levels.set(Target::Group(Group::Pedantic), Level::Warn);
        assert_eq!(levels.level_of(pedantic()), Level::Warn);
        levels.set(Target::Lint(pedantic()), Level::Allow);
        assert_eq!(levels.level_of(pedantic()), Level::Allow);
    }

    #[test]
    fn a_directive_moves_only_what_it_covers() {
        let mut levels = Levels::new();
        levels.set_charset(Level::Allow);
        assert_eq!(levels.level_of(charset()), Level::Allow);
        assert_eq!(levels.level_of(hazard()), Level::Forbid);
        assert_eq!(levels.level_of(pedantic()), Level::Allow);
    }

    #[test]
    fn a_group_directive_moves_every_lint_in_that_group() {
        let mut levels = Levels::new();
        levels.set(Target::Group(Group::Pedantic), Level::Warn);
        assert_eq!(levels.level_of(pedantic()), Level::Warn);
        let other = Lint::new("confusable", Group::Pedantic);
        assert_eq!(levels.level_of(other), Level::Warn);
    }

    /// The invariant that matters most in this node: a hazard finding has
    /// to survive a relaxed configuration.
    #[test]
    fn no_directive_of_any_level_lowers_a_forbidding_group() {
        for level in Level::ALL {
            let mut levels = Levels::new();
            levels.set(Target::Group(Group::Hazard), *level);
            levels.set(Target::Lint(hazard()), *level);
            assert_eq!(levels.level_of(hazard()), Level::Forbid, "{level:?}");
        }
    }

    #[test]
    fn a_forbid_a_rule_raised_is_just_as_final() {
        let mut levels = Levels::new();
        levels.set_charset(Level::Forbid);
        levels.set_charset(Level::Allow);
        levels.set(Target::Lint(charset()), Level::Warn);
        assert_eq!(levels.level_of(charset()), Level::Forbid);
    }

    #[test]
    fn strict_does_not_lower_a_forbid_either() {
        let mut levels = Levels::new();
        levels.set(Target::Group(Group::Hazard), Level::Allow);
        levels.set_strict(true);
        assert_eq!(levels.level_of(hazard()), Level::Forbid);
    }

    #[test]
    fn strict_raises_a_warned_lint_to_deny() {
        let mut levels = Levels::new();
        levels.set_charset(Level::Warn);
        assert_eq!(levels.level_of(charset()), Level::Warn);
        levels.set_strict(true);
        assert_eq!(levels.level_of(charset()), Level::Deny);
    }

    #[test]
    fn strict_leaves_an_allowed_lint_silent() {
        let mut levels = Levels::new();
        levels.set_charset(Level::Allow);
        levels.set_strict(true);
        assert_eq!(levels.level_of(charset()), Level::Allow);
        assert_eq!(levels.level_of(pedantic()), Level::Allow);
    }

    #[test]
    fn strict_can_be_turned_back_off() {
        let mut levels = Levels::new();
        levels.set_charset(Level::Warn);
        levels.set_strict(true);
        levels.set_strict(false);
        assert_eq!(levels.level_of(charset()), Level::Warn);
    }

    #[test]
    fn a_finding_carries_the_level_this_table_resolved() {
        let mut levels = Levels::new();
        levels.set_charset(Level::Warn);
        let finding = levels.finding(hit(), charset());
        assert_eq!(finding.level, Level::Warn);
        assert_eq!(finding.lint, charset());
        assert_eq!(finding.hit, hit());
    }
}
