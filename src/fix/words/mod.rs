//! The opt-in `words` map and the spacing a word entry needs.
//!
//! See `src/fix/words/SPEC.md`. This file composes. The map grammar --
//! the `use` and `word` lines themselves -- is the parent's (`src/fix:I`);
//! this node owns what they name: the maps a `use` line may read, and how
//! a word is kept off a neighbouring letter, which the parent's walk asks
//! through [`Gap`].

mod gap;
mod named;

pub(super) use gap::Gap;
pub(super) use named::NAMED;
#[cfg(test)]
pub(super) use named::WORDS;

#[cfg(test)]
#[path = "apply_words_test.rs"]
mod spacing;
