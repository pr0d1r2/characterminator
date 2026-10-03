//! Dispatch: argv to a verb, a verb's answer to an exit code.
//!
//! `main.rs` is a shim over [`run`] (`src:V38`), so dispatch, usage and
//! exit codes are testable rather than reachable only by launching a
//! process.
//!
//! Every verb goes the same way: argv is read ONCE (`args`), the
//! configuration is built ONCE from it and the root (`config`), and the
//! verb gets both. A verb never looks at argv itself, which is what kept
//! a flag's value from being taken for a path in one verb and not another.

use super::args::{self, Args};
use super::{check, config, explain, fix, guard, init, out, stats};
use crate::judge::Config;
use crate::render::{self, Format, Shape};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

/// The three exits, named rather than spelled as bare integers at each
/// call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Nothing to report.
    Ok,
    /// A violation, or drift under a `--check` flag.
    Violation,
    /// The caller asked for something that is not a usage.
    Usage,
    /// A hook adapter that could not do its job: exit 1, which a harness
    /// reads as a NON-BLOCKING error it shows the user. Never 2, which
    /// Claude Code reads as "block the tool": a broken adapter answering 2
    /// would deny every read for the rest of the session
    /// (`src/cli/guard:V53`).
    Broken,
}

impl Outcome {
    /// The exit code the root spec's interface section fixes.
    #[must_use]
    pub(crate) fn code(self) -> ExitCode {
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
    verb: &'a str,
}

/// Parse argv and dispatch. The binary itself holds nothing.
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match verb_of(args) {
        Some("--version" | "-V") => version(),
        Some("--help" | "-h") => help().code(),
        // Before argv is parsed: a hook must never exit 2
        // (`src/cli/guard:V53`), even when its command line carries a flag
        // the parser would refuse.
        Some("guard") => guarded().code(),
        Some(
            verb @ ("check" | "explain" | "sets" | "fix" | "stats" | "init"),
        ) => invoked(verb, args).code(),
        _ => usage().code(),
    }
}

fn verb_of(args: &[String]) -> Option<&str> {
    args.first().map(String::as_str)
}

/// Read argv, build the configuration, run the verb.
fn invoked(verb: &str, argv: &[String]) -> Outcome {
    if asks_help(argv) {
        return help();
    }
    match prepared(verb, argv) {
        Ok((args, config, format)) => {
            let run = Run {
                args: &args,
                config: &config,
                format,
                verb,
            };
            dispatched(verb, &run)
        }
        Err(message) => refused(verb, argv, &message),
    }
}

/// `<verb> --help` (V101). Read through the flag table, so a `--help`
/// that is a VALUE (`--rule --help`) or a path after `--` is not a
/// request, and before the configuration loads, so a broken `.ctrm`
/// cannot stand between a reader and the usage that explains it.
fn asks_help(argv: &[String]) -> bool {
    args::parse(argv).is_ok_and(|read| read.has("--help") || read.has("-h"))
}

/// Everything a verb is handed. Anything wrong here is a usage error,
/// named, before any file is looked at.
fn prepared(
    verb: &str,
    argv: &[String],
) -> Result<(Args, Config, Format), String> {
    prepared_at(&cwd(), verb, argv)
}

/// [`prepared`] for a caller standing at `cwd`, which a test can choose.
fn prepared_at(
    cwd: &std::path::Path,
    verb: &str,
    argv: &[String],
) -> Result<(Args, Config, Format), String> {
    let mut parsed = args::parse(argv)?;
    arity(verb, &parsed.paths)?;
    let format = format_of(&parsed)?;
    if sarif_misused(verb, format) {
        return Err(sarif_refused(verb));
    }
    let base = config::situated(cwd, &mut parsed);
    let config = config::load(&base, &parsed)?;
    config.validate()?;
    Ok((parsed, config, format))
}

fn dispatched(verb: &str, run: &Run<'_>) -> Outcome {
    match verb {
        "check" => checked(run),
        "explain" => explained(run),
        "sets" => listed(run),
        "init" => said(run, init::run(run.config, run.args.has("--print"))),
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

/// How many paths a verb takes (V73): `explain` at most one, the rest any
/// number; `sets` takes set names instead, each of which must resolve. A
/// path past that is REFUSED: `explain a.md b.txt` used to answer for
/// `a.md` alone, which reads as an answer about both.
fn arity(verb: &str, paths: &[String]) -> Result<(), String> {
    let most = match verb {
        "explain" => 1,
        "init" => 0,
        _ => return Ok(()),
    };
    match paths.get(most) {
        Some(extra) => Err(format!(
            "`{verb}` takes at most {most} path(s); `{extra}` is one too many"
        )),
        None => Ok(()),
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

/// [`failed`], and under `--format json` the error as a document on
/// stdout as well (`src/render:V122`): a script that asked for json
/// parses the failure instead of an empty stream. Same exit 2.
fn failed_as(verb: &str, json: bool, message: &str) -> Outcome {
    if json {
        out::shown(&render::error(verb, message), Outcome::Usage);
    }
    failed(message)
}

/// A verb that failed after its arguments were read.
fn failed_in(run: &Run<'_>, message: &str) -> Outcome {
    failed_as(run.verb, run.format == Format::Json, message)
}

/// A command line refused before the run began. Whether json was asked
/// for is read from the flags when they parse, and from the raw words
/// when they do not, so `check --format json --stirct` still answers in
/// json.
fn refused(verb: &str, argv: &[String], message: &str) -> Outcome {
    let json = args::parse(argv).map_or_else(
        |_| argv.windows(2).any(|w| w == ["--format", "json"]),
        |read| read.value("--format") == Some("json"),
    );
    failed_as(verb, json, message)
}

fn cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn checked(run: &Run<'_>) -> Outcome {
    let shape = match shape_of(run) {
        Ok(shape) => shape,
        Err(message) => return failed_in(run, &message),
    };
    match check::shaped(run.config, &run.args.paths, run.format, shape) {
        Ok(report) => report_of(&report),
        Err(message) => failed_in(run, &message),
    }
}

/// `--summary` and `--max N` (`src/render:V124`). Refused where they
/// would be ignored: SARIF has one result per finding, and the json is the
/// whole contract, so `--max` is human only.
fn shape_of(run: &Run<'_>) -> Result<Shape, String> {
    let summary = run.args.has("--summary");
    let max = run.args.value("--max").map(count).transpose()?;
    let refused = (summary && run.format == Format::Sarif)
        || (max.is_some() && run.format != Format::Human);
    if refused {
        return Err(String::from(
            "--summary takes --format human or json; --max, human only",
        ));
    }
    Ok(Shape { summary, max })
}

fn count(word: &str) -> Result<usize, String> {
    word.parse()
        .map_err(|_| format!("--max takes a count of rows, not `{word}`"))
}

/// `explain` and `sets` REPORT and exit 0 whatever they find (V7): they
/// answer questions about configuration, so there is nothing to fail at.
/// A usage error is still a usage error.
fn said(run: &Run<'_>, answer: Result<String, String>) -> Outcome {
    match answer {
        Ok(text) => out::shown(&text, Outcome::Ok),
        Err(message) => failed_in(run, &message),
    }
}

/// `--as` asks for the configuration in another form (`src/cli/explain:V32`).
fn explained(run: &Run<'_>) -> Outcome {
    let (config, paths) = (run.config, &run.args.paths);
    said(
        run,
        match run.args.value("--as") {
            Some(form) => explain::exported(config, paths, run.format, form),
            None => explain::run(config, paths, run.format),
        },
    )
}

/// `sets` resolves at the family the run's rules give (`src/rules:V29`),
/// where `--fidelity` is already the rule `* @<f>`: reading the flag here
/// as well ignored a `--rule '* @emoji'` that meant the same (B30).
///
/// Its words are set NAMES, not paths (`src/cli/explain:V127`).
fn listed(run: &Run<'_>) -> Outcome {
    let asked = explain::Asked {
        names: &run.args.paths,
        locales: run.args.has("--locales"),
        containing: run.args.value("--containing"),
    };
    said(run, explain::sets(run.config, run.format, &asked))
}

/// `fix` WRITES unless `--check` is given, which is V7's split: the verb
/// that rewrites files is the one a caller names deliberately.
fn fixed(run: &Run<'_>) -> Outcome {
    let write = !run.args.has("--check");
    match fix::run(run.config, &run.args.paths, run.format, write) {
        Ok(report) => fix_report_of(&report),
        Err(message) => failed_in(run, &message),
    }
}

fn fix_report_of(report: &fix::Report) -> Outcome {
    let outcome = reported(&report.text, report.code);
    noted(&report.note);
    outcome
}

/// A report's one-line tally, on STDERR so stdout stays rows a script
/// can read (`src/render:V123`). Written after the rows, so a terminal
/// shows it last.
fn noted(note: &str) {
    if !note.is_empty() {
        eprintln!("{note}");
    }
}

/// `--bpe` asks for the real tokenizer. The default is the estimate,
/// which the figure itself declares (`src/tokens:V10`).
fn counted(run: &Run<'_>) -> Outcome {
    let bpe = run.args.has("--bpe");
    said(
        run,
        stats::run(run.config, &run.args.paths, run.format, bpe),
    )
}

/// `guard`: hook JSON on stdin, decision JSON on stdout
/// (`src/cli/guard:V35`). Arguments are ignored, so a flag added by
/// mistake cannot turn a hook into an exit 2.
fn guarded() -> Outcome {
    let mut stdin = Vec::new();
    let read = std::io::stdin().read_to_end(&mut stdin);
    adapted(match read {
        Ok(_) => guard::run(&stdin, &cwd()),
        Err(e) => Err(format!("stdin: {e}")),
    })
}

/// The decision travels in the JSON; the exit code only says whether the
/// adapter worked (`src/cli/guard:V53`). A failure is NAMED on stderr and
/// exits 1, which lets the tool call through rather than bricking the
/// session, and even a write that failed is mapped off 2 for the same
/// reason.
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
    let outcome = reported(&report.text, report.code);
    noted(&report.note);
    outcome
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

/// Asked for, the usage is the ANSWER: stdout, exit 0 (V101), so
/// `ctrm --help | less` pages it and a script probing for the tool does
/// not read a usage error.
fn help() -> Outcome {
    out::shown(USAGE, Outcome::Ok)
}

/// Exit 2 names the surface rather than pretending to offer it.
fn usage() -> Outcome {
    eprintln!("{USAGE}");
    Outcome::Usage
}

/// The surface, in one place so a test can hold it to the flag table.
pub(super) const USAGE: &str =
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
  ctrm --help | -h               this text; also after any verb but guard

configuration, any verb, repeatable, later wins:
  --rule <line>     one .ctrm line       --rules-file <f>
  --map <line>      one .ctrm-map line   --map-file <f>
  --set <line>      one .ctrm-sets line  --sets-file <f>
  --no-files  --no-builtin-map  --no-builtin-sets
  --fidelity <family>  --strict  --pedantic  --no-color  -C <dir>

any verb but guard takes --format json; check also takes --format sarif;
an unknown flag is refused; `--` ends the flags; guard reads no flags;
ctrm never prints colour: --no-color and NO_COLOR are accepted, no-ops";

#[cfg(test)]
#[path = "dispatch_test.rs"]
mod tests;
