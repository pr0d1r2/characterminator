//! The draft `.ctrm` text (V128): one commented line per file type that
//! needs a grant, comments for what needs none, and the hazards named as
//! work to do, never granted.

use super::cover::{Candidates, Grant};
use super::survey::{Kind, Survey};
use crate::render::codepoint;

const HEADER: &str = "\
# Drafted by `ctrm init` from the tracked files. Review it before you
# commit it: every grant is a decision. A pattern with no `/` matches a
# file name at any depth; the last matching line wins; a path no line
# matches gets `ascii`. `ctrm explain <path>` says which line decides.";

/// The whole draft, deterministic: kinds in pattern order, code points
/// in code point order.
pub(super) fn draft(survey: &Survey, candidates: &Candidates) -> String {
    let mut out = vec![String::from(HEADER), summary(survey)];
    for (pattern, kind) in &survey.kinds {
        if !is_ascii(kind) {
            out.push(section(pattern, kind, candidates));
        }
    }
    let mut text = out.join("\n\n");
    text.push('\n');
    text
}

fn is_ascii(kind: &Kind) -> bool {
    kind.needed.is_empty()
        && kind.fixable.is_empty()
        && kind.hazards.is_empty()
        && !kind.sequences
}

/// The file count and the types that need no line at all.
fn summary(survey: &Survey) -> String {
    let files: usize = survey.kinds.values().map(|kind| kind.files).sum();
    let plain: Vec<String> = survey
        .kinds
        .iter()
        .filter(|(_, kind)| is_ascii(kind))
        .map(|(pattern, kind)| format!("{pattern} ({})", kind.files))
        .collect();
    let mut lines = vec![format!(
        "# {files} text files read, {} skipped (binary or not UTF-8).",
        survey.skipped
    )];
    lines.extend(wrapped("Pure ASCII, no line needed:", &plain));
    lines.join("\n")
}

/// One file type: why, then the rule line when it needs one.
fn section(pattern: &str, kind: &Kind, candidates: &Candidates) -> String {
    let mut lines = vec![format!("# {pattern}, files: {}.", kind.files)];
    let rule = if kind.needed.is_empty() {
        lines.push(String::from("# Needs no grant."));
        None
    } else {
        let grant = candidates.cover(&kind.needed);
        Some(granted(pattern, &grant, &mut lines))
    };
    lines.extend(notes(kind));
    lines.extend(rule);
    lines.join("\n")
}

/// The rule line, with a comment per set saying what it is there for.
fn granted(pattern: &str, grant: &Grant, lines: &mut Vec<String>) -> String {
    match grant {
        Grant::Sets(sets) => {
            for (name, covers) in sets {
                lines.extend(wrapped(&format!("{name} for"), &points(covers)));
            }
            let names: Vec<&str> =
                sets.iter().map(|(n, _)| n.as_str()).collect();
            format!("{pattern} ascii+{}", names.join("+"))
        }
        Grant::Any(left) => {
            lines.extend(wrapped("No preset or locale covers", &points(left)));
            lines.push(String::from(ANY));
            format!("{pattern} any")
        }
    }
}

const ANY: &str = "# so `any`, which grants everything (hazards still fire).\n\
                   # A set of your own in `.ctrm-sets` is tighter.";

/// What `fix` handles and what must be fixed by hand.
fn notes(kind: &Kind) -> Vec<String> {
    let fixable: Vec<char> = kind.fixable.iter().copied().collect();
    let mut lines =
        wrapped("`ctrm fix` rewrites, no grant needed:", &points(&fixable));
    if kind.sequences {
        lines.push(String::from(
            "# Emoji sequences: `ctrm fix` compresses them to one emoji.",
        ));
    }
    let hazards: Vec<String> = kind
        .hazards
        .iter()
        .map(|(c, (lint, path))| format!("{} {lint} in {path}", codepoint(*c)))
        .collect();
    lines.extend(wrapped("Hazards, never granted -- fix these:", &hazards));
    lines
}

fn points(chars: &[char]) -> Vec<String> {
    chars.iter().map(|c| codepoint(*c)).collect()
}

/// `# <lead> a b c` wrapped at 72 columns; nothing when `items` is empty.
fn wrapped(lead: &str, items: &[String]) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![format!("# {lead}")];
    for item in items {
        appended(&mut lines, item);
    }
    lines
}

/// `item` on the last line if it fits, else on a continuation line.
fn appended(lines: &mut Vec<String>, item: &str) {
    match lines.last_mut() {
        Some(line) if line.len().saturating_add(item.len()) < 72 => {
            line.push(' ');
            line.push_str(item);
        }
        _ => lines.push(format!("#   {item}")),
    }
}
