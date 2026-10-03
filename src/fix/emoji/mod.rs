//! Emoji compression: the sequence map, flag pairing and the multi-pass
//! fix a sequence needs.
//!
//! See `src/fix/emoji/SPEC.md`. This file composes. The engine in
//! `src/fix` walks the text; it asks this node two things through the
//! narrow surface below -- whether a flag may start here (V76), and how
//! a later pass's positions map back to the original (V65). The
//! sequence map itself (`emoji-seq.ctrm-map`, V62) is data the parent's
//! builtin map compiles in.

mod flags;
mod settle;

pub(super) use flags::Pairing;
pub(super) use settle::{Extent, Layer, SETTLE, Trail, itself};

#[cfg(test)]
#[path = "apply_sequences_test.rs"]
mod sequences;
