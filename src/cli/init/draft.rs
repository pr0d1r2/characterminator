//! The draft `.ctrm` text (V128): one commented line per file type that
//! needs a grant, comments for what needs none, and the hazards named as
//! work to do, never granted.

use super::cover::{Candidates, Grant};
use super::survey::{Hazard, Kind, Survey};
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
        && kind.fixable().is_empty()
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
    let grant =
        (!kind.needed.is_empty()).then(|| candidates.cover(&kind.needed));
    let rule = if let Some(grant) = &grant {
        Some(granted(pattern, grant, &mut lines))
    } else {
        lines.push(String::from("# Needs no grant."));
        None
    };
    let allowed = |c| grant.as_ref().is_some_and(|g| candidates.allows(g, c));
    lines.extend(notes(kind, &allowed));
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
fn notes(kind: &Kind, allowed: &dyn Fn(char) -> bool) -> Vec<String> {
    let mut lines = rewritten(kind, allowed);
    if kind.sequences {
        lines.push(String::from(
            "# Emoji sequences: `ctrm fix` compresses them to one emoji.",
        ));
    }
    lines.extend(wrapped(
        "Hazards, never granted -- fix these:",
        &hazards(kind),
    ));
    lines
}

/// One entry per hazard code point: its lints and every place it sits.
fn hazards(kind: &Kind) -> Vec<String> {
    let each = kind.hazards.iter();
    each.map(|(c, found)| {
        let (lints, at) = (&found.lints, listed(found));
        format!("{} {lints} in {at}", codepoint(*c))
    })
    .collect()
}

/// `a:1:3, b:2:4`, cut at `PLACES` with `and K more -- ctrm check`.
fn listed(found: &Hazard) -> String {
    let mut text = found.places.join(", ");
    let more = found.total.saturating_sub(found.places.len());
    if more > 0 {
        text.push_str(&format!(" and {more} more -- ctrm check"));
    }
    text
}

/// The characters the builtin map rewrites: what `check` reports on them
/// until `fix` runs, and the ones the rule line grants anyway, which are
/// no finding and which `fix` therefore leaves (`src/fix:V6`).
fn rewritten(kind: &Kind, allowed: &dyn Fn(char) -> bool) -> Vec<String> {
    let (kept, fixed): (Vec<char>, Vec<char>) =
        kind.fixable().into_iter().partition(|c| allowed(*c));
    let mut lines = wrapped(&until_fix(kind.unfixed(allowed)), &points(&fixed));
    let lead = "The builtin map rewrites these, but this line grants them:";
    lines.extend(wrapped(lead, &points(&kept)));
    lines
}

/// What leaving the rewritable characters ungranted costs until `fix` runs,
/// and the way to grant them that works once this draft is `.ctrm`: a plain
/// `init` then refuses to overwrite it, `--print` never does (V128).
fn until_fix((findings, files): (usize, usize)) -> String {
    format!(
        "`ctrm fix` rewrites these; until it runs, check reports \
         {} in {} (`init --print --keep-fixable` drafts them granted):",
        counted(findings, "finding"),
        counted(files, "file")
    )
}

/// `1 file`, `2 files`: the noun takes its count's number, as `check`'s
/// own tally does (B82).
fn counted(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

fn points(chars: &[char]) -> Vec<String> {
    chars.iter().map(|c| codepoint(*c)).collect()
}

/// `# <lead> a b c` wrapped at 72 columns, the lead's own words too;
/// nothing when `items` is empty.
fn wrapped(lead: &str, items: &[String]) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut words = lead.split(' ');
    let mut lines = vec![format!("# {}", words.next().unwrap_or_default())];
    for item in words.chain(items.iter().map(String::as_str)) {
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
