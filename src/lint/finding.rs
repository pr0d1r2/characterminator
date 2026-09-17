//! One reportable finding, and the exit decision a run makes from them.

use crate::lint::{Level, Lint};
use crate::scan::Hit;

/// One reportable finding: what was found, which lint found it, and how
/// loudly it is being said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub hit: Hit,
    pub lint: Lint,
    pub level: Level,
}

impl Finding {
    /// Whether this finding fails the run: its level decides, and nothing
    /// about the character does. A hazard is loud because its group
    /// forbids, not because this asks what it is.
    pub fn is_failure(&self) -> bool {
        self.level.is_failure()
    }
}

/// The exit decision: a deny or forbid finding means the run failed.
///
/// It answers 0 or 1 ONLY. Code 2 is usage, which is the cli node's to
/// give, and a run that could not parse its own arguments has no findings
/// to weigh. Warned and allowed findings are reported and still exit 0:
/// that is what makes `warn` different from `deny` rather than a second
/// spelling of it, and `--strict` is how a run asks for warnings to count.
pub fn exit_code(findings: &[Finding]) -> u8 {
    u8::from(findings.iter().any(Finding::is_failure))
}

#[cfg(test)]
mod tests {
    use super::{Finding, exit_code};
    use crate::lint::{Group, Level, Lint};
    use crate::scan::{Hit, Position};

    fn finding(level: Level) -> Finding {
        let position = Position {
            line: 1,
            column: 1,
            byte: 0,
        };
        let hit = Hit {
            position,
            character: 'x',
        };
        let lint = Lint::new("outside-set", Group::Charset);
        Finding { hit, lint, level }
    }

    #[test]
    fn a_clean_run_exits_zero() {
        assert_eq!(exit_code(&[]), 0);
    }

    #[test]
    fn allowed_and_warned_findings_do_not_fail_the_run() {
        let quiet = [finding(Level::Allow), finding(Level::Warn)];
        assert_eq!(exit_code(&quiet), 0);
    }

    #[test]
    fn one_denied_finding_fails_the_run() {
        let mixed = [finding(Level::Warn), finding(Level::Deny)];
        assert_eq!(exit_code(&mixed), 1);
        assert_eq!(exit_code(&[finding(Level::Deny)]), 1);
    }

    #[test]
    fn one_forbidden_finding_fails_the_run() {
        let mixed = [finding(Level::Allow), finding(Level::Forbid)];
        assert_eq!(exit_code(&mixed), 1);
    }

    #[test]
    fn a_finding_fails_exactly_when_its_level_does() {
        for level in Level::ALL {
            let one = finding(*level);
            assert_eq!(one.is_failure(), level.is_failure(), "{level:?}");
        }
    }
}
