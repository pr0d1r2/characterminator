//! Argument dispatch, verbs and exit codes. Not a verb's logic.
//!
//! See `src/cli/SPEC.md`. This file composes (`src:C`): the dispatch
//! itself, usage and the exit codes live in `dispatch.rs`.

mod args;
mod check;
mod config;
mod dispatch;
mod explain;
mod fix;
mod guard;
mod out;
mod stats;
#[cfg(test)]
pub(crate) mod testkit;

use dispatch::Outcome;

pub use dispatch::run;
