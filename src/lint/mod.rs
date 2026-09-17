//! How loud a finding is: lint names, groups and levels.
//!
//! See `src/lint/SPEC.md`. Types only for now; the logic arrives with T35,
//! T36, T38 and T39.

use crate::scan::Hit;

/// The rustc and clippy levels, in the order of increasing severity.
///
/// `Forbid` is the one that cannot be lowered by a later rule or flag, and
/// it exists so the hazard group cannot be argued down to a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Allow,
    Warn,
    Deny,
    Forbid,
}

/// The groups a lint can belong to. The group carries the default level:
/// hazard forbids, charset denies, pedantic allows until asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Hazard,
    Charset,
    Pedantic,
}

/// A named check. The name is what a rule line and the json output carry,
/// so it is the stable identifier rather than the message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lint {
    pub name: &'static str,
    pub group: Group,
}

/// One reportable finding: what was found, which lint found it, and how
/// loudly it is being said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub hit: Hit,
    pub lint: Lint,
    pub level: Level,
}
