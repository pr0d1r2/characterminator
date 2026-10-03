//! Bounded `check` output (V124): findings folded per (file, code point),
//! and the tally a human report ends with.
//!
//! A real tree reported 2,136 rows with no total, and ten thousand
//! findings cost an agent about 80k tokens to read. Folding them keeps
//! one row per character a file holds, which is the unit anyone acts on:
//! a grant, a map line or one edit fixes every occurrence at once.

use crate::render::line::Line;
use std::collections::HashMap;
use std::fmt::Write;

/// One (path, code point, lint, set): its first occurrence and how many.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fold<'a> {
    pub first: Line<'a>,
    pub count: usize,
}

/// What tells two findings of one file apart in a summary.
type Key<'a> = (char, &'a str, &'a str);

fn key<'a>(line: &Line<'a>) -> Key<'a> {
    (line.finding.hit.character, line.finding.lint.name, line.set)
}

/// `lines`, in report order, folded: groups in the order their first
/// occurrence was reported, so the summary reads top to bottom like the
/// full report it stands for.
pub(super) fn groups<'a>(
    lines: impl IntoIterator<Item = Line<'a>>,
) -> Vec<Fold<'a>> {
    let mut out: Vec<Fold<'a>> = Vec::new();
    let mut seen: HashMap<Key<'a>, usize> = HashMap::new();
    let mut path: Option<&str> = None;
    for line in lines {
        if path != Some(line.path) {
            seen.clear();
            path = Some(line.path);
        }
        fold(&mut out, &mut seen, line);
    }
    out
}

fn fold<'a>(
    out: &mut Vec<Fold<'a>>,
    seen: &mut HashMap<Key<'a>, usize>,
    line: Line<'a>,
) {
    let at = *seen.entry(key(&line)).or_insert(out.len());
    match out.get_mut(at) {
        Some(group) => group.count = group.count.saturating_add(1),
        None => out.push(Fold {
            first: line,
            count: 1,
        }),
    }
}

/// What the tally counts, in one pass over the report.
#[derive(Default)]
struct Counts<'a> {
    total: usize,
    files: usize,
    path: Option<&'a str>,
    by_char: HashMap<char, usize>,
}

impl<'a> Counts<'a> {
    fn add(&mut self, line: &Line<'a>) {
        self.total = self.total.saturating_add(1);
        if self.path != Some(line.path) {
            self.files = self.files.saturating_add(1);
            self.path = Some(line.path);
        }
        let n = self.by_char.entry(line.finding.hit.character).or_insert(0);
        *n = n.saturating_add(1);
    }

    /// The three commonest code points, most first, ties by code point.
    fn commonest(self) -> Vec<(char, usize)> {
        let mut all: Vec<(char, usize)> = self.by_char.into_iter().collect();
        all.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        all.truncate(3);
        all
    }
}

/// The footer of a human `check` (V124), for stderr: how many findings in
/// how many files, the three commonest code points, and where to go next.
/// Empty when there were none: a clean run stays silent.
pub(super) fn tally<'a>(lines: impl IntoIterator<Item = Line<'a>>) -> String {
    let mut counts = Counts::default();
    lines.into_iter().for_each(|line| counts.add(&line));
    if counts.total == 0 {
        return String::new();
    }
    let head = format!(
        "{} in {}; most:",
        plural(counts.total, "finding"),
        plural(counts.files, "file")
    );
    head + &most(&counts.commonest())
}

fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// ` U+2014 x490, U+00E9 x120 -- see 'ctrm sets --containing U+2014' or
/// 'ctrm init'`: the commands another node answers with what to grant.
fn most(top: &[(char, usize)]) -> String {
    let mut out = String::new();
    for (at, (c, n)) in top.iter().enumerate() {
        let comma = if at == 0 { "" } else { "," };
        let _infallible = write!(out, "{comma} {} x{n}", super::codepoint(*c));
    }
    let first = top
        .first()
        .map_or_else(String::new, |t| super::codepoint(t.0));
    let _infallible = write!(
        out,
        " -- see 'ctrm sets --containing {first}' or 'ctrm init'"
    );
    out
}

#[cfg(test)]
#[path = "group_test.rs"]
mod tests;
