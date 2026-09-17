//! How a figure says what it is (V10).
//!
//! The convention is `itok`'s and the labels are READ FROM it rather than
//! spelled again here: a tilde marks the cheap estimate, a real
//! tokenizer's count names its encoding, and every figure carries its
//! unit. Restating those strings would let this crate and the counter it
//! calls drift apart while both still looked right.

use crate::tokens::{Count, Error, Method};
use std::fmt;

/// The unit every figure carries: INPUT tokens, what text costs fed into
/// a model. itok keeps its own unit string private, so this is the one
/// label that cannot be read from the crate that defines it.
const UNIT: &str = "itok";

/// itok's tier for one of ours. itok owns both the tier's name and
/// whether its number is approximate; neither is decided here.
fn tier(method: Method) -> itok::render::Method {
    match method {
        Method::Estimate => itok::render::DUMMY,
        Method::Bpe => itok::render::O200K,
    }
}

impl fmt::Display for Count {
    /// `~123 itok (bytes/4)` or `123 itok (o200k)`. The tilde marks the
    /// estimate, the parenthesis names the method that produced the
    /// number, and neither tier can be read as the other.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tier = tier(self.method);
        let mark = if tier.approximate { "~" } else { "" };
        write!(f, "{mark}{} {UNIT} ({})", self.tokens, tier.label())
    }
}

impl fmt::Display for Error {
    /// The path first, then the reason: the shape a reader already scans
    /// for in compiler and linter output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(tokens: u64, method: Method) -> Count {
        Count { tokens, method }
    }

    /// V10: the cheap tier is MARKED as an estimate and names its unit.
    #[test]
    fn the_estimate_wears_its_tilde() {
        let shown = count(12, Method::Estimate).to_string();
        assert!(shown.starts_with('~'), "{shown}");
        assert!(shown.contains(UNIT), "{shown}");
    }

    /// V10: a real tokenizer's count names the encoding that produced it
    /// and drops the tilde -- it is a measurement, not a guess.
    #[test]
    fn a_real_count_names_its_encoding() {
        let shown = count(12, Method::Bpe).to_string();
        assert!(!shown.contains('~'), "{shown}");
        assert!(shown.contains("o200k"), "{shown}");
    }

    /// The property behind both: the tiers can never render alike, so a
    /// number can never be mistaken for the better-measured one.
    #[test]
    fn the_two_tiers_never_render_alike() {
        assert_ne!(
            count(12, Method::Estimate).to_string(),
            count(12, Method::Bpe).to_string()
        );
    }

    /// A failure reads like a tool's failure: path, then reason.
    #[test]
    fn a_failure_names_the_path_and_the_reason() {
        let err = Error {
            path: "a/b.rs".into(),
            reason: "nope".to_owned(),
        };
        assert_eq!(err.to_string(), "a/b.rs: nope");
    }
}
