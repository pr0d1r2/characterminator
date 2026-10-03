//! The hazard group: which lint, if any, a character fires, and the
//! emoji sequences that let a joiner, a tag or VS16 off.
//!
//! See `src/lint/hazard/SPEC.md`. This file COMPOSES: detection lives in
//! `hazards`, the RGI sequence list in `sequence`, and the parent
//! `src/lint` re-exports what a sibling node may name.

mod hazards;
mod sequence;

pub use hazards::Hazards;
pub(crate) use hazards::carries_no_text;
