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
        assert!(text.contains("ctrm check"), "{words:?}: {text}");
    }
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
