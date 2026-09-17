//! `characterminator` -- find and eliminate characters outside an allowed
//! set, per file type and per file, so text costs fewer tokens.
//!
//! The crate is federated: every directory under `src/` is one Rust module
//! and one spec node, and each node's `SPEC.md` states what it owns. This
//! file composes and re-exports; it holds no logic of its own (`src:V38`).
//!
//! Each module currently declares its public type vocabulary and nothing
//! else. Those types are the seam between nodes: a node may use a sibling
//! only through its public surface (`src:V39`), so with the vocabulary in
//! place every node can be built independently of the others.

pub mod charset;
pub mod cli;
pub mod fix;
pub mod lint;
pub mod render;
pub mod rules;
pub mod scan;
pub mod tokens;
