//! Which set applies WHERE: config files and flags, precedence, origin.
//!
//! See `src/rules/SPEC.md`. Types only for now; the logic arrives with T6,
//! T15, T18, T19, T20, T21, T24 and T31.

use crate::lint::Level;
use std::path::PathBuf;

/// Where an effective entry came from.
///
/// Every rule, map entry and set carries one, because with builtins, files
/// and flags all contributing, "why is this character allowed here" has no
/// answer without it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A line of a data file.
    File { path: PathBuf, line: usize },
    /// An inline flag, by its position in argv.
    Argument { index: usize },
    /// A line of a compiled-in data file.
    Builtin { line: usize },
}

/// A level set for one lint or group by a rule, as in `!pedantic=warn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelChoice {
    pub target: String,
    pub level: Level,
}

/// One rule line: which paths it matches and what it grants them.
///
/// The family is held as a NAME rather than a resolved family, so this node
/// stays independent of where families are declared; resolving it is the
/// fix node's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub pattern: String,
    pub sets: Vec<String>,
    pub family: Option<String>,
    pub levels: Vec<LevelChoice>,
    pub origin: Origin,
}
