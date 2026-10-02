//! What a character set IS: membership, unions, presets.
//!
//! See `src/charset/SPEC.md`. This file composes and declares the public
//! vocabulary; the behaviour lives in the submodules. The preset DATA is
//! still to come with T22, T23 and T32.

pub mod builtin;
pub mod compose;
pub mod locale;
pub mod parse;
pub mod range;
pub mod set;

pub use compose::{ComposeError, SetCatalog};
pub use parse::{ParseError, SetDefinition, SetMember, parse_line};

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
pub struct CharSet {
    pub name: String,
    pub ranges: Vec<CharRange>,
}
