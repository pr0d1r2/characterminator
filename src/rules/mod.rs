//! Which set applies WHERE: config files and flags, precedence, origin.
//!
//! See `src/rules/SPEC.md`. This file composes (`src:C`): the public type
//! vocabulary, the submodules, and what they re-export. The logic for T15
//! is still to come.

mod line;
mod property;
mod resolve;
mod rule_line;

pub use line::{
    ParseError, describe, error, parse_builtin, parse_file, parse_flag,
    parse_lines,
};
pub use resolve::{PathMatcher, Resolution, resolve};
pub use rule_line::parse_rule;

use crate::lint::Level;
use std::path::PathBuf;

/// The set every path gets when no rule matches it (V1).
///
/// It is a CONSTANT IN CODE rather than a line of a data file, because a
/// run given no files at all (V21) must still be able to say what a path
/// may contain. A default that lives in a file is not a default; it is a
/// file that is usually present.
pub const ASCII: &str = "ascii";

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
    /// Levels set for named lints and groups, as in `!pedantic=warn`.
    pub levels: Vec<LevelChoice>,
    /// The rule's own level, as in a bare `!allow`.
    ///
    /// A separate field rather than a `LevelChoice` with a magic target
    /// name: the bare form names no target, and spelling that as an empty
    /// or reserved string would be a value every reader has to be told
    /// about.
    pub default_level: Option<Level>,
    pub origin: Origin,
}
