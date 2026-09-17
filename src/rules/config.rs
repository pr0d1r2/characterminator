//! Where configuration COMES FROM, in the order V19 fixes.
//!
//! Low to high: the builtin data, then discovered dotfiles, then the
//! files named by `--*-file` in argv order, then inline flags in argv
//! order. Assembling them produces one list of entries in exactly that
//! order, and "later wins" is then nothing more than reading the list
//! forward: the rules node does it per path (V2), and the map and sets
//! nodes do it per character and per set name.
//!
//! This holds no file system walk. WHICH dotfiles are discovered, and
//! from where, is an open question in the node spec, so a discovered
//! file arrives here as a path and its text, exactly like any other.

use crate::rules::line::{ParseError, parse_builtin, parse_file, parse_flag};
use crate::rules::rule_line::parse_rule;
use crate::rules::{Origin, Rule};
use std::path::PathBuf;

/// The contributing sources of one data-file kind.
///
/// Four groups rather than one list of "sources with a precedence
/// number": the chain is not a property a caller sets, it is the shape
/// of this type, so a source cannot be added at the wrong precedence and
/// the ordering cannot drift from the invariant that states it.
///
/// Within the last two groups, ORDER OF ADDITION is argv order. The
/// caller walks the arguments once, in order, which is what the CLI does
/// anyway.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources {
    builtin: Option<String>,
    dotfiles: Vec<(PathBuf, String)>,
    files: Vec<(PathBuf, String)>,
    flags: Vec<(usize, String)>,
}

impl Sources {
    /// No source at all.
    ///
    /// This is the zero-file starting point (V21): `--no-files` and the
    /// `--no-builtin-*` flags are not a mode this type has to know
    /// about, they are sources the caller does not add.
    pub fn new() -> Self {
        Self::default()
    }

    /// The compiled-in data file, lowest of all.
    pub fn builtin(mut self, text: impl Into<String>) -> Self {
        self.builtin = Some(text.into());
        self
    }

    /// A dotfile found by discovery.
    pub fn dotfile(
        mut self,
        path: impl Into<PathBuf>,
        text: impl Into<String>,
    ) -> Self {
        self.dotfiles.push((path.into(), text.into()));
        self
    }

    /// A file named by a `--*-file` flag.
    pub fn file(
        mut self,
        path: impl Into<PathBuf>,
        text: impl Into<String>,
    ) -> Self {
        self.files.push((path.into(), text.into()));
        self
    }

    /// One inline flag, at its position in argv.
    pub fn flag(mut self, index: usize, value: impl Into<String>) -> Self {
        self.flags.push((index, value.into()));
        self
    }

    /// The `--fidelity <family>` flag.
    ///
    /// V29 defines it as `--rule '* @<family>'` at that argv position,
    /// so here it IS that rule. Synthesising the line rather than
    /// carrying a separate field keeps one precedence chain, one parser
    /// and one kind of origin: `explain` names the argument that set the
    /// family the same way it names any other rule.
    pub fn fidelity(self, index: usize, family: &str) -> Self {
        self.flag(index, format!("* @{family}"))
    }

    /// A run configured by argv and nothing else (V21).
    ///
    /// `--no-files --no-builtin-map --no-builtin-sets` is not a mode
    /// anything here switches on: it IS this constructor, the one that
    /// adds no builtin and no file. A mode flag would mean every reader
    /// of every source has to remember to consult it; a source that was
    /// never added cannot be forgotten.
    ///
    /// V1 survives the emptiness because the fallback set is a constant
    /// in code: `ascii` is not a line of a file that is not there.
    pub fn from_argv<I>(flags: I) -> Self
    where
        I: IntoIterator<Item = (usize, String)>,
    {
        let mut sources = Self::new();
        sources.flags = flags.into_iter().collect();
        sources
    }

    /// Every entry of every source, in precedence order.
    ///
    /// The kind's line parser is a parameter, so one chain serves rules,
    /// map and sets alike (V18) and this node still calls into no
    /// sibling.
    pub fn assemble<T, P>(&self, parse: P) -> Result<Vec<T>, ParseError>
    where
        P: Fn(&str, Origin) -> Result<T, ParseError> + Copy,
    {
        let mut entries = Vec::new();
        if let Some(text) = &self.builtin {
            entries.extend(parse_builtin(text, parse)?);
        }
        for (path, text) in self.dotfiles.iter().chain(&self.files) {
            entries.extend(parse_file(text, path, parse)?);
        }
        for (index, value) in &self.flags {
            entries.extend(parse_flag(value, *index, parse)?);
        }
        Ok(entries)
    }

    /// The rules kind, the one whose parser lives in this node.
    pub fn rules(&self) -> Result<Vec<Rule>, ParseError> {
        self.assemble(parse_rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::line::describe;
    use crate::rules::{ASCII, resolve};

    /// The crude stand-in for the undecided matcher, again: a trailing
    /// star matches a prefix, anything else is an exact path.
    fn matches(pattern: &str, path: &str) -> bool {
        match pattern.strip_suffix('*') {
            Some(prefix) => path.starts_with(prefix),
            None => pattern == path,
        }
    }

    /// The set in force for a path, which is what precedence decides.
    /// `ascii` leads every answer because it is the base of every rule
    /// (V24), so what these tests read is the grant the winner added.
    fn winning_set(sources: &Sources, path: &str) -> String {
        let rules = sources.rules().unwrap_or_default();
        let resolution = resolve(path, &rules, &matches);
        resolution.sets.join("+")
    }

    /// The fidelity family in force for a path.
    fn family_of(sources: &Sources, path: &str) -> String {
        let rules = sources.rules().unwrap_or_default();
        resolve(path, &rules, &matches).family
    }

    fn origins(sources: &Sources) -> Vec<String> {
        let rules = sources.rules().unwrap_or_default();
        rules.iter().map(|rule| describe(&rule.origin)).collect()
    }

    /// One source of each group, each granting a different set.
    fn every_group() -> Sources {
        Sources::new()
            .builtin("* box")
            .dotfile(".ctrm", "* caveman")
            .file("other.ctrm", "* math")
            .flag(3, "* legal")
    }

    #[test]
    fn a_flag_beats_a_named_file() {
        let sources = every_group();
        assert_eq!(winning_set(&sources, "SPEC.md"), "ascii+legal");
    }

    #[test]
    fn a_named_file_beats_a_discovered_dotfile() {
        let sources = Sources::new()
            .dotfile(".ctrm", "* caveman")
            .file("other.ctrm", "* math");
        assert_eq!(winning_set(&sources, "SPEC.md"), "ascii+math");
    }

    #[test]
    fn a_dotfile_beats_the_builtin() {
        let sources = Sources::new()
            .builtin("* box")
            .dotfile(".ctrm", "* caveman");
        assert_eq!(winning_set(&sources, "SPEC.md"), "ascii+caveman");
    }

    #[test]
    fn the_chain_holds_whatever_order_the_groups_were_filled_in() {
        let backwards = Sources::new()
            .flag(3, "* legal")
            .file("other.ctrm", "* math")
            .dotfile(".ctrm", "* caveman")
            .builtin("* box");
        assert_eq!(backwards.assemble(parse_rule), every_group().rules());
    }

    #[test]
    fn two_files_keep_the_order_they_were_named_in() {
        let sources = Sources::new()
            .file("first.ctrm", "* caveman")
            .file("second.ctrm", "* math");
        assert_eq!(winning_set(&sources, "SPEC.md"), "ascii+math");
    }

    #[test]
    fn two_flags_keep_their_argv_order() {
        let sources = Sources::new().flag(2, "* caveman").flag(5, "* math");
        assert_eq!(winning_set(&sources, "SPEC.md"), "ascii+math");
    }

    #[test]
    fn every_entry_carries_an_origin_that_names_its_source() {
        let expected = vec![
            "builtin:1".to_string(),
            ".ctrm:1".to_string(),
            "other.ctrm:1".to_string(),
            "argv[3]".to_string(),
        ];
        assert_eq!(origins(&every_group()), expected);
    }

    #[test]
    fn an_origin_keeps_the_line_number_inside_its_own_file() {
        let sources =
            Sources::new().dotfile(".ctrm", "# note\n* caveman\nSPEC.md box");
        assert_eq!(origins(&sources), vec![".ctrm:2", ".ctrm:3"]);
    }

    #[test]
    fn a_bad_line_names_the_source_it_came_from() {
        let sources = Sources::new().file("other.ctrm", "*.md !loud");
        let failure = sources.rules().err().map(|fail| fail.to_string());
        let message = "other.ctrm:1: unknown level `loud`".to_string();
        assert_eq!(failure, Some(message));
    }

    #[test]
    fn a_source_contributing_nothing_changes_nothing() {
        let sources = every_group().file("empty.ctrm", "# only a comment\n");
        assert_eq!(sources.rules(), every_group().rules());
    }

    /// A run with `--no-files` and both `--no-builtin-*` flags.
    fn argv_only() -> Sources {
        let flags =
            [(1, "* caveman".to_string()), (4, "src/* ascii".to_string())];
        Sources::from_argv(flags)
    }

    #[test]
    fn the_fidelity_flag_is_exactly_the_rule_flag_it_stands_for() {
        let spelled = Sources::new().flag(2, "* @emoji");
        assert_eq!(Sources::new().fidelity(2, "emoji"), spelled);
    }

    #[test]
    fn the_fidelity_flag_beats_a_family_named_in_a_file() {
        let sources = Sources::new()
            .dotfile(".ctrm", "docs/* marks @emoji")
            .fidelity(4, "text");
        assert_eq!(family_of(&sources, "docs/a.md"), "text");
    }

    #[test]
    fn a_file_read_after_the_flag_would_not_exist_to_beat_it() {
        let sources = Sources::new()
            .file("other.ctrm", "docs/* @emoji")
            .fidelity(4, "text");
        assert_eq!(family_of(&sources, "docs/a.md"), "text");
    }

    #[test]
    fn a_run_with_no_source_at_all_holds_no_rule() {
        assert_eq!(Sources::new().rules(), Ok(Vec::new()));
    }

    #[test]
    fn a_path_in_a_zero_file_run_still_gets_ascii() {
        assert_eq!(winning_set(&Sources::new(), "src/main.rs"), ASCII);
    }

    #[test]
    fn argv_alone_carries_a_whole_configuration() {
        assert_eq!(winning_set(&argv_only(), "src/main.rs"), "ascii");
        assert_eq!(winning_set(&argv_only(), "SPEC.md"), "ascii+caveman");
    }

    #[test]
    fn a_zero_file_run_records_argv_origins_only() {
        let expected = vec!["argv[1]".to_string(), "argv[4]".to_string()];
        assert_eq!(origins(&argv_only()), expected);
    }

    #[test]
    fn a_path_no_argv_rule_matches_gets_ascii_too() {
        let flags = [(1, "docs/** caveman".to_string())];
        let sources = Sources::from_argv(flags);
        assert_eq!(winning_set(&sources, "src/main.rs"), ASCII);
    }
}
