//! The tests of `select.rs`, in a file of their own (sherd V50).

use super::{OUTPUTS, matches, selected};

fn names(paths: &[&str]) -> Vec<&'static str> {
    let paths: Vec<String> = paths.iter().map(|p| (*p).to_owned()).collect();
    selected(&paths).iter().map(|o| o.name).collect()
}

#[test]
fn no_paths_selects_every_output() {
    assert_eq!(names(&[]), vec!["readme", "notices"]);
}

#[test]
fn a_shared_input_selects_both() {
    assert_eq!(names(&["Cargo.lock"]), vec!["readme", "notices"]);
}

#[test]
fn an_input_of_one_selects_only_that_one() {
    assert_eq!(names(&[".coverage"]), vec!["readme"]);
    assert_eq!(names(&["./docs/THIRD-PARTY-NOTICES.md"]), vec!["notices"]);
}

#[test]
fn an_unrelated_change_selects_nothing() {
    assert!(names(&["src/main.rs", "dev/SPEC.md"]).is_empty());
}

#[test]
fn a_directory_pattern_needs_a_path_beneath_it() {
    assert!(matches("dev/src/**", "dev/src/lib.rs"));
    assert!(!matches("dev/src/**", "dev/srcx/lib.rs"));
}

/// THE GLOB IS TIED TO THE INPUTS (`dev:V136`). hk only runs the scoped
/// step when a changed file matches its glob, so an input missing from it
/// is a change the pre-commit layer never sees.
fn dev_generated_glob() -> String {
    let hk = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../hk.pkl"
    ));
    assert!(hk.is_ok(), "hk.pkl is not readable from the dev crate");
    let hk = hk.unwrap_or_default();
    let step = hk.split("[\"dev-generated\"]").nth(1).unwrap_or_default();
    let glob = step.lines().find(|l| l.trim_start().starts_with("glob"));
    glob.unwrap_or_default().to_owned()
}

#[test]
fn every_declared_input_is_in_the_hk_glob() {
    let glob = dev_generated_glob();
    for input in OUTPUTS.iter().flat_map(|o| o.inputs) {
        let quoted = format!("\"{input}\"");
        assert!(glob.contains(&quoted), "{input} not in the glob: {glob}");
    }
}
