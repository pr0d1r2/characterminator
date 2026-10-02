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
mod tests {
    use super::{Config, MAP, PEDANTIC, RULES, SETS, from_argv};
    use crate::cli::{args, check, explain, fix};
    use crate::render::Format;
    use crate::rules::Rule;
    use std::path::{Path, PathBuf};

    /// Lines of every shape a file may hold: comments, a blank, a set the
    /// repo declares, a level, a delete-free map entry and an opt-in map.
    const RULES_TEXT: &str =
        "# rules\n*.md ascii+house\n\nlegal.md ascii+legal !warn\n";
    const SETS_TEXT: &str = "# mine\nhouse U+2261\n";
    const MAP_TEXT: &str = "# map\nU+2014 -\nuse words\n";

    /// IDENTICAL TO, an em dash, UP TACK and a copyright sign: one char
    /// the declared set grants, two the map rewrites, one it does not.
    const NOTES: &str = "a \u{2261} \u{2014} \u{22A5}x \u{00A9}\n";
    const LEGAL: &str = "\u{00A9} \u{2014}\n";

    fn fixture(name: &str, files: &[(&str, &str)]) -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(name);
        std::fs::create_dir_all(&root).ok()?;
        for (file, text) in files {
            std::fs::write(root.join(file), text).ok()?;
        }
        Some(root)
    }

    fn configured(name: &str) -> Option<PathBuf> {
        let files = [
            (RULES, RULES_TEXT),
            (SETS, SETS_TEXT),
            (MAP, MAP_TEXT),
            ("notes.md", NOTES),
            ("legal.md", LEGAL),
        ];
        fixture(name, &files)
    }

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    /// `flag line` for every line of `text`, comments and blanks included:
    /// V18 says those are flags too, and yield nothing.
    fn twins(flag: &str, text: &str) -> Vec<String> {
        let pairs = text.lines().map(|line| [flag, line].map(String::from));
        pairs.flatten().collect()
    }

    /// A report and its exit code, or why the run never reached one.
    type Verdict = Result<(String, u8), String>;

    /// What a gating verb reports, as json, and its code. `fix` runs as
    /// `--check`, so a fixture is never rewritten under another test.
    fn verdict(root: &Path, words: &[String]) -> Verdict {
        let config = from_argv(root, words)?;
        let paths = args::parse(words)?.paths;
        if words.first().is_some_and(|verb| verb == "fix") {
            let report = fix::run(&config, &paths, Format::Json, false)?;
            return Ok((report.text, report.code));
        }
        let report = check::run(&config, &paths, Format::Json)?;
        Ok((report.text, report.code))
    }

    /// Both gating verbs over both files, with `extra` flags.
    fn both(root: &Path, extra: &[String]) -> Vec<Verdict> {
        let run = |verb: &str| {
            let mut words = argv(&[verb, "notes.md", "legal.md"]);
            words.extend_from_slice(extra);
            verdict(root, &words)
        };
        vec![run("check"), run("fix")]
    }

    /// V18 at the CLI, all three kinds at once: `--no-files` plus one
    /// flag per line of each dotfile reaches the verdict the dotfiles do.
    #[test]
    fn the_dotfiles_equal_no_files_plus_one_flag_per_line() {
        let Some(root) = configured("ctrm-twins-fixture") else {
            return;
        };
        let mut flags = argv(&["--no-files"]);
        flags.extend(twins("--rule", RULES_TEXT));
        flags.extend(twins("--set", SETS_TEXT));
        flags.extend(twins("--map", MAP_TEXT));
        let from_files = both(&root, &[]);
        assert_eq!(from_files, both(&root, &flags));
        assert!(
            from_files
                .iter()
                .all(|r| r.as_ref().is_ok_and(|v| v.1 == 1))
        );
    }

    /// The same files, named rather than discovered.
    #[test]
    fn the_dotfiles_equal_no_files_plus_the_file_flags() {
        let Some(root) = configured("ctrm-file-twins-fixture") else {
            return;
        };
        let named = [
            "--no-files",
            "--rules-file",
            RULES,
            "--sets-file",
            SETS,
            "--map-file",
            MAP,
        ];
        assert_eq!(both(&root, &[]), both(&root, &argv(&named)));
    }

    /// A dotfile left out changes the verdict, so the equality above is
    /// not two runs that both ignored their configuration.
    #[test]
    fn no_files_alone_drops_what_the_dotfiles_granted() {
        let Some(root) = configured("ctrm-no-files-fixture") else {
            return;
        };
        let bare = both(&root, &argv(&["--no-files"]));
        assert_ne!(both(&root, &[]), bare);
    }

    /// `src/lint:V36`: `legal.md` warns about its em dash and passes;
    /// under `--strict` the same finding fails the run.
    #[test]
    fn strict_turns_a_warned_finding_into_a_failure() {
        let Some(root) = configured("ctrm-strict-fixture") else {
            return;
        };
        let plain = verdict(&root, &argv(&["check", "legal.md"]));
        assert!(plain.is_ok_and(|(text, code)| code == 0 && !text.is_empty()));
        let words = argv(&["check", "--strict", "legal.md"]);
        let strict = verdict(&root, &words);
        assert!(
            strict.is_ok_and(|(text, code)| code == 1 && text.contains("deny"))
        );
    }

    /// A rule with its origin forgotten, so two spellings compare.
    fn shape(rule: &Rule) -> String {
        format!(
            "{} {:?} {:?} {:?}",
            rule.pattern, rule.sets, rule.family, rule.levels
        )
    }

    fn shapes(root: &Path, words: &[&str]) -> Vec<String> {
        let config = from_argv(root, &argv(words)).unwrap_or_default();
        let rules = config.rules().unwrap_or_default();
        rules.iter().map(shape).collect()
    }

    /// `src/lint:V37`: `--pedantic` IS `--rule '* !pedantic=warn'`, at its
    /// own argv position, and it adds a level without revoking a grant.
    #[test]
    fn pedantic_is_the_rule_it_stands_for_and_revokes_nothing() {
        let Some(root) = configured("ctrm-pedantic-fixture") else {
            return;
        };
        let spelled = shapes(&root, &["check", "--rule", PEDANTIC]);
        assert_eq!(shapes(&root, &["check", "--pedantic"]), spelled);
        let words = argv(&["check", "--pedantic", "legal.md"]);
        let kept = verdict(&root, &words);
        assert!(kept.is_ok_and(
            |(text, code)| code == 0 && text.contains("ascii+legal")
        ));
    }

    /// `src/rules:V29` through the verb: the family flag is a rule line
    /// too, and it keeps the grant beneath it (`src/rules:V56`).
    #[test]
    fn fidelity_is_the_rule_it_stands_for_and_revokes_nothing() {
        let Some(root) = configured("ctrm-fidelity-flag-fixture") else {
            return;
        };
        let spelled = shapes(&root, &["check", "--rule", "* @emoji"]);
        assert_eq!(shapes(&root, &["check", "--fidelity", "emoji"]), spelled);
        let words = argv(&["check", "--fidelity", "emoji", "legal.md"]);
        assert!(verdict(&root, &words).is_ok_and(|(_, code)| code == 0));
    }

    /// `src/fix:V51` through the flag: the notation is left alone until
    /// `--map 'use words'` opts in, and then it becomes a word.
    #[test]
    fn the_words_map_works_through_the_map_flag() {
        let Some(root) = fixture("ctrm-words-flag-fixture", &[]) else {
            return;
        };
        let written = std::fs::write(root.join("notes.md"), "x \u{22A5}owns\n");
        let _ = std::fs::remove_file(root.join(MAP));
        assert!(written.is_ok());
        let words = argv(&["fix", "--map", "use words", "notes.md"]);
        let config = from_argv(&root, &words).unwrap_or_default();
        let asked = [String::from("notes.md")];
        let fixed =
            fix::run(&config, &asked, Format::Human, true).map(|r| r.text);
        let text = std::fs::read_to_string(root.join("notes.md"));
        assert_eq!(text.unwrap_or_default(), "x not owns\n", "{fixed:?}");
    }

    /// `src/rules:V20` at the CLI: `explain` names the flag that won by
    /// its position in the process argv.
    #[test]
    fn explain_names_a_flag_as_the_origin() {
        let Some(root) = fixture("ctrm-explain-flag-fixture", &[]) else {
            return;
        };
        let words = argv(&["explain", "--rule", "*.md caveman", "a.md"]);
        let config = from_argv(&root, &words).unwrap_or_default();
        let asked = [String::from("a.md")];
        let said = explain::run(&config, &asked, Format::Human);
        let said = said.unwrap_or_else(|why| why);
        assert!(said.contains("origin argv[3]"), "{said}");
    }

    /// `-C <dir>` moves the run root, and discovery with it
    /// (`src/rules:V45`).
    #[test]
    fn a_directory_flag_moves_the_root_and_the_discovery() {
        let Some(root) =
            fixture("ctrm-dash-c-fixture/sub", &[(RULES, "* box\n")])
        else {
            return;
        };
        let parent = root.parent().map(Path::to_path_buf).unwrap_or_default();
        let config = from_argv(&parent, &argv(&["check", "-C", "sub"]));
        let config: Config = config.unwrap_or_default();
        assert_eq!(config.root, root);
        assert_eq!(config.rules.lines(), vec!["* box"]);
    }

    fn refusal(words: &[&str]) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let loaded = from_argv(root, &argv(words)).and_then(|c| c.map());
        loaded.err().unwrap_or_default()
    }

    /// Each refusal names what it refused, and where it came from.
    #[test]
    fn a_bad_flag_value_is_refused_at_its_origin() {
        let missing = refusal(&["check", "--rules-file", "no-such.ctrm"]);
        assert!(missing.contains("no-such.ctrm"), "{missing}");
        let two = refusal(&["fix", "--map", "U+2014 -\nU+2013 -"]);
        assert!(two.contains("argv[3]") && two.contains("one line"), "{two}");
        let bad = refusal(&["fix", "--set", "x", "--map", "use nothing"]);
        assert!(bad.contains("argv[5]") && bad.contains("nothing"), "{bad}");
    }
}
