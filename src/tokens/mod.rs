//! Count tokens and list files. The sole call site for `itok`.
//!
//! See `src/tokens/SPEC.md`. Types only for now; the logic arrives with the
//! stats verb's task.

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
