//! Rewrite a disallowed character SAFELY: map, families, classes.
//!
//! See `src/fix/SPEC.md`. This file composes and declares the vocabulary;
//! the work lives in the modules below. Whether a character is ALLOWED is
//! never decided here: it arrives as a predicate, because that answer
//! belongs to the charset and rules nodes (`src:V39`).

mod apply;
mod class;
mod codepoint;
mod family;
mod map;

pub use apply::{Fixed, Report, check, fix};
pub use class::resolve;
pub use family::{ROOT, Tree};
pub use map::Map;

use crate::rules::Origin;
use crate::scan::Hit;

/// A character family, declared as a name and its fallback parent. The
/// chain ends at `ascii`, so resolution always terminates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    pub name: String,
    pub parent: Option<String>,
}

/// One member of an equivalence class: the family it belongs to and the
/// text it is written as. Text rather than char, because a member may be a
/// sequence of code points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub family: String,
    pub text: String,
}

/// An equivalence class: characters that mean the same thing, grouped by
/// family so each project can say which family it wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    pub members: Vec<Member>,
}

/// A plain transliteration entry. An empty `to` is an explicit delete,
/// which is the only way a character is ever removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntry {
    pub from: String,
    pub to: String,
    pub origin: Origin,
}

/// A rewrite that `fix` would apply: what was found, and what replaces it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    pub hit: Hit,
    pub to: String,
}

/// Why a map cannot be used, or a fix cannot be trusted.
///
/// Every variant is a refusal to produce text, never a partial rewrite: a
/// config error the caller turns into exit 2, or an invariant this node
/// checked on itself before anything could be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A map line that is not one of the declared forms, by 1-based number.
    Syntax { line: usize },
    /// A replacement is rewritten again without ever settling, so the map
    /// chains back to a source it already used.
    MapCycle,
    /// A family was named that nothing declares.
    UnknownFamily { name: String },
    /// A parent chain comes back to a family it already visited (V27).
    FamilyCycle { name: String },
    /// A family has no parent, so its chain never reaches `ascii` (V27).
    UnrootedFamily { name: String },
    /// `ascii` is the intrinsic root and cannot be given a parent (V27).
    RootReparented,
    /// `fix` would have changed a byte outside a violation (V6).
    TouchedAllowedBytes,
    /// `fix(fix(x))` would differ from `fix(x)` (V5).
    NotIdempotent,
}
