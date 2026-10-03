//! What a reader sees asking for help or getting the command wrong.
//!
//! See `src/cli/usage/SPEC.md`. Dispatch decides WHEN to show these and
//! with which exit code; this node owns the words.

mod help;

use crate::judge::nearest;
pub(in crate::cli) use help::{USAGE, verb_help};

/// Every verb, in the order the usage lists them.
pub(in crate::cli) const VERBS: &[&str] =
    &["check", "explain", "sets", "fix", "stats", "init", "guard"];

/// The one line every refusal of a command line ends with: where to look,
/// rather than the whole usage, which buries the line saying what was
/// wrong (V119).
pub(in crate::cli) const POINTER: &str =
    "run 'ctrm --help' for the verbs and flags";

/// A first word that is no verb (V119), with the nearest verb when one is
/// within two edits.
#[must_use]
pub(in crate::cli) fn unknown_verb(word: &str) -> String {
    let hint = nearest(word, VERBS.iter().copied())
        .map(|verb| format!(" (did you mean '{verb}'?)"))
        .unwrap_or_default();
    format!("unknown verb '{word}'{hint}\n{POINTER}")
}

/// Flags and no verb at all.
#[must_use]
pub(in crate::cli) fn missing_verb() -> String {
    format!("no verb given\n{POINTER}")
}

/// `ctrm guard --help`, the one word guard reads (`src/cli/guard:V93`).
pub(in crate::cli) const GUARD: &str = "ctrm guard -- agent hook adapter

reads one hook payload (JSON) on stdin and writes the hook decision on
stdout; the rules come from the dotfiles at the payload's cwd. Takes no
flags: any other word is ignored, so a hook line can never exit 2.

exit codes: 0 decided (the decision is in the JSON), 1 the adapter
failed; never 2, which a harness reads as \"block\"

example: ctrm guard < payload.json   (as a PreToolUse/PostToolUse hook)";
