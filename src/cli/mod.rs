//! Argument dispatch, verbs and exit codes. Not a verb's logic.
//!
//! See `src/cli/SPEC.md`. `main.rs` is a shim over `run` (`src:V38`), so
//! dispatch, usage and exit codes are testable rather than reachable only
//! by launching a process.

mod check;
mod explain;
mod fix;
mod out;
mod stats;

use crate::render::Format;
use crate::rules::TEXT;
use std::path::PathBuf;
use std::process::ExitCode;

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

impl Outcome {
    /// The exit code the root spec's interface section fixes.
    #[must_use]
    pub fn code(self) -> ExitCode {
        match self {
            Self::Ok => ExitCode::SUCCESS,
            Self::Violation => ExitCode::from(1),
            Self::Usage => ExitCode::from(2),
        }
    }
}

/// Parse argv and dispatch. The binary itself holds nothing.
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match verb_of(args) {
        Some("--version" | "-V") => version(),
        Some(verb) if sarif_misused(verb, args) => sarif_refused(verb).code(),
        Some("check") => checked(args).code(),
        Some("explain") => explained(args).code(),
        Some("sets") => listed(args).code(),
        Some("fix") => fixed(args).code(),
        Some("stats") => counted(args).code(),
        _ => usage().code(),
    }
}

fn verb_of(args: &[String]) -> Option<&str> {
    args.first().map(String::as_str)
}

/// `--format json` picks the stable contract and `--format sarif` the
/// code-scanning log (`src/render:V50`); anything else is the human form,
/// which `src/render:V11` allows to change.
///
/// The VALUE is read positionally, as the word after the flag. Sniffing
/// argv for a bare `json` would make `ctrm check json` silently switch
/// contracts, and would read a path named `json` as a format.
fn format_of(args: &[String]) -> Format {
    match value_of(args, "--format") {
        Some("json") => Format::Json,
        Some("sarif") => Format::Sarif,
        _ => Format::Human,
    }
}

/// SARIF carries findings, and `check` is the only verb with any. Asked of
/// another verb it is REFUSED rather than answered with an empty log: a
/// log with no results reads as a clean run, and none was performed.
fn sarif_misused(verb: &str, args: &[String]) -> bool {
    verb != "check" && format_of(args) == Format::Sarif
}

fn sarif_refused(verb: &str) -> Outcome {
    eprintln!(
        "ctrm: --format sarif reports findings, and only `check` has them; \
         `{verb}` takes --format human or json"
    );
    Outcome::Usage
}

/// The paths a caller named, which reach untracked files
/// (`src/tokens:V9`). A flag and its value are not paths.
fn paths_of(args: &[String]) -> Vec<String> {
    let mut paths = Vec::new();
    let mut words = args.iter().skip(1);
    while let Some(word) = words.next() {
        if word == "--format" || word == "--fidelity" {
            words.next();
        } else if !word.starts_with('-') {
            paths.push(word.clone());
        }
    }
    paths
}

/// The word after a named flag, read positionally for the reason
/// `format_of` states: sniffing argv for a bare value would read a path
/// as a flag's argument.
fn value_of<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let mut words = args.iter();
    while let Some(word) = words.next() {
        if word == flag {
            return words.next().map(String::as_str);
        }
    }
    None
}

fn root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn checked(args: &[String]) -> Outcome {
    match check::run(&root(), &paths_of(args), format_of(args)) {
        Ok(report) => report_of(&report),
        Err(message) => {
            eprintln!("ctrm: {message}");
            Outcome::Usage
        }
    }
}

/// `explain` and `sets` REPORT and exit 0 whatever they find (V7): they
/// answer questions about configuration, so there is nothing to fail at.
/// A usage error is still a usage error.
fn said(answer: Result<String, String>) -> Outcome {
    match answer {
        Ok(text) => out::shown(&text, Outcome::Ok),
        Err(message) => {
            eprintln!("ctrm: {message}");
            Outcome::Usage
        }
    }
}

fn explained(args: &[String]) -> Outcome {
    said(explain::run(&root(), &paths_of(args), format_of(args)))
}

/// The fidelity a listing is resolved at (`src/charset:V41`), which
/// `--fidelity` names and `src/rules:V29` defaults to `text`.
fn listed(args: &[String]) -> Outcome {
    let family = value_of(args, "--fidelity").unwrap_or(TEXT);
    said(explain::sets(&root(), family, format_of(args)))
}

/// `fix` WRITES unless `--check` is given, which is V7's split: the verb
/// that rewrites files is the one a caller names deliberately.
fn fixed(args: &[String]) -> Outcome {
    let write = !args.iter().any(|word| word == "--check");
    match fix::run(&root(), &paths_of(args), format_of(args), write) {
        Ok(report) => fix_report_of(&report),
        Err(message) => {
            eprintln!("ctrm: {message}");
            Outcome::Usage
        }
    }
}

fn fix_report_of(report: &fix::Report) -> Outcome {
    reported(&report.text, report.code)
}

/// `--bpe` asks for the real tokenizer. The default is the estimate,
/// which the figure itself declares (`src/tokens:V10`).
fn counted(args: &[String]) -> Outcome {
    let bpe = args.iter().any(|word| word == "--bpe");
    said(stats::run(&root(), &paths_of(args), format_of(args), bpe))
}

fn report_of(report: &check::Report) -> Outcome {
    reported(&report.text, report.code)
}

/// A gating verb's report and verdict. Success is silence, so a clean run
/// writes nothing at all rather than an empty line.
fn reported(text: &str, code: u8) -> Outcome {
    let verdict = if code == 0 {
        Outcome::Ok
    } else {
        Outcome::Violation
    };
    if text.is_empty() {
        return verdict;
    }
    out::shown(text, verdict)
}

fn version() -> ExitCode {
    let name = env!("CARGO_PKG_NAME");
    out::shown(
        &format!("{name} {}", env!("CARGO_PKG_VERSION")),
        Outcome::Ok,
    )
    .code()
}

/// Exit 2 names the surface rather than pretending to offer it.
fn usage() -> Outcome {
    eprintln!(
        "ctrm -- eliminate characters outside an allowed set\n\n  \
         ctrm check [<path>...]       report characters outside the set\n  \
         ctrm explain [<path>]        the set in force, and the rule behind it\n  \
         ctrm sets [--fidelity <f>]   every declared set and what it holds\n  \
         ctrm fix [--check] [<path>...] rewrite them, or report the drift\n  \
         ctrm stats [--bpe] [<path>...] what they cost now, and after a fix\n\n\
         any verb takes --format json; check also takes --format sarif; \
         planned: guard"
    );
    Outcome::Usage
}

#[cfg(test)]
mod tests {
    use super::{
        Format, Outcome, format_of, paths_of, sarif_misused, sarif_refused,
        verb_of,
    };

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    #[test]
    fn the_three_outcomes_stay_distinct() {
        // Asserted on the OUTCOME, not on `ExitCode`'s Debug string: that
        // string is platform shaped and is no contract, so a test pinned
        // to it would pass here and break elsewhere for no reason.
        assert_ne!(Outcome::Ok, Outcome::Violation);
        assert_ne!(Outcome::Violation, Outcome::Usage);
        assert_ne!(Outcome::Ok, Outcome::Usage);
    }

    #[test]
    fn a_verb_is_the_first_word() {
        assert_eq!(verb_of(&argv(&["check"])), Some("check"));
        assert_eq!(verb_of(&argv(&[])), None);
    }

    #[test]
    fn json_is_asked_for_by_name() {
        assert_eq!(format_of(&argv(&["check"])), Format::Human);
        assert_eq!(
            format_of(&argv(&["check", "--format", "json"])),
            Format::Json
        );
    }

    #[test]
    fn sarif_is_asked_for_by_name() {
        let asked = argv(&["check", "--format", "sarif"]);
        assert_eq!(format_of(&asked), Format::Sarif);
        assert_eq!(paths_of(&asked), Vec::<String>::new());
    }

    #[test]
    fn sarif_is_refused_for_every_verb_but_check() {
        let check = argv(&["check", "--format", "sarif"]);
        assert!(!sarif_misused("check", &check));
        for verb in ["fix", "stats", "explain", "sets"] {
            let asked = argv(&[verb, "--format", "sarif"]);
            assert!(sarif_misused(verb, &asked), "{verb}");
            assert!(!sarif_misused(verb, &argv(&[verb])), "{verb}");
        }
        assert_eq!(sarif_refused("fix"), Outcome::Usage);
    }

    /// The whole path, on a tree of its own: a SARIF run reports the
    /// finding at its repo-relative uri and keeps `check`'s exit code.
    #[test]
    fn a_sarif_check_keeps_the_verdict_and_names_the_file() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("ctrm-sarif-fixture");
        // EM DASH, outside the `ascii` an unconfigured tree is held to.
        let written = std::fs::create_dir_all(&root)
            .and_then(|()| std::fs::write(root.join("a.md"), "a\u{2014}b\n"));
        if written.is_err() {
            return;
        }
        let asked = [String::from("a.md")];
        let report = super::check::run(&root, &asked, Format::Sarif);
        assert!(report.is_ok());
        let report = report.unwrap_or_else(|why| unreachable!("{why}"));
        assert_eq!(report.code, 1);
        let at = r#""uri":"a.md"},"region":{"startLine":1,"startColumn":2"#;
        assert!(report.text.contains(at), "{}", report.text);
    }

    #[test]
    fn a_bare_word_json_is_a_path_and_not_a_format() {
        // The first version sniffed argv for `json` anywhere, so this
        // silently switched contracts -- and the test above passed
        // regardless, which is what let it through.
        assert_eq!(format_of(&argv(&["check", "json"])), Format::Human);
        assert_eq!(paths_of(&argv(&["check", "json"])), vec!["json"]);
    }

    #[test]
    fn paths_are_the_words_that_are_not_flags_or_their_values() {
        assert_eq!(paths_of(&argv(&["check"])), Vec::<String>::new());
        assert_eq!(
            paths_of(&argv(&["check", "src", "--format", "json", "docs"])),
            vec!["src", "docs"]
        );
    }
}
