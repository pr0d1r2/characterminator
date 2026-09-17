//! The level a lint speaks at.

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
