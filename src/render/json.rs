//! The JSON form: the STABLE CONTRACT (V11).
//!
//! What that means in practice: a key does not change its name, its type or
//! its meaning under a caller, and the tests below assert whole documents
//! EXACTLY rather than probing field by field. A contract that only promises
//! "contains what you need" is not one, because nothing then catches a key
//! quietly renamed.
//!
//! One compact document per run, and pure ASCII: see `value` and `escape`.
//!
//! A document carries what the invariants NAME and no more. A violation
//! carries its position in all three units and its code point
//! (`src/scan:V12`), the set that did not contain it (the root spec's
//! interface section), and the lint name and level (`src/lint:V36`). The
//! lint's GROUP is deliberately absent: it follows from the name, and a key
//! added to a stable contract is a key that can never be taken back.

use crate::charset::{CharRange, CharSet};
use crate::lint::Level;
use crate::render::Violation;
use crate::render::escape::string;
use crate::render::line::{Line, Spelled};
use crate::render::name::{Codepoint, codepoint, level_name, method_name};
use crate::render::order;
use crate::render::value::{
    Fields, array, field, list, number, object, optional,
};
use crate::render::{Change, Explanation, FileStats, InForce, Skipped};
use crate::rules::{LevelChoice, Origin, Rule, Sourced};
use crate::scan::{Hit, Unreadable};
use crate::tokens::Count;
use std::borrow::Cow;
use std::path::Path;

/// About what one violation object costs, so a report of millions is
/// sized once rather than grown by doubling (R17).
const VIOLATION_BYTES: usize = 160;

/// `check`: every violation, and every file that could not be read.
#[cfg(test)]
pub(super) fn check(
    items: &[Violation<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    check_lines(order::lines(items), skipped)
}

/// [`check`] over violations ALREADY in report order.
pub(super) fn check_lines<'a>(
    lines: impl IntoIterator<Item = Line<'a>>,
    skipped: &[Skipped<'_>],
) -> String {
    let mut out = String::new();
    let mut doc = Fields::open(&mut out);
    doc.text("verb", "check");
    violations(doc.key("violations"), lines);
    doc.raw("skipped", &unread(skipped));
    doc.close();
    out
}

/// The array of violation objects, each path escaped once per file.
fn violations<'a>(out: &mut String, lines: impl IntoIterator<Item = Line<'a>>) {
    let lines = lines.into_iter();
    out.reserve(lines.size_hint().0.saturating_mul(VIOLATION_BYTES));
    let mut path = Spelled::new(literal);
    list(out, lines, |out, item| {
        violation(out, path.of(item.path), item)
    });
}

/// A path as a json literal, for [`Spelled`].
fn literal(path: &str) -> Cow<'_, str> {
    Cow::Owned(string(path))
}

/// `path` is the json literal, already escaped.
fn violation(out: &mut String, path: &str, item: Line<'_>) {
    let mut fields = Fields::open(out);
    fields.raw("path", path);
    hit_fields(&mut fields, item.finding.hit);
    fields.text("set", item.set);
    fields.text("lint", item.finding.lint.name);
    fields.text("level", level_name(item.finding.level));
    fields.close();
}

/// The position in all three units, plus the character itself: a consumer
/// that wants to show it should not have to parse `U+XXXX` back into a code
/// point, and the escaper keeps it ASCII on the way out.
fn hit_fields(fields: &mut Fields<'_>, hit: Hit) {
    fields.number("line", hit.position.line);
    fields.number("column", hit.position.column);
    fields.number("byte", hit.position.byte);
    fields.number(
        "codepoint",
        format_args!("\"{}\"", Codepoint(hit.character)),
    );
    let mut buffer = [0_u8; 4];
    fields.text("character", hit.character.encode_utf8(&mut buffer));
}

fn unread(items: &[Skipped<'_>]) -> String {
    let rendered: Vec<String> =
        order::skipped(items).into_iter().map(unread_item).collect();
    array(&rendered)
}

fn unread_item(item: &Skipped<'_>) -> String {
    let mut fields = vec![field("path", &string(item.path))];
    fields.extend(reason_fields(item.reason));
    object(&fields)
}

fn reason_fields(reason: Unreadable) -> Vec<String> {
    match reason {
        Unreadable::NotUtf8 { byte } => vec![
            field("reason", &string("not-utf8")),
            field("byte", &number(byte)),
        ],
        Unreadable::Binary => vec![field("reason", &string("binary"))],
    }
}

/// `fix`: every rewrite, every character no map entry covers (`src/fix:V4`)
/// in `check`'s violation shape, and every file that could not be read.
pub(super) fn fix(
    items: &[Change<'_>],
    unmapped: &[Violation<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    let mut out = String::new();
    let mut doc = Fields::open(&mut out);
    doc.text("verb", "fix");
    let mut path = Spelled::new(literal);
    let rewrites = order::changes(items);
    list(doc.key("rewrites"), rewrites, |out, item| {
        change(out, path.of(item.path), item);
    });
    violations(doc.key("unmapped"), order::lines(unmapped));
    doc.raw("skipped", &unread(skipped));
    doc.close();
    out
}

fn change(out: &mut String, path: &str, item: &Change<'_>) {
    let mut fields = Fields::open(out);
    fields.raw("path", path);
    hit_fields(&mut fields, item.rewrite.hit);
    fields.text("to", &item.rewrite.to);
    fields.close();
}

/// `stats`: one row per file, and every file that is not text.
pub(super) fn stats(
    files: &[FileStats<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    let rows: Vec<String> = order::stats(files).into_iter().map(row).collect();
    object(&[
        field("verb", &string("stats")),
        field("files", &array(&rows)),
        field("skipped", &unread(skipped)),
    ])
}

fn row(item: &FileStats<'_>) -> String {
    object(&[
        field("path", &string(item.path)),
        field("outside", &number(item.outside)),
        field("bytes", &number(item.bytes)),
        field("tokens_now", &count(item.now)),
        field("tokens_after", &count(item.after)),
    ])
}

/// The method travels WITH the number (`src/tokens:V10`), as an object
/// rather than as a suffix on a string: a consumer should not have to strip
/// a tilde to get at the figure.
fn count(value: Count) -> String {
    object(&[
        field("tokens", &number(value.tokens)),
        field("method", &string(method_name(value.method))),
    ])
}

/// `explain`: the effective set, and the rule that won.
pub(super) fn explain(item: &Explanation<'_>) -> String {
    object(&[
        field("verb", &string("explain")),
        field("path", &optional(item.path)),
        field("set", &char_set(item.set)),
        field("rule", &item.rule.map_or_else(null, rule)),
        field("effective", &in_force(&item.in_force)),
    ])
}

/// What is in force, each setting with its line (`src/rules:V20`, B28).
/// ADDED after `rule`, so every key a consumer already read keeps its
/// place and meaning (V11). A null origin is the default family.
fn in_force(item: &InForce<'_>) -> String {
    let family = object(&[
        field("name", &string(item.family)),
        field("origin", &item.family_origin.map_or_else(null, origin)),
    ]);
    let levels: Vec<String> = item.levels.iter().map(sourced_level).collect();
    object(&[
        field("family", &family),
        field(
            "default_level",
            &item.level.as_ref().map_or_else(null, bare),
        ),
        field("levels", &array(&levels)),
    ])
}

fn bare(item: &Sourced<'_, Level>) -> String {
    object(&[
        field("level", &string(level_name(item.value))),
        field("origin", &origin(item.origin)),
    ])
}

fn sourced_level(item: &Sourced<'_, LevelChoice>) -> String {
    object(&[
        field("target", &string(&item.value.target)),
        field("level", &string(level_name(item.value.level))),
        field("origin", &origin(item.origin)),
    ])
}

/// `sets`: the builtin sets and what they hold.
pub(super) fn sets(items: &[CharSet]) -> String {
    let rendered: Vec<String> = items.iter().map(char_set).collect();
    object(&[
        field("verb", &string("sets")),
        field("sets", &array(&rendered)),
    ])
}

fn char_set(item: &CharSet) -> String {
    let ranges: Vec<String> = item.ranges.iter().map(range).collect();
    object(&[
        field("name", &string(&item.name)),
        field("ranges", &array(&ranges)),
    ])
}

/// Endpoints as `U+XXXX` rather than as bare numbers: the same spelling the
/// human form uses, and one a person can read in a raw document.
fn range(item: &CharRange) -> String {
    object(&[
        field("start", &string(&codepoint(item.start))),
        field("end", &string(&codepoint(item.end))),
    ])
}

/// The absent rule: nothing matched, so `src/rules:V1` decided.
fn null() -> String {
    String::from("null")
}

/// `default_level` is the bare `!<level>` a rule line wrote, or `null` when
/// it wrote none (T43). It sits beside `levels` rather than inside it as a
/// choice with a magic target, for the reason the `Rule` field does: the
/// bare form names no target. `null` rather than the group default a run
/// falls back to, because the document reports what the RULE says, and a
/// synthesised level would put words in a line that never wrote them.
fn rule(item: &Rule) -> String {
    let sets: Vec<String> = item.sets.iter().map(|set| string(set)).collect();
    let levels: Vec<String> = item.levels.iter().map(level_choice).collect();
    let bare = item.default_level;
    object(&[
        field("pattern", &string(&item.pattern)),
        field("sets", &array(&sets)),
        field("family", &optional(item.family.as_deref())),
        field("levels", &array(&levels)),
        field("default_level", &optional(bare.map(level_name))),
        field("origin", &origin(&item.origin)),
    ])
}

fn level_choice(item: &LevelChoice) -> String {
    object(&[
        field("target", &string(&item.target)),
        field("level", &string(level_name(item.level))),
    ])
}

/// A tagged union: `kind` says which case, and the remaining keys follow
/// from it.
fn origin(item: &Origin) -> String {
    match item {
        Origin::File { path, line } => file_origin(path, *line),
        Origin::Argument { index } => object(&[
            field("kind", &string("argument")),
            field("index", &number(*index)),
        ]),
        Origin::Builtin { line } => object(&[
            field("kind", &string("builtin")),
            field("line", &number(*line)),
        ]),
    }
}

/// The path is rendered LOSSILY. What a non-UTF-8 path should print is a
/// rendering question and this is the honest answer: the replacement
/// character, which the escaper then writes as `\ufffd` -- visibly wrong in
/// the output rather than silently dropped from it.
fn file_origin(path: &Path, line: usize) -> String {
    object(&[
        field("kind", &string("file")),
        field("path", &string(&path.to_string_lossy())),
        field("line", &number(line)),
    ])
}

#[cfg(test)]
#[path = "json_test.rs"]
mod tests;
