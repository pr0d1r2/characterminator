//! What a figure counts, and how it was arrived at (V10).
//!
//! Two tiers, both `itok`'s: the cheap proxy over a file's bytes, and
//! o200k through a real tokenizer. The method rides along with every
//! number, so nothing downstream has to remember which tier ran.
//!
//! Only TEXT is counted: the caller reads the file, and an unreadable one
//! is its error, never a zero. Zero is what an empty file costs; spending
//! it on "could not read" would quietly understate every total.

use crate::tokens::{Count, Method};

/// Count text already in memory -- the "after" side of a rewrite, which
/// exists nowhere on disk to be read back.
#[must_use]
pub(crate) fn of_text(text: &str, method: Method) -> Count {
    let tokens = match method {
        Method::Estimate => itok::estimate::dummy(width(text)),
        Method::Bpe => itok::bpe::count(text),
    };
    Count { tokens, method }
}

/// A text's byte length, saturating. The cheap tier takes a `u64`, and a
/// cast that wrapped would report a smaller file than the one in hand.
fn width(text: &str) -> u64 {
    u64::try_from(text.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A count carries the method it was asked for. Without this the two
    /// tiers become one number wearing either label.
    #[test]
    fn a_count_carries_its_method() {
        assert_eq!(of_text("hi", Method::Bpe).method, Method::Bpe);
        let cheap = of_text("hi", Method::Estimate);
        assert_eq!(cheap.method, Method::Estimate);
    }
}
