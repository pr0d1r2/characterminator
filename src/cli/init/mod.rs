//! The `init` verb (V128): from no configuration to a draft `.ctrm`. See
//! `src/cli/init/SPEC.md`.
//!
//! Composes, does not implement (`src:C`): `survey` reads the tracked
//! files by type, `cover` picks the grant, `draft` writes the text, and
//! `verb` is the entry point the parent sees.

mod cover;
mod draft;
mod survey;
mod verb;

pub(super) use verb::run;

#[cfg(test)]
#[path = "init_test.rs"]
mod tests;
