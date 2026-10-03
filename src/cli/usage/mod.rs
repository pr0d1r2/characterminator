//! What a reader sees asking for help or getting the command wrong.
//!
//! See `src/cli/usage/SPEC.md`. Dispatch decides WHEN to show these and
//! with which exit code; this node owns the words.

use crate::judge::nearest;

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

example: a PreToolUse or PostToolUse hook whose command is `ctrm guard`
exit codes: 0 decided (the decision is in the JSON), 1 the adapter
failed; never 2, which a harness reads as \"block\"";

/// The surface, in one place so a test can hold it to the flag table.
pub(in crate::cli) const USAGE: &str =
    "ctrm -- eliminate characters outside an allowed set

  ctrm check [<path>...]         report characters outside the set
    [--summary] [--max <n>]      one row per file and code point; at most n
  ctrm explain [<path>]          the set in force, and the rule behind it
    [--as args|lines|prompt]     or the config as flags, files, a prompt
  ctrm sets [<name>...]          the presets, what each is for, members
    [--locales]                  the CLDR locale sets instead
    [--containing <c>]           only the sets holding c (or U+XXXX)
  ctrm fix [--check] [<path>...] rewrite them, or report the drift
  ctrm stats [--bpe] [<path>...] what they cost now, and after a fix
  ctrm init [--print]            draft a .ctrm from the tracked files
  ctrm guard                     agent hook: hook JSON in, decision out
  ctrm --help | -h               this text; also after any verb
  ctrm --version | -V            the version

configuration, any verb, repeatable, later wins:
  --rule <line>     one .ctrm line       --rules-file <f>
  --map <line>      one .ctrm-map line   --map-file <f>
  --set <line>      one .ctrm-sets line  --sets-file <f>
  --no-files  --no-builtin-map  --no-builtin-sets
  --fidelity <family>  --strict  --pedantic  --no-color  -C <dir>

any verb but guard takes --format json; check also takes --format sarif;
an unknown flag is refused; `--` ends the flags; guard reads no flags;
ctrm never prints colour: --no-color and NO_COLOR are accepted, no-ops";
