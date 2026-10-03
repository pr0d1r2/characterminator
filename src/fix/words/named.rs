//! The named maps a `use <name>` line may read (V51).

/// The opt-in `words` map (V51): notation to the English it abbreviates.
///
/// NOT layered by default, which is the whole of `src/fix:V26`'s promise:
/// the builtin rewrites only what a writer never chose, and these symbols
/// carry meaning somebody typed on purpose. A `use words` line asks.
pub(in crate::fix) const WORDS: &str = include_str!("words.ctrm-map");

/// Every map a `use` line may name. A table rather than a match, so the
/// test that each one parses walks the same list the parser reads.
pub(in crate::fix) const NAMED: &[(&str, &str)] = &[("words", WORDS)];
