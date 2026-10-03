//! Named blocks between whole-line markers (`dev:V133`, `dev:V134`).
//!
//! A block is the text between a `<!-- BEGIN name -->` line and its
//! `<!-- END name -->` line. A marker is a WHOLE line with the EXACT name,
//! so a block called `x` never matches `<!-- BEGIN xy -->`. Every rendered
//! body ends in a newline and `current` returns the body byte for byte, so
//! splicing a document twice gives the same bytes as splicing it once.

/// Byte offsets of a block's body: after the BEGIN line, up to the END line.
struct Span {
    start: usize,
    end: usize,
}

fn begin(name: &str) -> String {
    format!("<!-- BEGIN {name} -->")
}

fn end(name: &str) -> String {
    format!("<!-- END {name} -->")
}

/// The byte offset where each line equal to `marker` starts.
fn lines_at(doc: &str, marker: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut offset = 0_usize;
    for line in doc.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == marker {
            found.push(offset);
        }
        offset = offset.saturating_add(line.len());
    }
    found
}

/// The offset just past the newline that ends the line starting at `at`.
fn after_line(doc: &str, at: usize) -> Option<usize> {
    let newline = doc.get(at..)?.find('\n')?;
    at.checked_add(newline)?.checked_add(1)
}

fn span(doc: &str, name: &str) -> Result<Span, String> {
    let begins = lines_at(doc, &begin(name));
    let ends = lines_at(doc, &end(name));
    match (begins.as_slice(), ends.as_slice()) {
        ([], _) | (_, []) => {
            Err(format!("no `{}` / `{}` pair", begin(name), end(name)))
        }
        ([b], [e]) if b < e => after_line(doc, *b)
            .map(|start| Span { start, end: *e })
            .ok_or_else(|| format!("`{}` has no line end", begin(name))),
        ([_], [_]) => {
            Err(format!("`{}` comes before `{}`", end(name), begin(name)))
        }
        _ => Err(format!("the `{name}` markers appear more than once")),
    }
}

/// The body of block `name` as the document holds it today.
pub(crate) fn current<'a>(doc: &'a str, name: &str) -> Result<&'a str, String> {
    let at = span(doc, name)?;
    doc.get(at.start..at.end)
        .ok_or_else(|| format!("block `{name}` is not on a character boundary"))
}

/// `doc` with block `name`'s body replaced by `body`; markers kept.
pub(crate) fn replace(
    doc: &str,
    name: &str,
    body: &str,
) -> Result<String, String> {
    let at = span(doc, name)?;
    let head = doc.get(..at.start).unwrap_or_default();
    let tail = doc.get(at.end..).unwrap_or_default();
    Ok(format!("{head}{body}{tail}"))
}

/// Up to 3 lines each side has that the other lacks: what a stale block
/// is told, so a reader sees the drift without running a diff.
pub(crate) fn sample(want: &str, have: &str) -> Vec<String> {
    let only = |a: &str, b: &str, tag: &str| -> Vec<String> {
        a.lines()
            .filter(|line| !b.lines().any(|other| other == *line))
            .take(3)
            .map(|line| format!("{tag}: {line}"))
            .collect()
    };
    let mut lines = only(want, have, "want");
    lines.extend(only(have, want, "have"));
    lines
}

#[cfg(test)]
#[path = "splice_test.rs"]
mod tests;
