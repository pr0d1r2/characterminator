//! The `guard` verb: a hook adapter between an agent harness and the
//! hazard lints. See `src/cli/guard/SPEC.md`.
//!
//! Composes, does not implement (`src:C`). One direction of knowledge:
//! `json` reads a payload whole, `hook` is the only file that knows a
//! harness's shape, `adapter` takes the harness-agnostic call and
//! decides, `tier` says which hazards block and which are noted, and
//! `reason` words what the model is told, and `cap` says where a capped
//! pre-read may end. The parent sees one function.

mod adapter;
mod cap;
mod hook;
mod json;
mod reason;
mod tier;

pub(super) use adapter::run;
