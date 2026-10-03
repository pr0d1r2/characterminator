//! The configuration a run is judged by, FILLED once from argv and a root.
//!
//! Every verb reads its configuration here, and so does `guard`: one
//! loader is what makes `--rule` mean the same thing to `check` as it
//! does to `fix`, and what makes a flag's origin the same `argv[<n>]` in
//! every report (`src/rules:V20`).
//!
//! This file owns argv -> sources and dotfile discovery, nothing more.
//! What the sources assemble into, and whether they are valid, is
//! `src/judge`'s [`Config`]. The flags are not a mode the chain knows
//! about: `--no-files` and the `--no-builtin-*` flags are sources this
//! loader does not add, and a `--rule` is one it does (`src/rules:V21`).

use super::args::{Args, Flag};
use crate::charset::builtin;
use crate::fix as engine;
use crate::judge::Config;
use crate::rules::Sources;
use std::path::{Path, PathBuf};

/// The rules file discovered at the run root (`src/rules:V45`).
pub(super) const RULES: &str = ".ctrm";

/// The sets file discovered beside it.
pub(super) const SETS: &str = ".ctrm-sets";

/// The map file discovered beside it.
pub(super) const MAP: &str = ".ctrm-map";

/// What `--pedantic` stands for, word for word (`src/lint/pedantic:V37`). A
/// rule line rather than a mode, so it sits in the chain at its argv position
/// and `explain` names that position as its origin.
pub(super) const PEDANTIC: &str = "* !pedantic=warn";

/// What one flag adds to the chain.
enum Added {
    Line(String),
    File(String, String),
    Fidelity(String),
}

/// Build the configuration from argv -- verb first, program name not
/// included -- and the working directory. The one entry point a verb,
/// `guard` included, needs.
///
/// # Errors
///
/// A flag that is not one, or a `--*-file` that cannot be read.
#[cfg(test)]
pub(crate) fn from_argv(cwd: &Path, argv: &[String]) -> Result<Config, String> {
    load(cwd, &super::args::parse(argv)?)
}

/// Build the configuration from flags already read.
///
/// # Errors
///
/// A `--*-file` that cannot be read.
pub(super) fn load(cwd: &Path, args: &Args) -> Result<Config, String> {
    let root = args
        .value("-C")
        .map_or_else(|| cwd.to_owned(), |d| cwd.join(d));
    let mut config = seeded(root, args);
    for flag in &args.flags {
        absorb(&mut config, flag)?;
    }
    Ok(config)
}

/// The run root for a caller standing at `cwd` with no `-C`: the git work
/// tree holding it, else `cwd` itself (`src/rules:V45`).
///
/// Shared with `guard`, whose payload names the cwd of the tool call: a
/// hook fired in `docs/` reads the same `.ctrm` as a run at the top.
/// Found LEXICALLY, as `src/tokens` finds the work tree: the first
/// ancestor holding `.git` (a directory, or a linked worktree's file), so
/// no git process is spawned to ask.
#[must_use]
pub(crate) fn root_of(cwd: &Path) -> PathBuf {
    let top = cwd.ancestors().find(|dir| dir.join(".git").exists());
    top.map_or_else(|| cwd.to_owned(), Path::to_path_buf)
}

/// Where a run stands (`src/cli:V115`): the base `load` resolves `-C`
/// against, with every path in argv moved from the caller's directory to
/// the root.
///
/// `-C` IS the root, as given, and its paths are already relative to it.
/// Without it the root is [`root_of`] the cwd, and a path typed in
/// `docs/` -- a named file or a `--*-file` -- is prefixed with `docs/`,
/// so what is matched and shown stays relative to the root.
#[must_use]
pub(super) fn situated(cwd: &Path, args: &mut Args) -> PathBuf {
    if args.has("-C") {
        return cwd.to_owned();
    }
    let root = root_of(cwd);
    let within = cwd.strip_prefix(&root).unwrap_or_else(|_| Path::new(""));
    if within.as_os_str().is_empty() {
        return root;
    }
    let moved = |word: &str| within.join(word).to_string_lossy().into_owned();
    args.paths = args.paths.iter().map(|path| moved(path)).collect();
    let files = args.flags.iter_mut().filter(|f| f.name.ends_with("-file"));
    for flag in files {
        flag.value = flag.value.as_deref().map(moved);
    }
    root
}

/// The configuration of a run given no flags: builtins and the dotfiles
/// at `root`, which is what every verb read before T55.
#[must_use]
pub(crate) fn discovered(root: &Path) -> Config {
    seeded(root.to_owned(), &Args::default())
}

/// The builtins and the discovered dotfiles, unless argv said otherwise.
fn seeded(root: PathBuf, args: &Args) -> Config {
    let sets = Some((builtin::SETS, "--no-builtin-sets"));
    let map = Some((engine::BUILTIN, "--no-builtin-map"));
    Config {
        rules: seed(&root, args, RULES, None),
        sets: seed(&root, args, SETS, sets),
        map: seed(&root, args, MAP, map),
        strict: args.has("--strict"),
        root,
    }
}

/// One kind's chain before any flag: its builtin unless the flag that
/// removes it was given, then its dotfile unless `--no-files` was.
///
/// An ABSENT dotfile is no error: it is a source this run does not have,
/// exactly as `--no-files` makes it one (`src/rules:V21`).
fn seed(
    root: &Path,
    args: &Args,
    name: &str,
    builtin: Option<(&str, &str)>,
) -> Sources {
    let mut sources = Sources::new();
    if let Some((text, _)) = builtin.filter(|(_, off)| !args.has(off)) {
        sources = sources.builtin(text);
    }
    let read = || std::fs::read_to_string(root.join(name)).ok();
    match (!args.has("--no-files")).then(read).flatten() {
        Some(text) => sources.dotfile(name, text),
        None => sources,
    }
}

/// One flag, into the chain of the kind it twins.
fn absorb(config: &mut Config, flag: &Flag) -> Result<(), String> {
    let Some(added) = added(&config.root, flag)? else {
        return Ok(());
    };
    let Some(sources) = kind(config, flag.name) else {
        return Ok(());
    };
    let held = std::mem::take(sources);
    *sources = match added {
        Added::Line(line) => held.flag(flag.index, line),
        Added::File(path, text) => held.file(path, text),
        Added::Fidelity(family) => held.fidelity(flag.index, &family),
    };
    Ok(())
}

/// What a flag contributes, reading a named file now so a missing one is
/// a usage error at load rather than a silent empty source.
fn added(root: &Path, flag: &Flag) -> Result<Option<Added>, String> {
    let value = flag.value.clone().unwrap_or_default();
    Ok(Some(match flag.name {
        "--rule" | "--set" | "--map" => Added::Line(value),
        "--pedantic" => Added::Line(PEDANTIC.to_owned()),
        "--fidelity" => Added::Fidelity(value),
        "--rules-file" | "--sets-file" | "--map-file" => {
            let text =
                std::fs::read_to_string(root.join(&value)).map_err(|e| {
                    format!("{value}: {}", crate::judge::plain(&e.to_string()))
                })?;
            Added::File(value, text)
        }
        _ => return Ok(None),
    }))
}

/// The chain a flag feeds, by the kind it twins (`src/rules:V18`).
fn kind<'a>(config: &'a mut Config, flag: &str) -> Option<&'a mut Sources> {
    match flag {
        "--rule" | "--rules-file" | "--fidelity" | "--pedantic" => {
            Some(&mut config.rules)
        }
        "--set" | "--sets-file" => Some(&mut config.sets),
        "--map" | "--map-file" => Some(&mut config.map),
        _ => None,
    }
}

/// T55 through the verbs: every flag reaches the same chain the dotfiles
/// do, and a run configured by argv alone reaches the same verdict.
#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
