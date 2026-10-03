//! The tests of `dispatch.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `dispatch`.

use super::{
    Format, Outcome, adapted, args, format_of, prepared, sarif_misused,
    sarif_refused, verb_of,
};
use crate::cli::testkit::argv;

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

/// `src/cli/guard:V53`: a failed adapter exits 1, which Claude Code
/// reads as a non-blocking error, and never 2, which it reads as a
/// block.
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

/// V73: a path a verb would ignore is refused, not dropped.
#[test]
fn a_path_the_verb_would_ignore_is_refused() {
    let refused = |words: &[&str]| {
        let verb = words.first().copied().unwrap_or_default();
        prepared(verb, &argv(words)).err().unwrap_or_default()
    };
    assert!(refused(&["sets", "a.md"]).contains("`a.md`"));
    assert!(refused(&["explain", "a.md", "b.txt"]).contains("`b.txt`"));
    let as_args = ["explain", "--as", "args", "a.md", "b.txt"];
    assert!(refused(&as_args).contains("`b.txt`"));
    assert!(prepared("explain", &argv(&["explain", "a.md"])).is_ok());
    assert!(prepared("check", &argv(&["check", "a", "b"])).is_ok());
}

/// B29: a configuration that does not parse stops EVERY verb, not
/// only the ones that read the broken kind; and `--map` takes one
/// line exactly as `--set` and `--rule` do (`src/rules:V18`).
#[test]
fn a_broken_config_of_any_kind_stops_every_verb() {
    let broken = [
        ["--map", "U+ZZZZ x y z"],
        ["--map", "U+2014 -\nU+2013 -"],
        ["--set", "house U+2261\nmore U+2262"],
        ["--rule", "* ascii\n* box"],
    ];
    for verb in ["check", "explain", "sets", "fix", "stats"] {
        for [flag, value] in broken {
            let asked = argv(&[verb, flag, value]);
            assert!(prepared(verb, &asked).is_err(), "{verb} {value}");
        }
    }
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
    let config = crate::cli::config::discovered(&root);
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
