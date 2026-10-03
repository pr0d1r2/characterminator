//! The `guard` verb (T37): a hook adapter between an agent harness and the
//! hazard lints (V35, V53).
//!
//! An ADAPTER, not a daemon: one process per hook call, no state. The
//! harness's shape lives in `hook`; this file takes a harness-agnostic
//! call and decides. Detection is not here either. A file is judged by the
//! same `Checker` `check` uses, and tool output by the lint node's
//! `Hazards`, so the guard can never fire on something the gate passes.
//!
//! ONLY A HAZARD BLOCKS (V35). A file holding em dashes is noted and read;
//! a hook that refused it would be switched off within the hour, and a
//! switched-off hook guards nothing. Nothing is stripped either: a guard
//! that quietly removed characters would change what the model reads
//! without anyone being told, which is the harm it exists to prevent.
//!
//! HAZARDS DO NOT DEPEND ON CONFIGURATION (V53). The classes are compiled in
//! (`src/lint/hazard:V49`), so a `.ctrm` that cannot be read or applied still
//! leaves every hazard judged; only the note about it changes.

use super::hook::{self, Call, Event, Verdict};
use super::reason::{denied, denied_binary, file_note, output_note, tainted};
use super::tier::blocks;
use crate::judge::{Checker, Unruled, hazards_in, shown_path, unruled};
use crate::lint::{Finding, Group, Hazards};
use crate::render::codepoint;
use crate::scan::scan_str;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Answer one hook payload: the decision document to print, or the empty
/// string for a silent pass.
///
/// `root` stands in for a payload that names no `cwd`: a command hook runs
/// in the session's directory, so it is the same answer by another route.
///
/// # Errors
///
/// Input that is not a hook payload.
pub(crate) fn run(stdin: &str, root: &Path) -> Result<String, String> {
    let hazards = Hazards::builtin();
    Ok(match hook::call(stdin)? {
        Call::Read { cwd, path } => {
            let cwd = cwd.map_or_else(|| root.to_path_buf(), PathBuf::from);
            let verdict = read(&cwd, &path, &hazards);
            hook::response(Event::Before, &legible(verdict))
        }
        Call::Output { tool, texts } => {
            let verdict = output(&tool, &texts, &hazards);
            hook::response(Event::After, &legible(verdict))
        }
        Call::Other => String::new(),
        Call::Unparsed { text, why } => unparsed(&text, &hazards).ok_or(why)?,
    })
}

/// Input no reader accepts -- broken, or nested past the depth bound --
/// judged as raw text, escapes decoded (V67). A hazard is a `block`
/// decision, which Claude Code honours on either event, and exit 0: an
/// attacker who shapes a tool's output must not turn a hazard into an
/// adapter error that lets it through. `None` is clean, and the caller
/// keeps the named error and its exit 1 (V53).
fn unparsed(text: &str, hazards: &Hazards) -> Option<String> {
    let blocking: Vec<Finding> =
        unsigned(text, hazards).into_iter().filter(blocks).collect();
    let first = blocking.first()?;
    let why = tainted("unparsed hook", "payload", first, blocking.len());
    Some(hook::response(Event::After, &legible(Verdict::Block(why))))
}

/// Every hazard in text with NO file start: tool output, or a payload
/// judged raw (V53), so a BOM at its byte 0 is a stray.
fn unsigned(text: &str, hazards: &Hazards) -> Vec<Finding> {
    let hits = scan_str(text, |c| !hazards.contains(c));
    let exempt = hazards.exempt(text);
    hazards_in(hits, |hit| hazards.lint_unsigned(hit, &exempt))
}

/// A verdict from the notes that apply: none is a silent pass.
fn notes(parts: [Option<String>; 2]) -> Verdict {
    let said: Vec<String> = parts.into_iter().flatten().collect();
    if said.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Note(said.join(" "))
    }
}

/// A reason quotes data -- a path, a field label, a tool's name -- and
/// data can hold the very character it reports. Every message here is
/// ASCII apart from what it quotes, so anything else in it is written as
/// `<U+XXXX>`: a reason must not smuggle what it warns about.
fn legible(verdict: Verdict) -> Verdict {
    match verdict {
        Verdict::Pass => Verdict::Pass,
        Verdict::Note(note) => Verdict::Note(spelled(&note)),
        Verdict::Block(why) => Verdict::Block(spelled(&why)),
    }
}

fn spelled(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' '..='~' => String::from(c),
            _ => format!("<{}>", codepoint(c)),
        })
        .collect()
}

/// A file about to be read, judged against `.ctrm` from `cwd`.
///
/// A file that cannot be read passes: the harness reports a missing file
/// itself, and there are no characters to judge. One that is not text, or
/// whose rules could not be applied, is judged for hazards alone, as the
/// harness will SHOW it (V66): see [`hazards_only`].
fn read(cwd: &Path, path: &str, hazards: &Hazards) -> Verdict {
    let full = cwd.join(path);
    let Some((bytes, whole)) = loaded(&full, CAP) else {
        return Verdict::Pass;
    };
    let shown = shown_path(cwd, &full);
    let (verdict, text) = judged(cwd, &shown, &bytes, hazards);
    if whole || !text {
        verdict
    } else {
        partial(&shown, verdict, bytes.len())
    }
}

/// The most of one file a pre-read judges (V102): 16 MiB. A hook runs
/// once per `Read`, and reading a multi-GB file whole would put GBs in
/// the hook's memory and seconds on every read of it.
const CAP: u64 = 16 * 1024 * 1024;

/// The file's first `cap` bytes, and whether that is all of it. A file
/// longer than that is cut back to its last newline inside the cap: a
/// line break is never inside a UTF-8 character, an emoji sequence, or a
/// bidi pair, so the cut cannot MAKE a hazard (a joiner whose partner
/// was cut off) or a decode error that would deny for size alone.
fn loaded(full: &Path, cap: u64) -> Option<(Vec<u8>, bool)> {
    let file = std::fs::File::open(full).ok()?;
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    let whole = u64::try_from(bytes.len()).is_ok_and(|n| n <= cap);
    if !whole {
        bytes.truncate(cut(&bytes, cap));
    }
    Some((bytes, whole))
}

/// Where a capped prefix ends: after its last newline, else after its
/// last whole UTF-8 character.
fn cut(bytes: &[u8], cap: u64) -> usize {
    let cap = usize::try_from(cap).map_or(bytes.len(), |c| c.min(bytes.len()));
    let head = bytes.get(..cap).unwrap_or(bytes);
    match head.iter().rposition(|b| *b == b'\n') {
        Some(at) => at.saturating_add(1),
        None => match std::str::from_utf8(head) {
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            _ => head.len(),
        },
    }
}

/// A prefix judged in place of the whole file (V102). A hazard in it
/// still denies; otherwise the read passes WITH a note saying how much
/// was judged, never in silence: a pass on a prefix that said nothing
/// would read as a pass on the file.
fn partial(shown: &str, verdict: Verdict, judged: usize) -> Verdict {
    let rest = format!(
        "ctrm: {shown} is over the guard's 16 MiB cap, so only its first \
         {judged} bytes were judged; the rest was NOT. `ctrm check \
         {shown}` judges it whole."
    );
    match verdict {
        Verdict::Block(why) => Verdict::Block(why),
        Verdict::Pass => Verdict::Note(rest),
        Verdict::Note(note) => Verdict::Note(format!("{note} {rest}")),
    }
}

/// `bytes`, judged against `.ctrm` from `cwd`, or for hazards alone, and
/// whether they are text.
fn judged(
    cwd: &Path,
    shown: &str,
    bytes: &[u8],
    hazards: &Hazards,
) -> (Verdict, bool) {
    let config = crate::cli::config::discovered(cwd);
    let checker = Checker::configured(&config);
    let why = match checker.and_then(|c| c.findings(shown, bytes)) {
        Ok(Some(found)) => return (judged_file(shown, &found), true),
        Ok(None) => None,
        Err(why) => Some(why),
    };
    hazards_only(shown, unruled(bytes, hazards), why.as_deref())
}

/// A file judged against its rules: the first BLOCKING hazard denies
/// (V110); the other hazards, and the findings that are not hazards,
/// are noted.
fn judged_file(shown: &str, found: &[Finding]) -> Verdict {
    let (hazards, rest): (Vec<&Finding>, Vec<&Finding>) =
        found.iter().partition(|f| f.lint.group == Group::Hazard);
    let (blocking, noted): (Vec<&Finding>, Vec<&Finding>) =
        hazards.into_iter().partition(|f| blocks(f));
    match blocking.first() {
        Some(first) => Verdict::Block(denied(shown, first, blocking.len())),
        None => notes([file_note(shown, &noted), noted_rules(shown, &rest)]),
    }
}

/// A file judged for hazards alone (`src/judge` `unruled`), and whether
/// it is text. A file `check` skips as not text is still READ, lossily,
/// so it is judged as read (V66); a broken `.ctrm` must not open the
/// door to a Trojan Source file.
fn hazards_only(
    shown: &str,
    unruled: Unruled,
    why: Option<&str>,
) -> (Verdict, bool) {
    match unruled {
        Unruled::NotText(found) => (not_text(shown, &found), false),
        Unruled::Text(found) => (unconfigured_file(shown, &found, why), true),
    }
}

/// A file that is not text: denied on a blocking hazard, else a SILENT
/// pass (V112). An agent reads images and PDFs all session, and the lossy
/// decode of a binary holds invisible characters by chance.
fn not_text(shown: &str, found: &[Finding]) -> Verdict {
    let blocking: Vec<&Finding> = found.iter().filter(|f| blocks(f)).collect();
    blocking.first().map_or(Verdict::Pass, |first| {
        Verdict::Block(denied_binary(shown, first, blocking.len()))
    })
}

/// Text whose rules could not be applied: denied on a blocking hazard,
/// else a note saying why no rule applied, and naming any other hazard.
fn unconfigured_file(
    shown: &str,
    found: &[Finding],
    why: Option<&str>,
) -> Verdict {
    let (blocking, noted): (Vec<&Finding>, Vec<&Finding>) =
        found.iter().partition(|f| blocks(f));
    match blocking.first() {
        Some(first) => Verdict::Block(denied(shown, first, blocking.len())),
        None => {
            notes([Some(unconfigured(shown, why)), file_note(shown, &noted)])
        }
    }
}

/// The note on a file whose rules could not be applied, and holds no
/// blocking hazard.
fn unconfigured(shown: &str, why: Option<&str>) -> String {
    let why = why.unwrap_or_default();
    format!(
        "ctrm: the rules could not be applied to {shown} ({why}), so it \
         was judged for hazards only, and holds no bidi override or tag \
         character."
    )
}

/// Findings that are not hazards: read anyway, with a note naming each
/// lint and its count, so the model knows without being stopped.
fn noted_rules(shown: &str, rest: &[&Finding]) -> Option<String> {
    if rest.is_empty() {
        return None;
    }
    Some(format!(
        "ctrm: {shown} holds characters its rules report ({}). Not \
         blocked; `ctrm check {shown}` lists them and `ctrm fix {shown}` \
         rewrites what the map covers.",
        tally(rest)
    ))
}

/// `outside-set: 3`, one entry per lint, in the order first seen.
fn tally(found: &[&Finding]) -> String {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for finding in found {
        let name = finding.lint.name;
        match counts.iter_mut().find(|(seen, _)| *seen == name) {
            Some((_, count)) => *count = count.saturating_add(1),
            None => counts.push((name, 1)),
        }
    }
    let shown: Vec<String> = counts
        .iter()
        .map(|(name, n)| format!("{name}: {n}"))
        .collect();
    shown.join(", ")
}

/// A tool's output, judged for hazards ONLY.
///
/// Not against a set: output has no path for `.ctrm` to match, so every
/// non-ASCII web page would earn a note under the `ascii` default, and a
/// note that always appears is one nobody reads.
fn output(
    tool: &str,
    texts: &[(String, String)],
    hazards: &Hazards,
) -> Verdict {
    let found = labelled(texts, hazards);
    let (blocking, noted): (Vec<_>, Vec<_>) =
        found.iter().partition(|(_, f)| blocks(f));
    if let Some((at, first)) = blocking.first() {
        return Verdict::Block(tainted(tool, at, first, blocking.len()));
    }
    let at = noted.first().map_or("", |(at, _)| *at);
    let noted: Vec<&Finding> = noted.iter().map(|(_, f)| f).collect();
    output_note(tool, at, &noted).map_or(Verdict::Pass, Verdict::Note)
}

/// Every hazard in every string of output, each with where it sat.
fn labelled<'a>(
    texts: &'a [(String, String)],
    hazards: &Hazards,
) -> Vec<(&'a str, Finding)> {
    texts
        .iter()
        .flat_map(|(at, text)| {
            unsigned(text, hazards)
                .into_iter()
                .map(move |f| (at.as_str(), f))
        })
        .collect()
}

#[cfg(test)]
#[path = "adapter_test.rs"]
mod tests;
