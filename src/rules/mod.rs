//! Which set applies WHERE: config files and flags, precedence, origin.
//!
//! See `src/rules/SPEC.md`. This file composes (`src:C`): the public type
//! vocabulary, the submodules, and what they re-export. The logic for T15
//! is still to come.

mod config;
mod glob;
mod line;
mod property;
mod resolve;
mod rule_line;

pub(crate) use config::{Place, Sources};
pub(crate) use glob::matches;
pub(crate) use line::{ParseError, describe, error, parse_flag};
#[cfg(test)]
pub(crate) use property::corpus;
pub(crate) use resolve::{Resolution, resolve};
#[cfg(test)]
pub(crate) use rule_line::parse_rule;

use crate::lint::Level;
use std::path::PathBuf;

/// The set every path gets when no rule matches it (V1).
///
/// It is a CONSTANT IN CODE rather than a line of a data file, because a
/// run given no files at all (V21) must still be able to say what a path
/// may contain. A default that lives in a file is not a default; it is a
/// file that is usually present.
pub(crate) use crate::charset::ASCII;

/// The fidelity family a rule gets when none is named (V29).
///
/// A constant for the same reason `ascii` is one: a run reading no file
/// still has to say which variant of a character it prefers, and the
/// answer cannot live in data that may be absent. What the name MEANS --
/// which members a family holds, which one it prefers -- is the fix
/// node's question (`src/fix:V27`); this node only carries the name.
pub(crate) const TEXT: &str = "text";

/// Where an effective entry came from.
///
/// Every rule, map entry and set carries one, because with builtins, files
/// and flags all contributing, "why is this character allowed here" has no
/// answer without it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A line of a data file.
    File { path: PathBuf, line: usize },
    /// An inline flag, by its position in argv.
    Argument { index: usize },
    /// A line of a compiled-in data file.
    Builtin { line: usize },
}

/// One effective setting and the origin of the line that set it (V20).
///
/// `explain` reports what is IN FORCE, and the winner of the grant is not
/// the only rule that decides it: a level-only line moves levels (V56)
/// and another line may name the family (V29). Each carries its own
/// origin here, so the report names every line that decided something.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sourced<'a, T> {
    pub value: T,
    pub origin: &'a Origin,
}

/// A level set for one lint or group by a rule, as in `!pedantic=warn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LevelChoice {
    pub target: String,
    pub level: Level,
}

/// One rule line: which paths it matches and what it grants them.
///
/// The family is held as a NAME rather than a resolved family, so this node
/// stays independent of where families are declared; resolving it is the
/// fix node's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rule {
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
