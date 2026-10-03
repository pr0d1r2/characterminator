//! The tests of `config.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `config`.

use super::{Config, MAP, PEDANTIC, RULES, SETS, from_argv};
use crate::cli::testkit::{argv, fixture};
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
    let fixed = fix::run(&config, &asked, Format::Human, true).map(|r| r.text);
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
    let Some(root) = fixture("ctrm-dash-c-fixture/sub", &[(RULES, "* box\n")])
    else {
        return;
    };
    let parent = root.parent().map(Path::to_path_buf).unwrap_or_default();
    let config = from_argv(&parent, &argv(&["check", "-C", "sub"]));
    let config: Config = config.unwrap_or_default();
    assert_eq!(config.root, root);
    assert_eq!(config.rules.lines(), vec!["* box"]);
}

/// `src/rules:V45`: a `--*-file` path resolves against the run root,
/// not the working directory, and a second `-C` replaces the first.
#[test]
fn a_named_file_resolves_against_the_last_directory_flag() {
    let files = [("own.ctrm", "* caveman\n")];
    let Some(root) = fixture("ctrm-dash-c-twice-fixture/b", &files) else {
        return;
    };
    let parent = root.parent().map(Path::to_path_buf).unwrap_or_default();
    let words = ["check", "-C", "a", "-C", "b", "--rules-file", "own.ctrm"];
    let config = from_argv(&parent, &argv(&words)).unwrap_or_default();
    assert_eq!(config.root, root);
    assert_eq!(config.rules.lines(), vec!["* caveman"]);
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

/// B47: a rule naming an undeclared set is refused at validation, by
/// its origin (V74) -- even when no file matches the rule, which is
/// when it used to pass.
#[test]
fn a_rule_naming_an_undeclared_set_is_refused_at_its_origin() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let words = ["check", "--no-files", "--rule", "*.nomatch asci"];
    let config = from_argv(root, &argv(&words));
    let why = config.and_then(|c| c.validate()).err().unwrap_or_default();
    assert!(why.contains("argv[4]") && why.contains("asci"), "{why}");
}
