//! What a character set IS: membership, unions, presets.
//!
//! See `src/charset/SPEC.md`. This file composes and declares the public
//! vocabulary; the behaviour lives in the submodules. The CLDR locale
//! letters are the `locale` sub-node (`src/charset/locale/SPEC.md`),
//! private here: siblings reach them through the re-export below.

pub(crate) mod builtin;
pub(crate) mod compose;
mod locale;
pub(crate) mod parse;
pub(crate) mod range;
pub(crate) mod set;

pub(crate) use builtin::ASCII;
pub use compose::{ComposeError, SetCatalog};
pub(crate) use locale::{adopt, adopt_all};
pub(crate) use parse::{
    ParseError, SetDefinition, SetMember, code_points, parse_line,
};

/// An inclusive range of code points, the unit a set is built from.
///
/// Ranges rather than a set of chars because a preset like a coarse block
/// spans 128 code points and a CLDR letter set spans a handful: one shape
/// holds both without a second representation to keep in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharRange {
    pub start: char,
    pub end: char,
}

/// A named set of code points.
///
/// The name is what a rule line says (`caveman`, `pl`, `ascii`) and what
/// `explain` prints back, so it travels with the ranges rather than being
/// looked up again at the point of use.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CharSet {
    pub name: String,
    pub ranges: Vec<CharRange>,
}
