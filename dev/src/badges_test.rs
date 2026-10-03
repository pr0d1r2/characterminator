//! The tests of `badges.rs`, in a file of their own (sherd V50).

use super::{Facts, render, shield};

fn facts() -> Facts {
    Facts {
        name: "demo".into(),
        slug: "me/demo".into(),
        licence: "MIT".into(),
        edition: "2024".into(),
        msrv: "1.95".into(),
        direct: 3,
        closure: 29,
        coverage: "97.5".into(),
        unsafe_level: "forbid".into(),
    }
}

#[test]
fn every_measured_number_is_in_the_block() {
    let block = render(&facts());
    for want in [
        "[![MSRV 1.95](https://img.shields.io/badge/MSRV-1.95-",
        "[![direct dependencies 3](https://img.shields.io/badge/direct_dependencies-3-",
        "[![runtime closure 29](https://img.shields.io/badge/runtime_closure-29-",
        "[![coverage 97.5%](https://img.shields.io/badge/coverage-97.5%25-brightgreen)](.coverage)",
        "[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-brightgreen)](Cargo.toml)",
        "https://github.com/me/demo/actions/workflows/ci.yml",
        "https://crates.io/crates/demo",
    ] {
        assert!(block.contains(want), "missing {want} in\n{block}");
    }
}

#[test]
fn the_block_ends_in_a_newline() {
    assert!(render(&facts()).ends_with('\n'));
}

#[test]
fn an_unsafe_level_short_of_forbid_does_not_read_as_a_pass() {
    let block = render(&Facts {
        unsafe_level: "deny".into(),
        ..facts()
    });
    assert!(block.contains(
        "[![unsafe deny](https://img.shields.io/badge/unsafe-deny-orange)]"
    ));
}

#[test]
fn shield_escapes_dashes_underscores_and_spaces() {
    assert_eq!(shield("MIT OR Apache-2.0"), "MIT_OR_Apache--2.0");
    assert_eq!(shield("a_b"), "a__b");
}
