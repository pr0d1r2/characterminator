//! `characterminator` -- find and eliminate characters outside an allowed
//! set, per file type and per file, so text costs fewer tokens.
//!
//! The crate is federated: every directory under `src/` is one Rust module
//! and one spec node, and each node's `SPEC.md` states what it owns. This
//! file composes and re-exports; it holds no logic of its own (`src:V38`).
//!
//! The nodes are PRIVATE. What a library user may name is the short list
//! below, re-exported here and nowhere else (`src:V98`): enough to check a
//! string against a set, rewrite it, and read the hazards in it. Everything
//! else -- rule resolution, config assembly, rendering -- is the binary's
//! business until a release decides to promise it.

mod charset;
mod cli;
mod fix;
mod lint;
mod render;
mod rules;
mod scan;
mod tokens;

pub use charset::{CharRange, CharSet, ComposeError, SetCatalog};
pub use fix::{
    Error as FixError, Fixed, Map, Report as FixReport, Rewrite, fix,
};
pub use lint::{Group, Hazards, Level, Lint};
pub use scan::{Hit, Position, Unreadable, scan_bytes, scan_str};

/// The binary's entry point. Not part of the library promise: `main.rs`
/// is a shim over it (`src:V38`), and it lives here so the dispatch is
/// testable rather than reachable only by launching a process.
#[doc(hidden)]
pub use cli::run;
