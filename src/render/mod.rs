//! Say it once: the human table and the stable json contract.
//!
//! See `src/render/SPEC.md`. Types only for now; the logic arrives with T9.

/// Which rendering a caller wants. The json form is a stable contract; the
/// human form is cosmetic and may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Human,
    Json,
}
