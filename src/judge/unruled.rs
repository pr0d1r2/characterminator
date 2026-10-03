//! Judging WITHOUT rules: the hazards alone.
//!
//! Two callers need it, both in the guard (`src/cli/guard:V66`): a file
//! whose rules could not be applied -- a broken `.ctrm` must not open the
//! door to a Trojan Source file -- and a file that is not text, which a
//! harness still decodes lossily and shows. Hazards need no rules, so this
//! is the one judgement that survives both.

use crate::lint::{Finding, Hazards, Lint};
use crate::scan::{Hit, scan_bytes, scan_str};

/// The lint a binary is made of, which a not-text judgement leaves out.
const CONTROL: &str = "control-character";

/// What a rule-less judgement found, and whether the bytes were text.
pub(crate) enum Unruled {
    /// The bytes decode: every hazard in them.
    Text(Vec<Finding>),
    /// They do not: the hazards in their lossy decode, less
    /// `control-character`.
    NotText(Vec<Finding>),
}

/// The hazards in `bytes`, judged as a reader will see them.
///
/// Text is judged as is. Bytes that are not text are judged as the
/// harness SHOWS them, decoded lossily, so a bidi override after a stray
/// `\xff` still counts (`src/cli/guard:V66`). Less `control-character`
/// there: a NUL is what makes a file binary, and C0 bytes are what every
/// binary is made of, so judging them would deny the read of every image.
pub(crate) fn unruled(bytes: &[u8], hazards: &Hazards) -> Unruled {
    match scan_bytes(bytes, |c| !hazards.contains(c)) {
        Ok((text, hits)) => Unruled::Text(found(text, hits, hazards)),
        Err(_) => Unruled::NotText(lossy(bytes, hazards)),
    }
}

/// The hazards among `hits` in `text`, less a joiner or tag inside an RGI
/// emoji sequence (`src/lint:V63`).
fn found(text: &str, hits: Vec<Hit>, hazards: &Hazards) -> Vec<Finding> {
    let exempt = hazards.exempt(text);
    hazards_in(hits, |hit| hazards.lint_at(hit, &exempt))
}

/// The hazards in the lossy decode of bytes that are not text, less
/// `control-character`.
fn lossy(bytes: &[u8], hazards: &Hazards) -> Vec<Finding> {
    let text = String::from_utf8_lossy(bytes);
    let hits = scan_str(&text, |c| !hazards.contains(c));
    let all = found(&text, hits, hazards);
    all.into_iter().filter(|f| f.lint.name != CONTROL).collect()
}

/// The hazards among `hits`, each as the finding the lint node names, at
/// its lint's own level. `lint` is the lint node's answer for one hit:
/// less a joiner or tag inside an RGI emoji sequence (`src/lint:V63`)
/// always, and less a BOM at byte 0 only where the text has a file start.
pub(crate) fn hazards_in(
    hits: Vec<Hit>,
    lint: impl Fn(Hit) -> Option<Lint>,
) -> Vec<Finding> {
    hits.into_iter()
        .filter_map(|hit| {
            let lint = lint(hit)?;
            let level = lint.default_level();
            Some(Finding { hit, lint, level })
        })
        .collect()
}
