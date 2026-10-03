//! How loud a finding is: lint names, groups and levels.
//!
//! See `src/lint/SPEC.md`. This file COMPOSES: the vocabulary and the
//! registry live one type to a file below it, and the names re-exported
//! here are the node's public surface, so a sibling still writes
//! `crate::lint::Level` and never names a file of mine.

mod finding;
mod group;
mod hazard;
mod level;
mod pedantic;
mod registry;
mod resolve;
mod sequence;
mod target;
mod ucd;

pub(crate) use finding::{Finding, exit_code, one_claim};
pub use group::Group;
pub use hazard::Hazards;
pub use level::Level;
pub(crate) use pedantic::{
    CHAR_LINTS, LINE_LINTS, TEXT_LINTS, char_lints, line_hits, text_hits,
    unicode_space,
};
pub use registry::Lint;
pub(crate) use registry::{LINTS, OUTSIDE_SET};
pub(crate) use resolve::Levels;
pub(crate) use target::Target;
