//! `explain --as args|lines|prompt` (T34): the effective configuration,
//! written back out in a form something else can take in (`V32`).
//!
//! `args` and `lines` are the SAME content in the two spellings V18 makes
//! equal: every entry-bearing line a run contributed -- dotfiles, named
//! files, flags -- in precedence order, as flags or as data-file lines.
//! What the BUILTINS hold is not repeated: it ships in the binary, every
//! run that keeps it gets it back, and whether this run kept it is said
//! with the `--no-builtin-*` flag that removed it.
//!
//! Asked about a path, the rules are narrowed to the ones that MATCH it,
//! all of them and in order: a family or a level can come from a rule
//! that does not decide the set (`src/rules:V56`), so the winner alone
//! would not reproduce the path's resolution. Sets and the map are kept
//! whole, because a set may be built from another set and a map line may
//! declare a family a later line uses.

use crate::cli::config::{MAP, RULES, SETS};
use crate::judge::Config;
use crate::rules;

/// The three forms V32 names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    Args,
    Lines,
    Prompt,
}

impl Shape {
    /// The form `--as` names.
    ///
    /// # Errors
    ///
    /// Any other word. A form the tool does not have is refused rather
    /// than answered with the plain report, which a script would then try
    /// to parse as the form it asked for.
    pub(super) fn named(word: &str) -> Result<Self, String> {
        match word {
            "args" => Ok(Self::Args),
            "lines" => Ok(Self::Lines),
            "prompt" => Ok(Self::Prompt),
            _ => Err(format!("unknown --as `{word}`: args, lines or prompt")),
        }
    }
}

/// The run-wide flags that have no line twin, as the flags themselves.
///
/// `--no-files` is not here: the `args` form always carries it, because
/// the dotfiles' lines are already in the output and reading them again
/// would apply them twice.
fn switches(config: &Config) -> Vec<&'static str> {
    let mut flags = Vec::new();
    if !config.map.has_builtin() {
        flags.push("--no-builtin-map");
    }
    if !config.sets.has_builtin() {
        flags.push("--no-builtin-sets");
    }
    if config.strict {
        flags.push("--strict");
    }
    flags
}

/// The rule lines in force, narrowed to those matching `path` if one was
/// asked about.
///
/// Lines and parsed rules are paired by position: both come from the same
/// sources in the same order, one rule per entry-bearing line, and rules
/// have no builtin source to make the two lists differ.
///
/// # Errors
///
/// A rule line that does not parse, named at its origin.
pub(super) fn rule_lines(
    config: &Config,
    path: Option<&str>,
) -> Result<Vec<String>, String> {
    let parsed = config.rules()?;
    let paired = config.rules.lines().into_iter().zip(parsed);
    let kept = paired.filter(|(_, rule)| {
        path.is_none_or(|path| rules::matches(&rule.pattern, path))
    });
    Ok(kept.map(|(line, _)| line).collect())
}

/// One word, quoted for a POSIX shell: single quotes, and a single quote
/// inside closed, escaped and reopened. Every value is quoted, so a line
/// holding a glob, a blank or a `!` survives a paste unexpanded.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// `--as args`: the configuration as flags, one per output line, joined
/// by line continuations so the block pastes after `ctrm <verb>`.
///
/// # Errors
///
/// As [`rule_lines`].
pub(super) fn args(
    config: &Config,
    path: Option<&str>,
) -> Result<String, String> {
    let mut words = vec![String::from("--no-files")];
    words.extend(switches(config).into_iter().map(String::from));
    let flagged = |flag: &str, lines: Vec<String>| -> Vec<String> {
        let each = lines.into_iter();
        each.map(|line| format!("{flag} {}", quoted(&line)))
            .collect()
    };
    words.extend(flagged("--rule", rule_lines(config, path)?));
    words.extend(flagged("--set", config.sets.lines()));
    words.extend(flagged("--map", config.map.lines()));
    Ok(words.join(" \\\n"))
}

/// `--as lines`: the configuration as the three data files, each under a
/// comment naming the file it would be.
///
/// A run-wide flag has no line to be written as, so it is NAMED in a
/// leading comment rather than dropped: a file set that quietly lost
/// `--strict` would describe a different run.
///
/// # Errors
///
/// As [`rule_lines`].
pub(super) fn lines(
    config: &Config,
    path: Option<&str>,
) -> Result<String, String> {
    let mut out = Vec::new();
    let flags = switches(config);
    if !flags.is_empty() {
        out.push(format!("# with: {}", flags.join(" ")));
    }
    let sections = [
        (RULES, rule_lines(config, path)?),
        (SETS, config.sets.lines()),
        (MAP, config.map.lines()),
    ];
    for (file, held) in sections {
        out.push(format!("# {file}"));
        out.extend(held);
    }
    Ok(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{Shape, args, lines};
    use crate::charset::builtin;
    use crate::cli::config::from_argv;
    use crate::cli::testkit::argv;
    use crate::fix::{self as engine, MapEntry};
    use crate::judge::Config;
    use crate::rules::corpus::{Noise, SHAPES, file};
    use crate::rules::{Origin, Rule, Sources};
    use std::path::Path;

    /// Map lines of every kind the round trip has to carry: a plain
    /// entry, a delete, a quote that the shell quoting has to survive, an
    /// opt-in map, a word entry, a comment and a blank.
    const MAP_SHAPES: [&str; 8] = [
        "U+2014 --",
        "U+200B",
        "U+2019 '",
        "use words",
        "word U+2234 thus",
        "# typography",
        "",
        "U+2026 ...",
    ];

    /// Sets lines: a declaration, one built from another, a range.
    const SET_SHAPES: [&str; 5] = [
        "house U+2261",
        "spec caveman house",
        "# mine",
        "",
        "wide U+2500-U+257F",
    ];

    /// Shell words, the way a POSIX shell splits the `args` output: blank
    /// separates, single quotes hold anything, a backslash escapes the
    /// next character, and a backslash-newline is a continuation.
    fn words(text: &str) -> Vec<String> {
        let mut lexer = Lexer::default();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            lexer.take(c, &mut chars);
        }
        lexer.finish()
    }

    #[derive(Default)]
    struct Lexer {
        words: Vec<String>,
        word: Option<String>,
    }

    impl Lexer {
        fn take(&mut self, c: char, rest: &mut std::str::Chars<'_>) {
            match c {
                '\'' => {
                    let quoted: String =
                        rest.by_ref().take_while(|q| *q != '\'').collect();
                    self.push(&quoted);
                }
                '\\' => match rest.next() {
                    Some('\n') | None => {}
                    Some(next) => self.push(&next.to_string()),
                },
                ' ' | '\n' => self.end(),
                _ => self.push(&c.to_string()),
            }
        }

        fn push(&mut self, text: &str) {
            self.word.get_or_insert_with(String::new).push_str(text);
        }

        fn end(&mut self) {
            self.words.extend(self.word.take());
        }

        fn finish(mut self) -> Vec<String> {
            self.end();
            self.words
        }
    }

    /// A configuration whose dotfiles are generated, plus the run-wide
    /// flags the sequence chooses.
    fn generated(noise: &mut Noise, count: usize) -> Config {
        let sets = maybe(noise, builtin::SETS);
        let map = maybe(noise, engine::BUILTIN);
        let rules = file(noise, &SHAPES, count);
        Config {
            rules: Sources::new().dotfile(".ctrm", rules),
            sets: sets.dotfile(".ctrm-sets", file(noise, &SET_SHAPES, count)),
            map: map.dotfile(".ctrm-map", file(noise, &MAP_SHAPES, count)),
            strict: flip(noise),
            root: Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        }
    }

    fn flip(noise: &mut Noise) -> bool {
        noise.next().checked_rem(2) == Some(0)
    }

    /// A chain with its builtin, or without it as `--no-builtin-*` would
    /// leave it, as the sequence chooses.
    fn maybe(noise: &mut Noise, text: &str) -> Sources {
        if flip(noise) {
            Sources::new().builtin(text)
        } else {
            Sources::new()
        }
    }

    fn unsourced(rule: &Rule) -> Rule {
        let mut rule = rule.clone();
        rule.origin = Origin::Builtin { line: 0 };
        rule
    }

    fn entry(entry: &MapEntry) -> (String, String, bool) {
        (entry.from.clone(), entry.to.clone(), entry.word)
    }

    /// Everything a configuration MEANS, with every origin forgotten:
    /// the origins necessarily differ (`.ctrm:4` against `argv[9]`), and
    /// V20 is what makes them differ. A `Debug` string, so one assertion
    /// compares the whole of it and prints the whole of it on failure.
    fn meaning(config: &Config) -> String {
        let rules = config.rules().unwrap_or_default();
        let rules: Vec<Rule> = rules.iter().map(unsourced).collect();
        let map = config.map().unwrap_or_default();
        let entries: Vec<_> = map.entries().iter().map(entry).collect();
        let catalog = config.catalog().ok();
        let classes = map.classes();
        format!(
            "{rules:?} {catalog:?} {entries:?} {classes:?} {:?}",
            config.strict
        )
    }

    /// V32's round trip, over the V18 corpus: `--as args`, split the way
    /// a shell splits it and handed back to a verb, is the same
    /// configuration.
    #[test]
    fn args_fed_back_are_the_same_configuration() {
        let mut noise = Noise(0xA265);
        for count in 0..48 {
            let config = generated(&mut noise, count);
            let rendered = args(&config, None).unwrap_or_default();
            let mut argv = vec![String::from("check")];
            argv.extend(words(&rendered));
            let back = from_argv(&config.root, &argv);
            assert!(back.is_ok(), "{back:?}\n{rendered}");
            let back = back.unwrap_or_default();
            assert_eq!(meaning(&back), meaning(&config), "{rendered}");
        }
    }

    /// The property is not vacuous: the generated configurations hold
    /// entries, and the meaning sees a difference when one is dropped.
    #[test]
    fn the_round_trip_compares_something() {
        let mut noise = Noise(0xA265);
        let config = generated(&mut noise, 24);
        assert!(config.rules().is_ok_and(|rules| rules.len() > 8));
        let mut fewer = config.clone();
        fewer.rules = Sources::new();
        assert_ne!(meaning(&fewer), meaning(&config));
    }

    fn loaded(words: &[&str]) -> Config {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let words = [&["explain", "--no-files"], words].concat();
        from_argv(root, &argv(&words)).unwrap_or_default()
    }

    #[test]
    fn args_quote_every_value_and_name_every_switch() {
        let config =
            loaded(&["--strict", "--no-builtin-map", "--rule", "*.md it's"]);
        let said = args(&config, None).unwrap_or_default();
        let expected = "--no-files \\\n--no-builtin-map \\\n--strict \\\n\
                        --rule '*.md it'\\''s'";
        assert_eq!(said, expected);
    }

    /// The lines form: a section per file, and a run-wide flag named
    /// rather than lost.
    #[test]
    fn lines_write_each_kind_under_its_file() {
        let config =
            loaded(&["--pedantic", "--set", "house U+2261", "--strict"]);
        let said = lines(&config, None).unwrap_or_default();
        let expected = "# with: --strict\n# .ctrm\n* !pedantic=warn\n\
                        # .ctrm-sets\nhouse U+2261\n# .ctrm-map";
        assert_eq!(said, expected);
    }

    /// Asked about a path, only the rules matching it are written, and
    /// all of them: the family below comes from a rule that is not the
    /// winner.
    #[test]
    fn a_path_narrows_the_rules_to_those_matching_it() {
        let rules = [
            "--rule",
            "* @emoji",
            "--rule",
            "*.md caveman",
            "--rule",
            "src/** box",
        ];
        let config = loaded(&rules);
        let said = lines(&config, Some("a.md")).unwrap_or_default();
        assert!(said.contains("* @emoji\n*.md caveman"), "{said}");
        assert!(!said.contains("box"), "{said}");
    }

    #[test]
    fn an_unknown_form_is_refused() {
        assert_eq!(Shape::named("prompt"), Ok(Shape::Prompt));
        assert!(Shape::named("yaml").is_err_and(|why| why.contains("yaml")));
    }
}
