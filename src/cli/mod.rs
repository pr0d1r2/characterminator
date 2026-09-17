//! Argument dispatch, verbs and exit codes. Not a verb's logic.
//!
//! See `src/cli/SPEC.md`. Types only for now; the logic arrives with T8,
//! T11, T12, T34 and T37.

/// The command surface the root spec fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Check,
    Fix,
    Stats,
    Explain,
    Sets,
    Guard,
}

/// The three exits, named rather than spelled as bare integers at each
/// call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to report.
    Ok,
    /// A violation, or drift under a `--check` flag.
    Violation,
    /// The caller asked for something that is not a usage.
    Usage,
}
