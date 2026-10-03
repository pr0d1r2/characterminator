//! The tests of `owners.rs`, in a file of their own (sherd V50).

use super::{Pkg, closure, coverage, dependency_count, unsafe_level, value};

const MANIFEST: &str = r#"[workspace]
members = ["dev"]

[package]
name = "demo"
# a comment = "not a key"
edition = "2024"
include = [
    "/src/**/*",
]

[[bin]]
name = "d"

[dependencies]
itok = { version = "0.3", default-features = false,
  features = ["bpe"] }
plain = "1"

[dependencies.big]
version = "2"

[workspace.lints.rust]
unsafe_code = "forbid"
"#;

#[test]
fn value_reads_the_named_table_only() {
    assert_eq!(value(MANIFEST, "package", "name"), Some("demo".to_owned()));
    assert_eq!(value(MANIFEST, "bin", "name"), Some("d".to_owned()));
}

#[test]
fn value_is_none_when_the_owner_lacks_it() {
    assert_eq!(value(MANIFEST, "package", "rust-version"), None);
}

#[test]
fn a_commented_key_is_not_a_key() {
    assert_eq!(value(MANIFEST, "package", "# a comment"), None);
}

#[test]
fn a_wrapped_dependency_counts_once() {
    assert_eq!(dependency_count(MANIFEST), 3);
}

#[test]
fn unsafe_level_reads_the_workspace_table() {
    assert_eq!(unsafe_level(MANIFEST), Some("forbid".to_owned()));
}

#[test]
fn unsafe_level_falls_back_to_the_package_table() {
    let toml = "[lints.rust]\nunsafe_code = \"deny\"\n";
    assert_eq!(unsafe_level(toml), Some("deny".to_owned()));
}

#[test]
fn coverage_truncates_rather_than_rounds() {
    assert_eq!(coverage("# c\nlines 97.59\n"), Some("97.5".to_owned()));
}

#[test]
fn coverage_pads_a_whole_number() {
    assert_eq!(coverage("lines 100\n"), Some("100.0".to_owned()));
}

#[test]
fn coverage_refuses_what_is_not_a_number() {
    assert_eq!(coverage("lines lots\n"), None);
    assert_eq!(coverage("key abc\n"), None);
}

#[test]
fn closure_dedups_and_strips_the_repeat_marker() {
    let tree = "demo v0.1.0 (/x)|MIT\nitok v0.3.1|MIT\nitok v0.3.1|MIT (*)\n";
    let want = [
        Pkg {
            name: "demo".into(),
            version: "0.1.0".into(),
            licence: "MIT".into(),
        },
        Pkg {
            name: "itok".into(),
            version: "0.3.1".into(),
            licence: "MIT".into(),
        },
    ];
    assert_eq!(closure(tree), want.into_iter().collect());
}

#[test]
fn closure_skips_lines_it_cannot_read() {
    assert!(closure("warning: something\n\n").is_empty());
}
