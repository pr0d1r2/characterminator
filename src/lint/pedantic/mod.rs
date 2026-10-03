//! The pedantic group: the opt-in lints, their walks and claim order,
//! and the one facade for the Unicode crates (`ucd`, `src:C`).
//!
//! See `src/lint/pedantic/SPEC.md`. This file COMPOSES: the lints and
//! their walks live in `lints`, the Unicode table questions in `ucd`.
//! The constants the registry and the claim order name go to the parent
//! only; what a sibling node uses goes out through the parent.

mod lints;
mod ucd;

pub(crate) use lints::{
    CHAR_LINTS, LINE_LINTS, TEXT_LINTS, char_lints, line_hits, text_hits,
    unicode_space,
};
pub(in crate::lint) use lints::{
    CLAIM_ORDER, CONFUSABLE, CRLF, FINAL_NEWLINE, MIXED_SCRIPT, NFKC_COMPAT,
    NOT_NFC, TRAILING_WHITESPACE, UNICODE_SPACE,
};
