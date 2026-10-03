//! `characterminator-dev`: the tooling that keeps this repository's
//! generated docs true, and ships to nobody (`dev:C`).
//!
//! `run` is the whole program with its world injected -- the arguments,
//! the repository root, the recorded answers that stand in for external
//! tools, and the error stream -- so every verb is testable against a
//! fixture repository instead of this one.

mod badges;
mod notices;
mod owners;
mod select;
mod splice;

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use owners::Pkg;
use select::{OUTPUTS, Output};

const USAGE: &str = "\
usage: characterminator-dev readme|notices [--check]
       characterminator-dev --check|--fix [<path>...]

  readme    the README `badges` block, from Cargo.toml, .coverage and cargo tree
  notices   the `closure` block of docs/THIRD-PARTY-NOTICES.md, from cargo tree
  --check   compare every output the paths can have changed; write nothing
  --fix     rewrite every output the paths can have changed

exit: 0 clean, 1 stale or an owner error, 2 usage";

/// Answers recorded in advance for the external tools, so a test needs no
/// cargo registry. `None` runs the real tool.
#[derive(Debug, Default, Clone)]
pub struct External {
    /// A file holding what `cargo tree` would print (`CTRM_DEV_CARGO_TREE`).
    pub cargo_tree: Option<PathBuf>,
}

impl External {
    /// The recorded answers the environment names.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            cargo_tree: std::env::var_os("CTRM_DEV_CARGO_TREE")
                .map(PathBuf::from),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Check,
    Write,
}

#[derive(Debug)]
enum Verb {
    One(&'static Output, Mode),
    Selected(Mode, Vec<String>),
    Help,
}

enum Outcome {
    Fresh,
    Wrote,
    Stale(Vec<String>),
}

/// Every owner's text, read once per run.
struct Sources {
    manifest: String,
    coverage: String,
    closure: BTreeSet<Pkg>,
}

struct Ctx<'a> {
    root: &'a Path,
    sources: Sources,
    mode: Mode,
}

/// The binary's entry: argv, the repository above the working directory,
/// and the environment's recorded answers.
#[must_use]
pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().ok().and_then(|dir| find_root(&dir));
    let code = run(
        &args,
        root.as_deref(),
        &External::from_env(),
        &mut std::io::stderr(),
    );
    ExitCode::from(code)
}

/// The nearest ancestor that is this kind of repository: a git work tree
/// with a `Cargo.toml` and a `SPEC.md` at its top.
fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|d| {
            d.join(".git").exists()
                && d.join("Cargo.toml").is_file()
                && d.join("SPEC.md").is_file()
        })
        .map(Path::to_path_buf)
}

/// Write `message` and hand back `code`, so every exit is one line.
fn fail(err: &mut dyn Write, code: u8, message: &str) -> u8 {
    let _ = writeln!(err, "{message}");
    code
}

/// The whole program. Returns the exit code (`dev:V137`).
pub fn run(
    args: &[String],
    root: Option<&Path>,
    ext: &External,
    err: &mut dyn Write,
) -> u8 {
    let Some(verb) = parse(args) else {
        return fail(err, 2, USAGE);
    };
    let Some(root) = root else {
        return fail(err, 2, NO_ROOT);
    };
    let (outputs, mode) = match verb {
        Verb::Help => return fail(err, 0, USAGE),
        Verb::One(output, mode) => (vec![output], mode),
        Verb::Selected(mode, paths) => (selected(root, &paths), mode),
    };
    run_outputs(&outputs, mode, (root, ext), err)
}

const NO_ROOT: &str = "characterminator-dev: not inside the repository \
                       (no .git, Cargo.toml and SPEC.md above here)";

fn parse(args: &[String]) -> Option<Verb> {
    let (verb, rest) = args.split_first()?;
    match (verb.as_str(), rest) {
        ("--check", paths) => selection(Mode::Check, paths),
        ("--fix", paths) => selection(Mode::Write, paths),
        ("help" | "-h" | "--help", []) => Some(Verb::Help),
        (name, []) => one(name, Mode::Write),
        (name, [flag]) if flag == "--check" => one(name, Mode::Check),
        _ => None,
    }
}

fn selection(mode: Mode, paths: &[String]) -> Option<Verb> {
    let plain = paths.iter().all(|p| !p.starts_with('-'));
    plain.then(|| Verb::Selected(mode, paths.to_vec()))
}

fn one(name: &str, mode: Mode) -> Option<Verb> {
    let output = OUTPUTS.iter().find(|o| o.name == name)?;
    Some(Verb::One(output, mode))
}

/// The outputs the paths select, each path taken relative to the root
/// whether the gate handed it in relative or absolute.
fn selected(root: &Path, paths: &[String]) -> Vec<&'static Output> {
    let relative: Vec<String> = paths
        .iter()
        .map(|p| match Path::new(p).strip_prefix(root) {
            Ok(rel) => rel.to_string_lossy().into_owned(),
            Err(_) => p.clone(),
        })
        .collect();
    select::selected(&relative)
}

/// Run each output and return the worst exit. Nothing selected is a clean
/// run that reads no owner, so an unrelated commit never spawns cargo.
fn run_outputs(
    outputs: &[&'static Output],
    mode: Mode,
    (root, ext): (&Path, &External),
    err: &mut dyn Write,
) -> u8 {
    if outputs.is_empty() {
        return 0;
    }
    match load(root, ext) {
        Err(m) => fail(err, 1, &format!("characterminator-dev: {m}")),
        Ok(sources) => worst(
            outputs,
            &Ctx {
                root,
                sources,
                mode,
            },
            err,
        ),
    }
}

fn worst(
    outputs: &[&'static Output],
    ctx: &Ctx<'_>,
    err: &mut dyn Write,
) -> u8 {
    outputs
        .iter()
        .map(|o| report(o, ctx, err))
        .max()
        .unwrap_or(0)
}

fn report(output: &Output, ctx: &Ctx<'_>, err: &mut dyn Write) -> u8 {
    match outcome(output, ctx) {
        Ok(Outcome::Fresh | Outcome::Wrote) => 0,
        Ok(Outcome::Stale(lines)) => fail(err, 1, &stale(output, &lines)),
        Err(message) => {
            let name = output.name;
            fail(err, 1, &format!("characterminator-dev: {name}: {message}"))
        }
    }
}

/// What a stale block is told: which, where, the fix, and a sample.
fn stale(output: &Output, lines: &[String]) -> String {
    let (name, marker, file) = (output.name, output.marker, output.file);
    let mut text = format!(
        "stale: {name} (`{marker}` in {file}); \
         run `cargo run -q -p characterminator-dev -- {name}`"
    );
    lines
        .iter()
        .for_each(|line| text.push_str(&format!("\n  {line}")));
    text
}

fn outcome(output: &Output, ctx: &Ctx<'_>) -> Result<Outcome, String> {
    let doc = read(ctx.root, output.file)?;
    let body = render(output, &ctx.sources)?;
    let new = splice::replace(&doc, output.marker, &body)
        .map_err(|m| format!("{}: {m}", output.file))?;
    if new == doc {
        return Ok(Outcome::Fresh);
    }
    if ctx.mode == Mode::Check {
        let have = splice::current(&doc, output.marker)?;
        return Ok(Outcome::Stale(splice::sample(&body, have)));
    }
    std::fs::write(ctx.root.join(output.file), new)
        .map_err(|e| format!("{}: {e}", output.file))?;
    Ok(Outcome::Wrote)
}

fn render(output: &Output, sources: &Sources) -> Result<String, String> {
    let direct = owners::dependency_count(&sources.manifest);
    match output.name {
        "notices" => notices::render(&sources.closure, direct),
        _ => facts(sources).map(|f| badges::render(&f)),
    }
}

/// An owner's value, or an error naming the owner (`dev:V131`).
fn need(value: Option<String>, owner: &str) -> Result<String, String> {
    value.ok_or_else(|| {
        format!("{owner} has no value, and a badge is never given a default")
    })
}

fn package(s: &Sources, key: &str) -> Result<String, String> {
    let value = owners::value(&s.manifest, "package", key);
    need(value, &format!("Cargo.toml [package] {key}"))
}

fn slug(s: &Sources) -> Result<String, String> {
    let repository = package(s, "repository")?;
    let slug = repository.strip_prefix("https://github.com/").ok_or(
        "Cargo.toml [package] repository is not a https://github.com/ URL",
    )?;
    Ok(slug.trim_end_matches('/').to_owned())
}

fn facts(s: &Sources) -> Result<badges::Facts, String> {
    let coverage = owners::coverage(&s.coverage);
    let unsafe_level = owners::unsafe_level(&s.manifest);
    Ok(badges::Facts {
        name: package(s, "name")?,
        slug: slug(s)?,
        licence: package(s, "license")?,
        edition: package(s, "edition")?,
        msrv: package(s, "rust-version")?,
        direct: owners::dependency_count(&s.manifest),
        closure: s.closure.len(),
        coverage: need(coverage, ".coverage `lines`")?,
        unsafe_level: need(unsafe_level, "Cargo.toml `unsafe_code`")?,
    })
}

fn read(root: &Path, file: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))
}

/// Every owner, read once.
fn load(root: &Path, ext: &External) -> Result<Sources, String> {
    let manifest = read(root, "Cargo.toml")?;
    let coverage = read(root, ".coverage")?;
    let name = owners::value(&manifest, "package", "name");
    let name = need(name, "Cargo.toml [package] name")?;
    let tree = cargo_tree(root, ext, &name)?;
    let closure = without_self(owners::closure(&tree), &name)?;
    Ok(Sources {
        manifest,
        coverage,
        closure,
    })
}

/// The package is the root of its own tree, not part of its closure. Its
/// absence means `cargo tree` answered about something else.
fn without_self(
    mut closure: BTreeSet<Pkg>,
    name: &str,
) -> Result<BTreeSet<Pkg>, String> {
    let before = closure.len();
    closure.retain(|p| p.name != name);
    if closure.len() == before {
        return Err(format!("`cargo tree` did not list `{name}` itself"));
    }
    Ok(closure)
}

/// What `cargo tree` prints for the package on every target (`dev:V132`),
/// or the recorded answer standing in for it.
fn cargo_tree(
    root: &Path,
    ext: &External,
    name: &str,
) -> Result<String, String> {
    match &ext.cargo_tree {
        Some(recorded) => std::fs::read_to_string(recorded)
            .map_err(|e| format!("{}: {e}", recorded.display())),
        None => spawn_tree(root, name),
    }
}

const TREE: [&str; 11] = [
    "-e",
    "normal",
    "--target",
    "all",
    "--prefix",
    "none",
    "-f",
    "{p}|{l}",
    "--locked",
    "--offline",
    "-p",
];

fn spawn_tree(root: &Path, name: &str) -> Result<String, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .arg("tree")
        .args(TREE)
        .arg(name)
        .current_dir(root)
        .output()
        .map_err(|e| missing_tool(&e))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr);
        return Err(format!("`cargo tree` failed: {}", why.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn missing_tool(e: &std::io::Error) -> String {
    format!(
        "cargo could not be spawned ({e}) -- a MISSING TOOL, not a finding: \
         enter the dev shell (`nix develop`, or `direnv reload`)"
    )
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
