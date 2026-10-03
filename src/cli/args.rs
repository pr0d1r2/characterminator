//! argv, read ONCE: the flags in argv order, and the paths.
//!
//! Every verb used to sniff argv for the words it cared about, which had
//! two failure modes this replaces. A value flag's argument could be read
//! as a path (`--rule '*.md caveman'` would have "named" a file called
//! `*.md caveman`), and a flag nobody knew -- `--stirct` -- was dropped
//! without a word, which reads exactly like a flag that worked.
//!
//! So the table below is the whole flag surface, and a word that starts
//! with a dash and is not in it is a USAGE error (exit 2). A value is
//! taken POSITIONALLY, as the word after its flag, whatever it looks
//! like: `--map '-- x'` is a map line, not a flag.

/// The flags that take a value, which is always the next word.
const VALUED: &[&str] = &[
    "--format",
    "--fidelity",
    "--rule",
    "--map",
    "--set",
    "--rules-file",
    "--map-file",
    "--sets-file",
    "-C",
    "--as",
    "--max",
    "--containing",
];

/// The flags that stand alone.
const SWITCHES: &[&str] = &[
    "--check",
    "--bpe",
    "--summary",
    "--no-files",
    "--no-builtin-map",
    "--no-builtin-sets",
    "--strict",
    "--pedantic",
    "--locales",
    "--print",
    // Accepted on every verb and changes nothing: ctrm never prints colour,
    // so there is none to turn off. It is in the table because unknown
    // flags are REFUSED, and a wrapper that passes `--no-color` to every
    // tool it runs should not get exit 2 for asking politely. `NO_COLOR`
    // needs no code for the same reason.
    "--no-color",
    // A request for the usage, answered by dispatch before the verb runs
    // (`src/cli:V101`). In the table so `check --help` is not an unknown
    // flag; every other unknown word stays refused (V74).
    "--help",
    "-h",
    // The version, answered by dispatch like `--help` (`src/cli/usage:V119`):
    // in the table so `ctrm --version --bogus` is refused, not answered.
    "--version",
    "-V",
];

/// The flags only one verb means anything to. Accepted elsewhere they
/// would be silently ignored, which is the failure this file exists to
/// refuse: `ctrm check --bpe` looks like it asked for something.
const OWNED: &[(&str, &str)] = &[
    ("--check", "fix"),
    ("--bpe", "stats"),
    ("--as", "explain"),
    ("--summary", "check"),
    ("--max", "check"),
    ("--locales", "sets"),
    ("--containing", "sets"),
    ("--print", "init"),
];

/// Ends the flags: every word after it is a path, so a file whose name
/// starts with a dash can still be named.
const END: &str = "--";

/// One flag as given: its name, its value if it takes one, and the argv
/// position that IS its origin (`src/rules:V20`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Flag {
    pub name: &'static str,
    pub value: Option<String>,
    /// The position in the PROCESS argv, program name at 0, of the word
    /// that carries the configuration: the value for a valued flag, the
    /// flag itself for a switch. `argv[3]` then names the word a reader
    /// would count to in their shell.
    pub index: usize,
}

/// A whole command line: the verb, the flags and the paths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Args {
    /// The first word that is neither a flag nor a flag's value
    /// (`src/cli/usage:V119`), so a flag may come before it.
    pub verb: Option<String>,
    pub flags: Vec<Flag>,
    pub paths: Vec<String>,
}

impl Args {
    /// The value of the LAST occurrence of a valued flag. Later wins, as it
    /// does for every other repeated setting in this tool (`src/rules:V19`).
    #[must_use]
    pub(super) fn value(&self, name: &str) -> Option<&str> {
        let named = self.flags.iter().rev().find(|flag| flag.name == name);
        named.and_then(|flag| flag.value.as_deref())
    }

    /// Whether a flag was given at all.
    #[must_use]
    pub(super) fn has(&self, name: &str) -> bool {
        self.flags.iter().any(|flag| flag.name == name)
    }
}

/// Read `args` -- argv WITHOUT the program name -- into the verb, the
/// flags and the paths. The verb is the first word that is not a flag or
/// a flag's value, so `-C .. check` reads as `check -C ..`; every flag
/// keeps its own process-argv position either way.
///
/// # Errors
///
/// A word that looks like a flag and is not one, a valued flag with no
/// word after it, or a flag that belongs to another verb.
pub(super) fn parse(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut words = args.iter().enumerate();
    while let Some((at, word)) = words.next() {
        if word == END {
            parsed.paths.extend(words.map(|(_, rest)| rest.clone()));
            break;
        }
        match flag_of(word)? {
            None if parsed.verb.is_none() => parsed.verb = Some(word.clone()),
            None => parsed.paths.push(word.clone()),
            Some(name) => parsed.flags.push(take(name, at, &mut words)?),
        }
    }
    owned(&parsed)?;
    Ok(parsed)
}

/// The flag a word names, `None` for a positional word, or why it is
/// refused.
///
/// A lone dash is refused rather than read as standard input: no verb
/// reads standard input, and a path called `-` is spelled `-- -`.
fn flag_of(word: &str) -> Result<Option<&'static str>, String> {
    if !word.starts_with('-') {
        return Ok(None);
    }
    let known = VALUED.iter().chain(SWITCHES).find(|name| **name == word);
    let name = known.ok_or_else(|| format!("unknown flag `{word}`"))?;
    Ok(Some(*name))
}

/// A flag only another verb means anything to is refused: accepted, it
/// would be silently ignored. Checked once the verb is known, which may
/// be after the flag.
fn owned(read: &Args) -> Result<(), String> {
    let verb = read.verb.as_deref().unwrap_or_default();
    if !super::usage::VERBS.contains(&verb) {
        // No verb, or an unknown one: dispatch names THAT, which is the
        // mistake, rather than a flag's owner.
        return Ok(());
    }
    for flag in &read.flags {
        let owner = OWNED.iter().find(|(name, _)| *name == flag.name);
        if let Some((name, owner)) = owner.filter(|(_, o)| *o != verb) {
            return Err(format!("`{name}` belongs to `{owner}`, not `{verb}`"));
        }
    }
    Ok(())
}

/// Build one flag, reading its value off `words` when it takes one.
fn take<'a, I>(
    name: &'static str,
    at: usize,
    words: &mut I,
) -> Result<Flag, String>
where
    I: Iterator<Item = (usize, &'a String)>,
{
    let (at, value) = if VALUED.contains(&name) {
        let next = words.next();
        let (at, value) =
            next.ok_or_else(|| format!("`{name}` needs a value"))?;
        (at, Some(value.clone()))
    } else {
        (at, None)
    };
    let index = at.saturating_add(1);
    Ok(Flag { name, value, index })
}

#[cfg(test)]
mod tests {
    use super::{Args, parse};
    use crate::cli::testkit::argv;

    fn parsed(words: &[&str]) -> Args {
        let read = parse(&argv(words));
        assert!(read.is_ok(), "{read:?}");
        read.unwrap_or_default()
    }

    fn refused(words: &[&str]) -> String {
        parse(&argv(words)).err().unwrap_or_default()
    }

    /// The case `paths_of` existed to get right, now for every valued
    /// flag: a rule line full of globs and blanks is a VALUE.
    #[test]
    fn a_value_is_never_a_path_whatever_it_looks_like() {
        let read = parsed(&["check", "--rule", "*.md caveman", "a.md"]);
        assert_eq!(read.paths, vec!["a.md"]);
        assert_eq!(read.value("--rule"), Some("*.md caveman"));
        let read = parsed(&["fix", "--map", "-- x", "--check"]);
        assert_eq!(read.value("--map"), Some("-- x"));
        assert!(read.has("--check") && read.paths.is_empty());
    }

    /// The origin is the PROCESS argv position: `ctrm` is 0 and the verb
    /// 1, so the value of the first flag after the verb is `argv[3]`.
    #[test]
    fn a_flag_carries_the_argv_position_of_its_value() {
        let read = parsed(&["check", "--rule", "* box", "--strict"]);
        let at: Vec<usize> = read.flags.iter().map(|f| f.index).collect();
        assert_eq!(at, vec![3, 4]);
    }

    /// A typo is not a flag that silently did nothing.
    #[test]
    fn an_unknown_flag_is_refused() {
        assert!(refused(&["check", "--stirct"]).contains("--stirct"));
        assert!(refused(&["check", "--format=json"]).contains("unknown"));
        assert!(refused(&["check", "-"]).contains("unknown"));
    }

    #[test]
    fn a_valued_flag_with_nothing_after_it_is_refused() {
        assert!(refused(&["check", "--rule"]).contains("needs a value"));
    }

    #[test]
    fn a_verb_specific_flag_is_refused_on_another_verb() {
        assert!(refused(&["check", "--bpe"]).contains("`stats`"));
        assert!(refused(&["stats", "--check"]).contains("`fix`"));
        assert!(refused(&["check", "--as", "args"]).contains("`explain`"));
        assert!(parsed(&["stats", "--bpe"]).has("--bpe"));
    }

    /// B70 / `src/cli/usage:V119`: a flag before the verb is a flag, the
    /// verb is the first word that is neither, and each flag keeps the
    /// process-argv position a reader would count to.
    #[test]
    fn a_flag_may_come_before_the_verb() {
        let read = parsed(&["-C", "..", "check", "--rule", "* box", "a.md"]);
        assert_eq!(read.verb.as_deref(), Some("check"));
        assert_eq!(read.value("-C"), Some(".."));
        assert_eq!(read.paths, vec!["a.md"]);
        let at: Vec<usize> = read.flags.iter().map(|f| f.index).collect();
        assert_eq!(at, vec![2, 5]);
        assert!(refused(&["--bpe", "check"]).contains("`stats`"));
        assert_eq!(parsed(&["--strict"]).verb, None);
    }

    #[test]
    fn a_double_dash_ends_the_flags() {
        let read = parsed(&["check", "--", "-odd", "--strict"]);
        assert_eq!(read.paths, vec!["-odd", "--strict"]);
        assert!(read.flags.is_empty());
    }

    /// The usage text names every flag the table accepts, so a flag
    /// cannot ship that `ctrm` with no arguments does not mention.
    #[test]
    fn the_usage_names_every_flag() {
        for flag in super::VALUED.iter().chain(super::SWITCHES) {
            assert!(crate::cli::usage::USAGE.contains(flag), "{flag}");
        }
    }

    /// A no-op, but an ACCEPTED one on every verb: refusing it would fail
    /// wrappers that pass it to every tool they run.
    #[test]
    fn no_color_is_accepted_by_every_verb() {
        for verb in ["check", "fix", "stats", "explain", "sets"] {
            assert!(parsed(&[verb, "--no-color"]).has("--no-color"), "{verb}");
        }
    }

    #[test]
    fn the_last_value_of_a_repeated_flag_wins() {
        let read = parsed(&["check", "--format", "json", "--format", "sarif"]);
        assert_eq!(read.value("--format"), Some("sarif"));
        assert_eq!(read.value("--fidelity"), None);
    }
}
