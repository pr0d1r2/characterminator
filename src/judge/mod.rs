//! Judging: rules, sets, lints, scan and fix composed into findings.
//!
//! See `src/judge/SPEC.md`. Every verb and the guard judge through this
//! node, so no two of them can disagree about the same bytes
//! (`src/cli/guard:V35`). It owns what a finding IS; argv, discovery and
//! rendering are `src/cli`'s and `src/render`'s.

mod checker;
mod config;
mod suggest;
mod unruled;
mod walk;

pub(crate) use checker::{Checker, Judge, Looked, inspect, judged};
pub(crate) use config::Config;
pub(crate) use suggest::nearest;
pub(crate) use unruled::{Unruled, hazards_in, unruled};
pub(crate) use walk::{File, files, plain, shown_path};
