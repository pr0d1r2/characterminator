//! Read text and locate characters: positions, UTF-8 validity, binary skip.
//!
//! See `src/scan/SPEC.md`. Types only for now; the logic arrives with T7.

/// Where a character sits, in the three units a reader or a tool needs.
///
/// Line and column are 1-based and the column counts CHARACTERS, because a
/// byte column is wrong for an editor the moment a line holds anything
/// multi-byte. The byte offset is kept beside them for tools that edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub byte: usize,
}

/// One located character. Whether it is ALLOWED is not decided here: this
/// node reports what is where, and the rules and lint nodes decide what it
/// means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub position: Position,
    pub character: char,
}

/// Why a file could not be scanned. Neither case is silent: an unreadable
/// file that reported nothing would be indistinguishable from a clean one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreadable {
    /// Not valid UTF-8, with the byte offset where decoding failed.
    NotUtf8 { byte: usize },
    /// Detected as binary, so it was skipped and named rather than read.
    Binary,
}
