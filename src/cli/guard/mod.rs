//! The `guard` verb: a hook adapter between an agent harness and the
//! hazard lints. See `src/cli/guard/SPEC.md`.
//!
//! Composes, does not implement (`src:C`). Three files, one direction of
//! knowledge: `json` reads a payload whole, `hook` is the only file that
//! knows a harness's shape, and `adapter` takes the harness-agnostic call
//! and decides. The parent sees one function.

mod adapter;
mod hook;
mod json;

pub(super) use adapter::run;
