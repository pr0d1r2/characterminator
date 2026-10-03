//! Applying a map: the `fix` pass and the `--check` reporting mode.
//!
//! One pass walks the text and rewrites a span ONLY when the span covers a
//! character the caller's predicate disallows, or starts at a hazard the
//! caller's law names (V104), so every other byte is copied through
//! untouched (V6). A character with no mapping is copied through and
//! reported rather than dropped (V4). A replacement is itself run through
//! the map before it is emitted, so a chain (skin tone to thumbs up to `+1`)
//! lands on its fixed point in one pass and a second run changes nothing
//! (V5) -- unless a rewrite revealed a sequence the scan had passed, and
//! then the passes repeat until one changes nothing (`src/fix/emoji:V65`).
//!
//! Whether a character is ALLOWED is not decided here. It arrives as a
//! predicate, because that answer belongs to the charset and rules nodes.

use crate::fix::emoji::{Layer, SETTLE, Trail, itself};
use crate::fix::map::Map;
use crate::fix::walk::{Ground, Pass, Span, run};
use crate::fix::{Error, Law, Rewrite};
use crate::scan::Hit;

/// What `fix` found, with no text to write: the `fix --check` answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Report {
    /// Every span that would be rewritten, in the order they were found.
    pub rewrites: Vec<Rewrite>,
    /// Every disallowed character that has no mapping, kept as it was (V4).
    pub unmapped: Vec<Hit>,
}

impl Report {
    /// Whether writing would change the file. `fix --check` exits 1 on it.
    #[must_use]
    pub fn drifted(&self) -> bool {
        !self.rewrites.is_empty()
    }
}

/// The rewritten text together with the report that explains it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fixed {
    pub output: String,
    pub report: Report,
    /// Every pass that changed its input, to map `output` back by.
    trail: Trail,
}

impl Fixed {
    /// Where each hit IN `output` sits in `original`, the text this was fixed
    /// from: byte, line and column. A hit inside a replacement is placed at the
    /// start of the span it replaced (`src/fix/emoji:V65`). This is how a
    /// caller that judges what is LEFT reports it in the file on disk.
    #[must_use]
    pub fn origins(&self, original: &str, hits: &[Hit]) -> Vec<Hit> {
        self.trail.origins(original, hits)
    }

    /// [`Fixed::origins`] IN PLACE, over any row that holds a hit: a
    /// caller judging millions of leftovers moves each one where it sits
    /// rather than copying every hit out and back (R18).
    pub(crate) fn place<T>(
        &self,
        original: &str,
        items: &mut [T],
        hit: fn(&mut T) -> &mut Hit,
    ) {
        self.trail.place(original, items, hit);
    }
}

/// Rewrite every disallowed span of `text` that the map covers.
///
/// The V6 and V5 guards run BEFORE this returns, so a caller that writes
/// what it gets back cannot write a file that failed either.
///
/// Passes repeat until one changes nothing (`src/fix/emoji:V65`), each guarded
/// by V6 on its own input. Every row is reported in the ORIGINAL text's
/// coordinates: a later pass read text an earlier one rewrote, so its positions
/// are mapped back through each earlier pass (`src/fix/emoji:B24`). A text that
/// settles in one pass -- every text before the sequence map -- maps through
/// nothing and is reported exactly as before.
///
/// ```
/// use characterminator::{Map, fix};
///
/// let fixed = fix("a \u{2026} b", &Map::builtin(), |c| c.is_ascii())?;
/// assert_eq!(fixed.output, "a ... b");
/// assert_eq!(fixed.report.rewrites.len(), 1);
/// # Ok::<(), characterminator::FixError>(())
/// ```
///
/// # Errors
///
/// A rewrite that would touch an allowed byte (V6) or would not settle
/// (V5): nothing is returned to write.
pub fn fix(
    text: &str,
    map: &Map,
    allowed: impl Fn(char) -> bool,
) -> Result<Fixed, Error> {
    let law = Law {
        allowed: &allowed,
        hazards: &|_| Vec::new(),
    };
    fix_under(text, map, law)
}

/// [`fix`], under a law that also knows the file's hazards (V104): each
/// one is rewritten whatever the set grants -- by the map where it maps,
/// else deleted when it carries no visible text. Every guard of [`fix`]
/// holds unchanged; V6 spares the hazards and nothing else.
///
/// # Errors
///
/// As [`fix`].
pub(crate) fn fix_under(
    text: &str,
    map: &Map,
    law: Law<'_>,
) -> Result<Fixed, Error> {
    let mut seen = History::new(text);
    let mut pass = guarded(text, map, law)?;
    for _ in 0..SETTLE {
        let again = guarded(&pass.output, map, law)?;
        if again.output == pass.output {
            return Ok(seen.settled(pass));
        }
        seen.record(pass.spans);
        pass = again;
    }
    Err(Error::NotIdempotent)
}

/// The passes so far: what they reported, already in the original text's
/// coordinates, and each one's own spans, in the coordinates it read.
#[derive(Debug)]
struct History<'t> {
    original: &'t str,
    report: Report,
    trail: Trail,
}

impl<'t> History<'t> {
    const fn new(original: &'t str) -> Self {
        Self {
            original,
            report: Report {
                rewrites: Vec::new(),
                unmapped: Vec::new(),
            },
            trail: Trail::new(),
        }
    }

    /// A pass that changed its input: its rewrites join the report, placed
    /// through the layers BEFORE this one, and its spans become the layer
    /// later positions are mapped through. The spans are consumed, so no
    /// replacement is ever held twice (R18).
    fn record(&mut self, spans: Vec<Span>) {
        if spans.is_empty() {
            return;
        }
        let layer = Layer::of(spans.iter().map(Span::extent));
        let mut rows: Vec<Rewrite> =
            spans.into_iter().map(Span::into_rewrite).collect();
        self.trail.place(self.original, &mut rows, rewrite_hit);
        if self.report.rewrites.is_empty() {
            self.report.rewrites = rows;
        } else {
            self.report.rewrites.append(&mut rows);
        }
        self.trail.push(layer);
    }

    /// The pass whose output is the fixed point. Its unmapped characters
    /// are the ones left, located where they sit in the original.
    fn settled(mut self, pass: Pass) -> Fixed {
        let Pass {
            output,
            spans,
            mut unmapped,
        } = pass;
        self.trail.place(self.original, &mut unmapped, itself);
        self.report.unmapped = unmapped;
        self.record(spans);
        self.report.rewrites.sort_by_key(|r| r.hit.position.byte);
        Fixed {
            output,
            report: self.report,
            trail: self.trail,
        }
    }
}

/// One pass, refused if it touched a byte outside a violation (V6).
fn guarded(text: &str, map: &Map, law: Law<'_>) -> Result<Pass, Error> {
    let ground = Ground {
        map,
        allowed: law.allowed,
        budget: map.budget(),
    };
    let pass = run(text, ground, (law.hazards)(text))?;
    if untouched_bytes_match(text, &pass) {
        Ok(pass)
    } else {
        Err(Error::TouchedAllowedBytes)
    }
}

/// Report what `fix` would do and hand back nothing to write (`--check`).
/// Test-only: the binary reads `Fixed::report` from the one `fix` it runs.
#[cfg(test)]
pub(super) fn check(
    text: &str,
    map: &Map,
    allowed: &dyn Fn(char) -> bool,
) -> Result<Report, Error> {
    Ok(fix(text, map, allowed)?.report)
}

/// A rewrite's hit, for [`Trail::place`].
fn rewrite_hit(rewrite: &mut Rewrite) -> &mut Hit {
    &mut rewrite.hit
}
/// V6, asserted BEFORE a caller can write: every byte outside a rewritten
/// span is identical in the input and the output.
fn untouched_bytes_match(text: &str, pass: &Pass) -> bool {
    let mut cut = Cut::default();
    for span in &pass.spans {
        if !cut.gap_matches(text, &pass.output, span.hit.position.byte) {
            return false;
        }
        cut.skip(span);
    }
    cut.tail_matches(text, &pass.output)
}

/// How far the comparison has read, on each side.
#[derive(Debug, Default)]
struct Cut {
    input: usize,
    output: usize,
}

impl Cut {
    fn gap_matches(&self, text: &str, out: &str, next: usize) -> bool {
        let before = text.get(self.input..next);
        let end = self.output.checked_add(next.saturating_sub(self.input));
        let after = end.and_then(|stop| out.get(self.output..stop));
        before.is_some() && before == after
    }

    /// Where `span`'s replacement begins in the output: the oracle the
    /// trail's layer is tested against.
    #[cfg(test)]
    fn start_of(&self, span: &Span) -> usize {
        let gap = span.hit.position.byte.saturating_sub(self.input);
        self.output.saturating_add(gap)
    }

    fn skip(&mut self, span: &Span) {
        let gap = span.hit.position.byte.saturating_sub(self.input);
        self.output = self
            .output
            .saturating_add(gap)
            .saturating_add(span.to.len());
        self.input = span.hit.position.byte.saturating_add(span.len);
    }

    fn tail_matches(&self, text: &str, out: &str) -> bool {
        text.get(self.input..) == out.get(self.output..)
    }
}

#[cfg(test)]
#[path = "apply_test.rs"]
mod tests;
