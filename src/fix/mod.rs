//! Rewrite a disallowed character SAFELY: map, families, classes.
//!
//! See `src/fix/SPEC.md`. This file composes and declares the vocabulary;
//! the work lives in the modules below. Whether a character is ALLOWED is
//! never decided here: it arrives as a predicate, because that answer
//! belongs to the charset and rules nodes (`src:V39`).

mod apply;
mod class;
mod codepoint;
mod error;
mod family;
mod map;

pub use apply::{Fixed, Report, fix};
pub use error::Error;
pub(crate) use map::BUILTIN;
pub use map::Map;

use crate::rules::Origin;
use crate::scan::Hit;

/// A character family, declared as a name and its fallback parent. The
/// chain ends at `ascii`, so resolution always terminates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Family {
    pub name: String,
    pub parent: Option<String>,
}

/// One member of an equivalence class: the family it belongs to and the
/// text it is written as. Text rather than char, because a member may be a
/// sequence of code points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Member {
    pub family: String,
    pub text: String,
}

/// An equivalence class: characters that mean the same thing, grouped by
/// family so each project can say which family it wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Class {
    pub name: String,
    pub members: Vec<Member>,
}

/// A plain transliteration entry. An empty `to` is an explicit delete,
/// which is the only way a character is ever removed.
///
/// `word` marks an entry a `word` line declared (V51): its replacement is
/// a word, so where it would land against a letter or a digit `fix` puts
/// a space between the two rather than fusing them into one (`notowns`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapEntry {
    pub from: String,
    pub to: String,
    pub word: bool,
    pub origin: Origin,
}

/// A rewrite that `fix` would apply: what was found, and what replaces it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rewrite {
    pub hit: Hit,
    pub to: String,
}
