//! The tests of `notices.rs`, in a file of their own (sherd V50).

use std::collections::BTreeSet;

use super::render;
use crate::owners::Pkg;

fn pkg(name: &str, licence: &str) -> Pkg {
    Pkg {
        name: name.into(),
        version: "1.0.0".into(),
        licence: licence.into(),
    }
}

fn closure() -> BTreeSet<Pkg> {
    [
        pkg("b", "MIT"),
        pkg("a", "MIT OR Apache-2.0"),
        pkg("c", "MIT"),
    ]
    .into_iter()
    .collect()
}

#[test]
fn the_block_counts_direct_and_transitive() {
    let block = render(&closure(), 1).unwrap_or_default();
    assert!(
        block.starts_with(
            "## The closure: 3 packages\n\n1 direct, 2 transitive,"
        ),
        "{block}"
    );
}

#[test]
fn the_table_lists_packages_by_name() {
    let block = render(&closure(), 1).unwrap_or_default();
    let a = block.find("| `a` | 1.0.0 | MIT OR Apache-2.0 |");
    let b = block.find("| `b` | 1.0.0 | MIT |");
    assert!(a.is_some() && a < b, "{block}");
}

#[test]
fn the_tally_puts_the_commonest_licence_first() {
    let block = render(&closure(), 1).unwrap_or_default();
    assert!(
        block.ends_with("- `MIT` (2)\n- `MIT OR Apache-2.0` (1)\n"),
        "{block}"
    );
}

#[test]
fn more_direct_dependencies_than_packages_is_an_error() {
    assert!(render(&closure(), 4).is_err());
}
