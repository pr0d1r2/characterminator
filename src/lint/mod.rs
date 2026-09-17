//! How loud a finding is: lint names, groups and levels.
//!
//! See `src/lint/SPEC.md`. This file COMPOSES: the vocabulary and the
//! registry live one type to a file below it, and the names re-exported
//! here are the node's public surface, so a sibling still writes
//! `crate::lint::Level` and never names a file of mine.

mod finding;
mod group;
mod level;
mod registry;
mod resolve;
mod target;

pub use finding::{Finding, exit_code};
pub use group::Group;
pub use level::Level;
pub use registry::{LINTS, Lint};
pub use resolve::Levels;
pub use target::Target;
