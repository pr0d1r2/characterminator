//! The files that OWN each generated value, read with pure line readers
//! (`dev:V131`, `dev:C`). Nothing here touches the disk: the caller reads
//! the file and hands its text in, so every rule is testable on a string.

use std::collections::BTreeSet;

/// One package of the closure, as `cargo tree -f '{p}|{l}'` prints it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Pkg {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) licence: String,
}

/// The table a TOML header line opens: `[a.b]` is `a.b`, `[[bin]]` is `bin`.
fn header(line: &str) -> Option<&str> {
    let inner = line.trim().strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim_matches(['[', ']']).trim())
}

/// The lines of one table, headers excluded.
fn in_table<'a>(
    toml: &'a str,
    table: &'a str,
) -> impl Iterator<Item = &'a str> {
    toml.lines()
        .scan("", move |current, line| match header(line) {
            Some(name) => {
                *current = name;
                Some(None)
            }
            None => Some((*current == table).then_some(line)),
        })
        .flatten()
}

/// `key = value` where a key STARTS: column 0, not a comment. A wrapped
/// inline table's continuation lines are indented, so they never count.
fn key_value(line: &str) -> Option<(&str, &str)> {
    if line.starts_with([' ', '\t', '#']) {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

/// A string or bare value of `key` in `table`, quotes removed.
pub(crate) fn value(toml: &str, table: &str, key: &str) -> Option<String> {
    in_table(toml, table)
        .filter_map(key_value)
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.trim_matches('"').to_owned())
}

/// Direct dependencies: one per key in `[dependencies]`, plus one per
/// `[dependencies.<name>]` table.
pub(crate) fn dependency_count(toml: &str) -> usize {
    let keys = in_table(toml, "dependencies")
        .filter_map(key_value)
        .map(|_| ());
    let tables = toml
        .lines()
        .filter_map(header)
        .filter(|h| h.starts_with("dependencies."))
        .map(|_| ());
    keys.chain(tables).count()
}

/// The `unsafe_code` level, from the workspace table or the package's own.
pub(crate) fn unsafe_level(toml: &str) -> Option<String> {
    value(toml, "workspace.lints.rust", "unsafe_code")
        .or_else(|| value(toml, "lints.rust", "unsafe_code"))
}

/// `.coverage`'s `lines` figure, TRUNCATED to one decimal as text
/// (`dev:V135`): `97.59` is `97.5`, never `97.6`.
pub(crate) fn coverage(text: &str) -> Option<String> {
    let figure = text
        .lines()
        .find_map(|line| line.strip_prefix("lines "))?
        .trim();
    let (whole, fraction) = figure.split_once('.').unwrap_or((figure, "0"));
    let tenth = fraction.chars().next().unwrap_or('0');
    let digits = !whole.is_empty() && whole.chars().all(|c| c.is_ascii_digit());
    (digits && tenth.is_ascii_digit()).then(|| format!("{whole}.{tenth}"))
}

fn package(line: &str) -> Option<Pkg> {
    let (left, licence) = line.split_once('|')?;
    let mut words = left.split_whitespace();
    let name = words.next()?.to_owned();
    let version = words.next()?.strip_prefix('v')?.to_owned();
    let licence = licence.trim().trim_end_matches("(*)").trim().to_owned();
    Some(Pkg {
        name,
        version,
        licence,
    })
}

/// Every package `cargo tree --prefix none -f '{p}|{l}'` printed, deduped.
pub(crate) fn closure(tree: &str) -> BTreeSet<Pkg> {
    tree.lines().filter_map(package).collect()
}

#[cfg(test)]
#[path = "owners_test.rs"]
mod tests;
