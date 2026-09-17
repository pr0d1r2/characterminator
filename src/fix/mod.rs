//! Rewrite a disallowed character SAFELY: map, families, classes.
//!
//! See `src/fix/SPEC.md`. Types only for now; the logic arrives with T10,
//! T26, T29, T30 and T33.

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
