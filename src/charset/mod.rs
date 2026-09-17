//! What a character set IS: membership, unions, presets.
//!
//! See `src/charset/SPEC.md`. Types only for now; the logic arrives with
//! T5, T22, T23, T25 and T32.

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
