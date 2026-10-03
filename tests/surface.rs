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
