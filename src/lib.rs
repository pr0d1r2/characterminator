//! `characterminator` -- find and eliminate characters outside an allowed
//! set, per file type and per file, so text costs fewer tokens.
//!
//! The crate is federated: every directory under `src/` is one Rust module
//! and one spec node, and each node's `SPEC.md` states what it owns. This
//! file composes and re-exports; it holds no logic of its own (`src:V38`).
//!
//! Nothing is implemented yet. The planned surface is in `SPEC.md` at the
//! repo root, and the tasks that build it are the task rows of each node.
