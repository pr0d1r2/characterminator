//! Argument dispatch, verbs and exit codes. Not a verb's logic.
//!
//! See `src/cli/SPEC.md`. `main.rs` is a shim over `run` (`src:V38`), so
//! dispatch, usage and exit codes are testable rather than reachable only
//! by launching a process.
//!
//! Every verb goes the same way: argv is read ONCE (`args`), the
//! configuration is built ONCE from it and the root (`config`), and the
//! verb gets both. A verb never looks at argv itself, which is what kept
//! a flag's value from being taken for a path in one verb and not another.

mod args;
mod check;
mod config;
mod explain;
mod fix;
mod guard;
mod hook;
mod json;
mod out;
mod stats;

pub use config::{Config, from_argv};

use crate::render::Format;
use crate::rules::TEXT;
use args::Args;
use std::io::Read;
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
    /// A hook adapter that could not do its job: exit 1, which a harness
    /// reads as a NON-BLOCKING error it shows the user. Never 2, which
    /// Claude Code reads as "block the tool": a broken adapter answering 2
    /// would deny every read for the rest of the session (V53).
    Broken,
}

impl Outcome {
    /// The exit code the root spec's interface section fixes.
    #[must_use]
    pub fn code(self) -> ExitCode {
        match self {
            Self::Ok => ExitCode::SUCCESS,
            Self::Violation | Self::Broken => ExitCode::from(1),
            Self::Usage => ExitCode::from(2),
        }
    }
}

/// One verb's whole input: the flags as read, the configuration built
/// from them, and the output form.
struct Run<'a> {
    args: &'a Args,
    config: &'a Config,
    format: Format,
}

/// Parse argv and dispatch. The binary itself holds nothing.
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match verb_of(args) {
        Some("--version" | "-V") => version(),
        // Before argv is parsed: a hook must never exit 2 (V53), even when
        // its command line carries a flag the parser would refuse.
        Some("guard") => guarded().code(),
        Some(verb @ ("check" | "explain" | "sets" | "fix" | "stats")) => {
            invoked(verb, args).code()
        }
        _ => usage().code(),
    }
}

fn verb_of(args: &[String]) -> Option<&str> {
    args.first().map(String::as_str)
}

/// Read argv, build the configuration, run the verb.
fn invoked(verb: &str, argv: &[String]) -> Outcome {
    match prepared(verb, argv) {
        Ok((args, config, format)) => {
            let run = Run {
                args: &args,
                config: &config,
                format,
            };
            dispatched(verb, &run)
        }
        Err(message) => failed(&message),
    }
}

/// Everything a verb is handed. Anything wrong here is a usage error,
/// named, before any file is looked at.
fn prepared(
    verb: &str,
    argv: &[String],
) -> Result<(Args, Config, Format), String> {
    let parsed = args::parse(argv)?;
    let format = format_of(&parsed)?;
    if sarif_misused(verb, format) {
        return Err(sarif_refused(verb));
    }
    let config = config::load(&cwd(), &parsed)?;
    Ok((parsed, config, format))
}

fn dispatched(verb: &str, run: &Run<'_>) -> Outcome {
    match verb {
        "check" => checked(run),
        "explain" => explained(run),
        "sets" => listed(run),
        "fix" => fixed(run),
        _ => counted(run),
    }
}

/// `--format json` picks the stable contract and `--format sarif` the
/// code-scanning log (`src/render:V50`); no flag is the human form, which
/// `src/render:V11` allows to change.
///
/// Any other value is REFUSED. `--format jsn` falling back to the human
/// form would hand a script a report it cannot parse, from a run that
/// said nothing was wrong with how it was asked.
fn format_of(args: &Args) -> Result<Format, String> {
    match args.value("--format") {
        None | Some("human") => Ok(Format::Human),
        Some("json") => Ok(Format::Json),
        Some("sarif") => Ok(Format::Sarif),
        Some(other) => Err(format!("unknown format `{other}`")),
    }
}

/// SARIF carries findings, and `check` is the only verb with any. Asked of
/// another verb it is REFUSED rather than answered with an empty log: a
/// log with no results reads as a clean run, and none was performed.
fn sarif_misused(verb: &str, format: Format) -> bool {
    verb != "check" && format == Format::Sarif
}

fn sarif_refused(verb: &str) -> String {
    format!(
        "--format sarif reports findings, and only `check` has them; \
         `{verb}` takes --format human or json"
    )
}

/// A usage or configuration error, named. Exit 2 either way: the run
/// never reached a verdict about any file.
fn failed(message: &str) -> Outcome {
    eprintln!("ctrm: {message}");
    Outcome::Usage
}

fn cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn checked(run: &Run<'_>) -> Outcome {
    match check::run(run.config, &run.args.paths, run.format) {
        Ok(report) => report_of(&report),
        Err(message) => failed(&message),
    }
}

/// `explain` and `sets` REPORT and exit 0 whatever they find (V7): they
/// answer questions about configuration, so there is nothing to fail at.
/// A usage error is still a usage error.
fn said(answer: Result<String, String>) -> Outcome {
    match answer {
        Ok(text) => out::shown(&text, Outcome::Ok),
        Err(message) => failed(&message),
    }
}

fn explained(run: &Run<'_>) -> Outcome {
    said(explain::run(run.config, &run.args.paths, run.format))
}

/// The fidelity a listing is resolved at (`src/charset:V41`), which
/// `--fidelity` names and `src/rules:V29` defaults to `text`.
fn listed(run: &Run<'_>) -> Outcome {
    let family = run.args.value("--fidelity").unwrap_or(TEXT);
    said(explain::sets(run.config, family, run.format))
}

/// `fix` WRITES unless `--check` is given, which is V7's split: the verb
/// that rewrites files is the one a caller names deliberately.
fn fixed(run: &Run<'_>) -> Outcome {
    let write = !run.args.has("--check");
    match fix::run(run.config, &run.args.paths, run.format, write) {
        Ok(report) => fix_report_of(&report),
        Err(message) => failed(&message),
    }
}

fn fix_report_of(report: &fix::Report) -> Outcome {
    reported(&report.text, report.code)
}

/// `--bpe` asks for the real tokenizer. The default is the estimate,
/// which the figure itself declares (`src/tokens:V10`).
fn counted(run: &Run<'_>) -> Outcome {
    let bpe = run.args.has("--bpe");
    said(stats::run(run.config, &run.args.paths, run.format, bpe))
}

/// `guard`: hook JSON on stdin, decision JSON on stdout (V35). Arguments
/// are ignored, so a flag added by mistake cannot turn a hook into an
/// exit 2.
fn guarded() -> Outcome {
    let mut stdin = String::new();
    let read = std::io::stdin().read_to_string(&mut stdin);
    adapted(match read {
        Ok(_) => guard::run(&stdin, &cwd()),
        Err(e) => Err(format!("stdin: {e}")),
    })
}

/// The decision travels in the JSON; the exit code only says whether the
/// adapter worked (V53). A failure is NAMED on stderr and exits 1, which
/// lets the tool call through rather than bricking the session, and even
/// a write that failed is mapped off 2 for the same reason.
fn adapted(answer: Result<String, String>) -> Outcome {
    match answer {
        Ok(text) if text.is_empty() => Outcome::Ok,
        Ok(text) => match out::shown(&text, Outcome::Ok) {
            Outcome::Usage => Outcome::Broken,
            kept => kept,
        },
        Err(message) => {
            eprintln!("ctrm guard: {message}");
            Outcome::Broken
        }
    }
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
    eprintln!("{USAGE}");
    Outcome::Usage
}

/// The surface, in one place so a test can hold it to the flag table.
const USAGE: &str = "ctrm -- eliminate characters outside an allowed set

  ctrm check [<path>...]         report characters outside the set
  ctrm explain [<path>]          the set in force, and the rule behind it
  ctrm sets [--fidelity <f>]     every declared set and what it holds
  ctrm fix [--check] [<path>...] rewrite them, or report the drift
  ctrm stats [--bpe] [<path>...] what they cost now, and after a fix
  ctrm guard                     agent hook: hook JSON in, decision out

configuration, any verb, repeatable, later wins:
  --rule <line>     one .ctrm line       --rules-file <f>
  --map <line>      one .ctrm-map line   --map-file <f>
  --set <line>      one .ctrm-sets line  --sets-file <f>
  --no-files  --no-builtin-map  --no-builtin-sets
  --fidelity <family>  --strict  --pedantic  -C <dir>

any verb but guard takes --format json; check also takes --format sarif;
an unknown flag is refused; `--` ends the flags; guard reads no flags";

#[cfg(test)]
mod tests {
    use super::{
        Config, Format, Outcome, adapted, args, format_of, prepared,
        sarif_misused, sarif_refused, verb_of,
    };

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    /// The format argv asks for, or the refusal, as text.
    fn format(words: &[&str]) -> Result<Format, String> {
        format_of(&args::parse(&argv(words))?)
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

    /// V53: a failed adapter exits 1, which Claude Code reads as a
    /// non-blocking error, and never 2, which it reads as a block.
    #[test]
    fn a_failed_guard_is_broken_and_never_usage() {
        let failed = adapted(Err(String::from("hook input is not JSON")));
        assert_eq!(failed, Outcome::Broken);
        assert_eq!(adapted(Ok(String::new())), Outcome::Ok);
        assert_ne!(Outcome::Broken, Outcome::Usage);
    }

    #[test]
    fn a_verb_is_the_first_word() {
        assert_eq!(verb_of(&argv(&["check"])), Some("check"));
        assert_eq!(verb_of(&argv(&[])), None);
    }

    #[test]
    fn json_is_asked_for_by_name() {
        assert_eq!(format(&["check"]), Ok(Format::Human));
        assert_eq!(format(&["check", "--format", "json"]), Ok(Format::Json));
    }

    #[test]
    fn sarif_is_asked_for_by_name() {
        let asked = format(&["check", "--format", "sarif"]);
        assert_eq!(asked, Ok(Format::Sarif));
    }

    /// `--format jsn` falling back to the human form would hand a script
    /// text it cannot parse from a run that said nothing was wrong.
    #[test]
    fn an_unknown_format_is_refused() {
        let refused = format(&["check", "--format", "jsn"]);
        assert!(refused.is_err_and(|why| why.contains("jsn")));
    }

    #[test]
    fn sarif_is_refused_for_every_verb_but_check() {
        assert!(!sarif_misused("check", Format::Sarif));
        for verb in ["fix", "stats", "explain", "sets"] {
            assert!(sarif_misused(verb, Format::Sarif), "{verb}");
            assert!(!sarif_misused(verb, Format::Human), "{verb}");
            let asked = argv(&[verb, "--format", "sarif"]);
            let why = prepared(verb, &asked).err().unwrap_or_default();
            assert_eq!(why, sarif_refused(verb));
        }
    }

    /// A typo'd flag is a usage error before any file is read, not a run
    /// that quietly ignored it and reported a clean tree.
    #[test]
    fn an_unknown_flag_stops_the_run() {
        let asked = argv(&["check", "--stirct"]);
        let why = prepared("check", &asked).err().unwrap_or_default();
        assert!(why.contains("--stirct"), "{why}");
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
        let config = Config::discovered(&root);
        let report = super::check::run(&config, &asked, Format::Sarif);
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
        assert_eq!(format(&["check", "json"]), Ok(Format::Human));
        let read = args::parse(&argv(&["check", "json"]));
        assert_eq!(read.map(|a| a.paths), Ok(vec![String::from("json")]));
    }

    #[test]
    fn paths_are_the_words_that_are_not_flags_or_their_values() {
        let words = ["check", "src", "--format", "json", "docs"];
        let read = args::parse(&argv(&words)).map(|a| a.paths);
        let both = vec![String::from("src"), String::from("docs")];
        assert_eq!(read, Ok(both));
    }
}
