//! Read text and locate characters: positions, UTF-8 validity, binary skip.
//!
//! See `src/scan/SPEC.md`. This file declares the node's public type
//! vocabulary and composes the node; it implements nothing, which is the
//! hub's rule for a `mod.rs` (`src/SPEC.md` CONSTRAINTS). The walk lives
//! in `text`, the refusals that come before it in `bytes`, and the order
//! a report goes out in in `order`.

mod bytes;
mod text;

pub use bytes::{Scanned, scan_bytes};
pub(crate) use bytes::{decode, decode_owned};
pub(crate) use text::located;
pub use text::scan_str;

/// Where a character sits, in the three units a reader or a tool needs.
///
/// Line and column are 1-based and the column counts CHARACTERS, because a
/// byte column is wrong for an editor the moment a line holds anything
/// multi-byte. The byte offset is kept beside them for tools that edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub byte: usize,
}

/// One located character. Whether it is ALLOWED is not decided here: this
/// node reports what is where, and the rules and lint nodes decide what it
/// means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Hit {
    pub position: Position,
    pub character: char,
}

/// Why a file could not be scanned. Neither case is silent: an unreadable
/// file that reported nothing would be indistinguishable from a clean one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unreadable {
    /// Not valid UTF-8: where decoding failed, as the 0-based byte offset
    /// and as the 1-based line and column (in code points, as a
    /// [`Position`] counts them) of that byte, so a reader can go to it.
    NotUtf8 {
        byte: usize,
        line: usize,
        column: usize,
    },
    /// Detected as binary, so it was skipped and named rather than read.
    Binary,
}
