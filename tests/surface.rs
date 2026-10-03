//! The parts of the command surface (`src/cli` INTERFACES) that only
//! exist at the process boundary: the version flags and the `NO_COLOR`
//! environment variable.

use std::process::{Command, Output};

fn ctrm(args: &[&str], no_color: bool) -> Option<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ctrm"));
    command.args(args).current_dir(env!("CARGO_MANIFEST_DIR"));
    if no_color {
        command.env("NO_COLOR", "1");
    } else {
        command.env_remove("NO_COLOR");
    }
    command.output().ok()
}

/// `--version` and `-V` print the crate name and version, exit 0.
#[test]
fn both_version_flags_name_the_crate_and_its_version() {
    let want =
        format!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    for flag in ["--version", "-V"] {
        let got = ctrm(&[flag], false);
        let code = got.as_ref().and_then(|o| o.status.code());
        let text = got.map(|o| o.stdout).unwrap_or_default();
        assert_eq!(code, Some(0), "{flag}");
        assert_eq!(String::from_utf8_lossy(&text), want, "{flag}");
    }
}

/// `NO_COLOR` is honoured because nothing is ever coloured: the output
/// is byte for byte the same with it set, and it is never refused.
#[test]
fn no_color_changes_nothing() {
    let plain = ctrm(&["sets"], false);
    let asked = ctrm(&["sets"], true);
    let code = asked.as_ref().and_then(|o| o.status.code());
    assert_eq!(code, Some(0));
    let bytes = |o: Option<Output>| o.map(|o| o.stdout).unwrap_or_default();
    let (plain, asked) = (bytes(plain), bytes(asked));
    assert!(!plain.is_empty());
    assert_eq!(plain, asked);
}

/// `src/cli:V101`: asked for, the usage is the answer -- stdout, exit 0,
/// bare and after a verb. A broken `.ctrm` cannot stand in the way: help
/// is answered before the configuration loads.
#[test]
fn help_is_printed_to_stdout_and_exits_zero() {
    let asks: [&[&str]; 5] = [
        &["--help"],
        &["-h"],
        &["check", "--help"],
        &["explain", "-h"],
        &["fix", "--rule", "* asci", "--help"],
    ];
    for words in asks {
        let got = ctrm(words, false);
        let code = got.as_ref().and_then(|o| o.status.code());
        let text = got.map(|o| o.stdout).unwrap_or_default();
        assert_eq!(code, Some(0), "{words:?}");
        let text = String::from_utf8_lossy(&text);
        assert!(is_page_for(words, &text), "{words:?}: {text}");
    }
}

/// The global page lists the verbs; after a verb, it is that verb's page
/// (`src/cli/usage:V120`). Either way it gives the exit codes.
fn is_page_for(words: &[&str], text: &str) -> bool {
    let verb = words.first().filter(|w| !w.starts_with('-'));
    let page =
        verb.map_or_else(|| "verbs:".to_owned(), |v| format!("ctrm {v}"));
    text.contains(&page) && text.contains("exit codes")
}

/// No arguments is still a usage ERROR: stderr, exit 2. And `--help`
/// after `--` is a path, not a request for help.
#[test]
fn no_arguments_and_a_help_that_is_not_a_flag_stay_errors() {
    let asks: [&[&str]; 2] = [&[], &["sets", "--", "--help"]];
    for words in asks {
        let got = ctrm(words, false);
        let code = got.as_ref().and_then(|o| o.status.code());
        let text = got.map(|o| o.stdout).unwrap_or_default();
        assert_eq!(code, Some(2), "{words:?}");
        assert!(text.is_empty(), "{words:?}");
    }
}

/// `src/render:V124`: `--summary` and `--max` are refused where they
/// would be ignored, and `--max` takes a number.
#[test]
fn a_shape_flag_that_would_be_ignored_is_refused() {
    let asks: [&[&str]; 3] = [
        &["check", "--format", "sarif", "--summary"],
        &["check", "--format", "json", "--max", "3"],
        &["check", "--max", "lots"],
    ];
    for words in asks {
        let code = ctrm(words, false).and_then(|o| o.status.code());
        assert_eq!(code, Some(2), "{words:?}");
    }
}

/// `src/render:V122`: under `--format json` a refused run is ALSO a
/// document on stdout, same exit 2; without it stdout stays empty.
#[test]
fn a_json_run_that_fails_says_so_in_json() {
    let asks: [(&[&str], &str); 2] = [
        (&["check", "--format", "json", "--stirct"], "check"),
        (&["stats", "--rule", "* asci", "--format", "json"], "stats"),
    ];
    for (words, verb) in asks {
        let got = ctrm(words, false);
        let code = got.as_ref().and_then(|o| o.status.code());
        let text = got.map(|o| o.stdout).unwrap_or_default();
        let text = String::from_utf8_lossy(&text);
        let head = format!("{{\"schema\":1,\"verb\":\"{verb}\",\"error\":\"");
        assert_eq!(code, Some(2), "{words:?}");
        assert!(text.starts_with(&head), "{words:?}: {text}");
    }
    let plain = ctrm(&["check", "--stirct"], false);
    assert!(plain.map(|o| o.stdout).unwrap_or_default().is_empty());
}

/// Exit code and stderr of one run.
fn refused(words: &[&str]) -> (Option<i32>, String) {
    let got = ctrm(words, false);
    let code = got.as_ref().and_then(|o| o.status.code());
    let err = got.map(|o| o.stderr).unwrap_or_default();
    (code, String::from_utf8_lossy(&err).into_owned())
}

/// B70 / `src/cli/usage:V119`: a mistyped verb is NAMED on the first
/// line, with a pointer rather than the whole usage; `--help` beside an
/// unknown flag is refused bare, as it is after a verb.
#[test]
fn a_wrong_command_line_says_what_was_wrong() {
    let (code, err) = refused(&["chek"]);
    assert_eq!(code, Some(2));
    assert!(
        err.starts_with("ctrm: unknown verb 'chek' (did you mean 'check'?)\n")
    );
    assert_eq!(err.lines().count(), 2, "{err}");
    for words in [&["--help", "--bogus"][..], &["check", "--help", "--bogus"]] {
        let (code, err) = refused(words);
        assert_eq!(code, Some(2), "{words:?}");
        assert!(err.contains("--bogus"), "{words:?}: {err}");
    }
}

/// A flag before the verb is a flag (`ctrm -C .. check`), and `guard
/// --help` alone is help rather than a read of stdin.
#[test]
fn flags_may_precede_the_verb_and_guard_has_help() {
    let got = ctrm(&["--no-color", "sets"], false);
    assert_eq!(got.and_then(|o| o.status.code()), Some(0));
    let got = ctrm(&["guard", "--help"], false);
    let code = got.as_ref().and_then(|o| o.status.code());
    let text = got.map(|o| o.stdout).unwrap_or_default();
    assert_eq!(code, Some(0));
    assert!(String::from_utf8_lossy(&text).contains("ctrm guard"));
}

/// B77 / `src/render:V122`: with flags before the verb, the error document
/// names the verb, not the value of `--format`.
#[test]
fn a_json_error_names_the_verb_after_leading_flags() {
    let got = ctrm(&["--format", "json", "check", "--stirct"], false);
    let text = got.map(|o| o.stdout).unwrap_or_default();
    let text = String::from_utf8_lossy(&text);
    assert!(
        text.starts_with("{\"schema\":1,\"verb\":\"check\""),
        "{text}"
    );
}
