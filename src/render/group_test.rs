//! The tests of `group.rs`: the fold behind `check --summary` and the
//! tally a human `check` ends with (V124).

use super::{groups, tally};
use crate::lint::{Finding, Level, OUTSIDE_SET};
use crate::render::line::Line;
use crate::scan::{Hit, Position};

fn finding(byte: usize, character: char) -> Finding {
    let position = Position {
        line: 1,
        column: byte.saturating_add(1),
        byte,
    };
    let hit = Hit {
        position,
        character,
    };
    let (lint, level) = (OUTSIDE_SET, Level::Deny);
    Finding { hit, lint, level }
}

fn line<'a>(path: &'a str, finding: &'a Finding) -> Line<'a> {
    Line {
        path,
        set: "ascii",
        finding,
        remedy: None,
    }
}

/// Three em dashes, an en dash, then one em dash in another file.
fn found() -> [Finding; 5] {
    let (em, en) = ('\u{2014}', '\u{2013}');
    let at = |byte, c| finding(byte, c);
    [at(0, em), at(3, en), at(6, em), at(9, em), at(0, em)]
}

/// One fold per (file, code point), first occurrence kept, in order.
#[test]
fn a_file_folds_to_one_row_per_code_point() {
    let all = found();
    let paths = ["a.md", "a.md", "a.md", "a.md", "b.md"];
    let lines = paths.iter().zip(&all).map(|(p, f)| line(p, f));
    let folded = groups(lines);
    let said: Vec<(&str, usize, usize)> = folded
        .iter()
        .map(|g| (g.first.path, g.first.finding.hit.position.byte, g.count))
        .collect();
    assert_eq!(said, [("a.md", 0, 3), ("a.md", 3, 1), ("b.md", 0, 1)]);
}

/// The tally counts findings and files and names the commonest first.
#[test]
fn the_tally_names_the_totals_and_the_commonest_code_points() {
    let all = found();
    let paths = ["a.md", "a.md", "a.md", "a.md", "b.md"];
    let lines = paths.iter().zip(&all).map(|(p, f)| line(p, f));
    let want = concat!(
        "5 findings in 2 files; most: U+2014 x4, U+2013 x1 -- see ",
        "'ctrm sets --containing U+2014' or 'ctrm init'"
    );
    assert_eq!(tally(lines), want);
    assert_eq!(tally(std::iter::empty()), "");
}
