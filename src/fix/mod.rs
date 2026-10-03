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

pub(crate) use apply::fix_under;
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

/// What `fix` asks of the file it rewrites (V6, V104): which characters
/// its set grants, and where a text holds a HAZARD -- a character that is
/// rewritten whatever the set grants, because a grant was never meant to
/// let in what hides or reorders text (`src/lint:V34`).
///
/// Both arrive from outside: whether a character is allowed, or a hazard
/// at that place in that file, is the judge's answer, not this node's
/// (`src:V39`). `hazards` is asked once per pass, of the text that pass
/// reads, so a later pass (V65) sees the hazards where they now sit.
#[derive(Clone, Copy)]
pub(crate) struct Law<'a> {
    pub allowed: &'a dyn Fn(char) -> bool,
    pub hazards: &'a HazardsIn<'a>,
}

/// The hazards of one text, ascending by byte.
pub(crate) type HazardsIn<'a> = dyn Fn(&str) -> Vec<Hazard> + 'a;

/// One hazard in one text: its byte offset, and whether it may be DELETED
/// when no map entry covers it. A hazard that carries no visible text --
/// a bidi control, a tag, a stray BOM, an invisible -- may; a control
/// character may not, since what it meant is not this tool's to guess
/// (V104).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hazard {
    pub byte: usize,
    pub delete: bool,
}
