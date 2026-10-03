//! Count tokens and list files. The sole call site for `itok`.
//!
//! See `src/tokens/SPEC.md`. This module composes: the vocabulary below,
//! plus the two rules that give it meaning -- which files are counted
//! (`fileset`, V9) and how a figure says what it is (`count` and `label`,
//! V10).

mod count;
mod fileset;
mod label;

pub use count::{of_file, of_text};
pub use fileset::{lexical, select};

use std::path::PathBuf;

/// How a count was arrived at. It travels WITH the number because a figure
/// that does not say how it was measured invites being read as exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// The cheap proxy, printed with a tilde.
    Estimate,
    /// A real tokenizer's count.
    Bpe,
}

/// A token count and the method behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count {
    pub tokens: u64,
    pub method: Method,
}

/// A path that could not be counted, and why.
///
/// It exists so that a file nothing can read is a RESULT rather than a
/// number: zero is the honest count of an empty file, and reusing it for
/// "unknown" understates a total while looking like a measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// The path as this crate tried to read it.
    pub path: PathBuf,
    /// Why it could not be counted, in the reader's words.
    pub reason: String,
}
