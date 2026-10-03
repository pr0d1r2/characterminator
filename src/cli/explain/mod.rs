//! The `explain` and `sets` verbs, and `explain --as`: what is in force,
//! why, and the configuration written back out. See
//! `src/cli/explain/SPEC.md`.
//!
//! Composes, does not implement (`src:C`). `answer` holds the two verbs,
//! `export` the `args` and `lines` forms, `prompt` the agent instruction.
//! The parent sees the three entry points and nothing else.

mod answer;
mod export;
mod listing;
mod prompt;
mod prompt_lines;

pub(super) use answer::{exported, run};
pub(super) use listing::{Asked, sets};
