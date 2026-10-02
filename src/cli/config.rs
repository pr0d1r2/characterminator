//! The configuration a run is judged by, built ONCE from argv and a root.
//!
//! Every verb reads its configuration here, and so will `guard`: one
//! loader is what makes `--rule` mean the same thing to `check` as it
//! does to `fix`, and what makes a flag's origin the same `argv[<n>]` in
//! every report (`src/rules:V20`).
//!
//! The three kinds each travel as a `Sources` -- the precedence chain
//! `src/rules:V19` fixes -- and nothing is parsed until a verb asks. The
//! flags are not a mode the chain knows about: `--no-files` and the
//! `--no-builtin-*` flags are sources this loader does not add, and a
//! `--rule` is one it does (`src/rules:V21`).

use super::args::{self, Args, Flag};
use crate::charset::{self, SetCatalog, SetDefinition, builtin, locale};
use crate::fix::{self as engine, Map};
use crate::rules::{self, Place, Rule, Sources};
use std::path::{Path, PathBuf};

/// The rules file discovered at the run root (`src/rules:V45`).
pub const RULES: &str = ".ctrm";

/// The sets file discovered beside it.
pub const SETS: &str = ".ctrm-sets";

/// The map file discovered beside it.
pub const MAP: &str = ".ctrm-map";

/// What `--pedantic` stands for, word for word (`src/lint:V37`). A rule
/// line rather than a mode, so it sits in the chain at its argv position
/// and `explain` names that position as its origin.
pub const PEDANTIC: &str = "* !pedantic=warn";

/// One run's configuration, unparsed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// The run root: `-C <dir>` resolved against the working directory.
    pub root: PathBuf,
    pub rules: Sources,
    pub sets: Sources,
    pub map: Sources,
    /// `--strict`: warn counts as deny (`src/lint:V36`). A flag with no
    /// line twin, because a level is per rule and this is per RUN.
    pub strict: bool,
}

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
pub fn from_argv(cwd: &Path, argv: &[String]) -> Result<Config, String> {
    load(cwd, &args::parse(argv)?)
}

/// Build the configuration from flags already read.
///
/// # Errors
///
/// A `--*-file` that cannot be read.
pub fn load(cwd: &Path, args: &Args) -> Result<Config, String> {
    let root = args
        .value("-C")
        .map_or_else(|| cwd.to_owned(), |d| cwd.join(d));
    let mut config = seeded(root, args);
    for flag in &args.flags {
        config.absorb(flag)?;
    }
    Ok(config)
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

impl Config {
    /// The configuration of a run given no flags: builtins and the
    /// dotfiles at `root`, which is what every verb read before T55.
    #[must_use]
    pub fn discovered(root: &Path) -> Self {
        seeded(root.to_owned(), &Args::default())
    }

    /// One flag, into the chain of the kind it twins.
    fn absorb(&mut self, flag: &Flag) -> Result<(), String> {
        let Some(added) = self.added(flag)? else {
            return Ok(());
        };
        let Some(sources) = self.kind(flag.name) else {
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

    /// What a flag contributes, reading a named file now so a missing one
    /// is a usage error at load rather than a silent empty source.
    fn added(&self, flag: &Flag) -> Result<Option<Added>, String> {
        let value = flag.value.clone().unwrap_or_default();
        Ok(Some(match flag.name {
            "--rule" | "--set" | "--map" => Added::Line(value),
            "--pedantic" => Added::Line(PEDANTIC.to_owned()),
            "--fidelity" => Added::Fidelity(value),
            "--rules-file" | "--sets-file" | "--map-file" => {
                let text = std::fs::read_to_string(self.root.join(&value))
                    .map_err(|e| format!("{value}: {e}"))?;
                Added::File(value, text)
            }
            _ => return Ok(None),
        }))
    }

    /// The chain a flag feeds, by the kind it twins (`src/rules:V18`).
    fn kind(&mut self, flag: &str) -> Option<&mut Sources> {
        match flag {
            "--rule" | "--rules-file" | "--fidelity" | "--pedantic" => {
                Some(&mut self.rules)
            }
            "--set" | "--sets-file" => Some(&mut self.sets),
            "--map" | "--map-file" => Some(&mut self.map),
            _ => None,
        }
    }

    /// Parse all three kinds once, whatever the verb will read (V74).
    ///
    /// A verb reads only the kinds it needs, so before this a broken map
    /// was refused by `fix` and `stats` and silently ignored by `check`,
    /// which then reported on a run whose configuration did not parse
    /// (B29). A configuration is one thing, and it is valid or it is not.
    ///
    /// # Errors
    ///
    /// The first line of any kind that does not parse, at its origin.
    pub fn validate(&self) -> Result<(), String> {
        self.catalog()?;
        let map = self.map()?;
        for rule in self.rules()? {
            declared(&map, &rule)?;
        }
        Ok(())
    }

    /// The rules, in precedence order.
    ///
    /// # Errors
    ///
    /// A line that does not parse, named at its origin.
    pub fn rules(&self) -> Result<Vec<Rule>, String> {
        self.rules.rules().map_err(|bad| bad.to_string())
    }

    /// The sets a run resolves against: the intrinsic `ascii`, then every
    /// declared set in precedence order, a later one replacing an earlier
    /// one of the same name (`src/rules:V19`).
    ///
    /// A repo whose files hold characters no preset covers declares a set
    /// of its own rather than reaching for `any`, which is the difference
    /// between a grant somebody wrote down and a check switched off.
    ///
    /// # Errors
    ///
    /// A sets line that does not parse, named at its origin.
    ///
    /// The CLDR locale sets come in only for the names the rules use
    /// (`src/charset:V61`), and only while the builtin sets are on.
    pub fn catalog(&self) -> Result<SetCatalog, String> {
        let mut catalog = self.declared()?;
        if self.sets.has_builtin() {
            let wanted = self.rules()?.into_iter().flat_map(|rule| rule.sets);
            locale::adopt(&mut catalog, wanted)
                .map_err(|bad| bad.to_string())?;
        }
        Ok(catalog)
    }

    /// The same with EVERY locale set in, which is what `sets` lists.
    ///
    /// # Errors
    ///
    /// As [`Config::catalog`].
    pub fn listing(&self) -> Result<SetCatalog, String> {
        let mut catalog = self.declared()?;
        if self.sets.has_builtin() {
            locale::adopt_all(&mut catalog).map_err(|bad| bad.to_string())?;
        }
        Ok(catalog)
    }

    /// `ascii` and every set the chain declares, without the locales.
    fn declared(&self) -> Result<SetCatalog, String> {
        let declared = self.sets.assemble(set_line);
        let declared = declared.map_err(|bad| bad.to_string())?;
        let mut catalog = builtin::intrinsic_catalog();
        for definition in declared {
            catalog.insert(definition);
        }
        Ok(catalog)
    }

    /// The map in force, each source layered over the last.
    ///
    /// A map is layered as WHOLE texts rather than entry by entry: a
    /// `family` line declares a tree a later line may use. A flag value
    /// still goes through `parse_flag` first, so `--map` refuses a second
    /// line exactly as `--rule` does (`src/rules:V18`).
    ///
    /// # Errors
    ///
    /// A map line that does not parse, or a tree that does not validate,
    /// named at the source it came from.
    pub fn map(&self) -> Result<Map, String> {
        let mut map = Map::default();
        for (place, text) in self.map.layers() {
            let line = one_line(place, text)?;
            let named = |bad: engine::Error| format!("{}: {bad}", at(place));
            map = map.layer(&line, &|n| place.origin(n)).map_err(named)?;
        }
        Ok(map)
    }
}

/// A rule's family is one the map's family tree declares
/// (`src/rules:V29`, `src/fix:V27`). An undeclared one used to pass as a
/// name and resolve to the unlabelled members alone, so `--fidelity
/// emjoi` was a typo that quietly meant something (B30).
fn declared(map: &Map, rule: &Rule) -> Result<(), String> {
    let Some(family) = &rule.family else {
        return Ok(());
    };
    match map.tree().path(family) {
        Ok(_) => Ok(()),
        Err(bad) => Err(format!("{}: {bad}", rules::describe(&rule.origin))),
    }
}

/// A source's text, or a flag's value once it has passed the one-line
/// rule. Blank and comment values pass through as the nothing they are.
fn one_line(place: Place<'_>, text: &str) -> Result<String, String> {
    let Place::Argument(index) = place else {
        return Ok(text.to_owned());
    };
    let kept = rules::parse_flag(text, index, |line, _| Ok(line.to_owned()));
    let kept = kept.map_err(|bad| bad.to_string())?;
    Ok(kept.unwrap_or_default())
}

/// A source named the way `src/rules:V20` spells an origin, less the
/// line: a map refusal already carries its own line number.
fn at(place: Place<'_>) -> String {
    match place {
        Place::Builtin => String::from("builtin"),
        Place::File(path) => path.display().to_string(),
        Place::Argument(index) => format!("argv[{index}]"),
    }
}

/// One sets line, at the origin the chain gave it.
///
/// The grammar's parser belongs to `src/charset` and the precedence chain
/// to `src/rules`, and neither calls the other: the parser travels as an
/// argument, so this adapter is the one place their two error types meet.
fn set_line(
    line: &str,
    origin: rules::Origin,
) -> Result<SetDefinition, rules::ParseError> {
    match charset::parse_line(line) {
        Ok(Some(declared)) => Ok(declared),
        // The chain skips blank and comment lines before calling this, so
        // a line declaring nothing cannot arrive here.
        Ok(None) => Err(rules::error(origin, "declares no set")),
        Err(bad) => Err(rules::error(origin, bad.to_string())),
    }
}

/// T55 through the verbs: every flag reaches the same chain the dotfiles
/// do, and a run configured by argv alone reaches the same verdict.
#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
