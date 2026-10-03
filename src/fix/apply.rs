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

use crate::fix::emoji::{Extent, Layer, Pairing, SETTLE, Trail, itself};
use crate::fix::map::{Map, Match, violates};
use crate::fix::words::Gap;
use crate::fix::{Error, Hazard, Law, Rewrite};
use crate::scan::{Hit, Position};

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

/// One walk over one text.
#[derive(Debug, Default)]
struct Pass {
    output: String,
    spans: Vec<Span>,
    unmapped: Vec<Hit>,
}

/// A rewritten span. The public `Rewrite` names the first character; the
/// byte length stays here, because it is what the V6 guard needs.
#[derive(Debug)]
struct Span {
    hit: Hit,
    len: usize,
    to: String,
}

impl Span {
    /// What the trail needs of this span (`src/fix/emoji:V65`).
    fn extent(&self) -> Extent {
        Extent {
            byte: self.hit.position.byte,
            len: self.len,
            to: self.to.len(),
        }
    }

    /// This span as the public row, its replacement MOVED rather than
    /// copied: a pass's spans are spent once its layer is built (R18).
    fn into_rewrite(self) -> Rewrite {
        Rewrite {
            hit: self.hit,
            to: self.to,
        }
    }
}

/// A rewrite's hit, for [`Trail::place`].
fn rewrite_hit(rewrite: &mut Rewrite) -> &mut Hit {
    &mut rewrite.hit
}

/// What a walk reads besides its text: the map, the set's predicate, and
/// how deep a replacement may still be rewritten (V5).
#[derive(Clone, Copy)]
struct Ground<'a> {
    map: &'a Map,
    allowed: &'a dyn Fn(char) -> bool,
    budget: usize,
}

fn run(
    text: &str,
    ground: Ground<'_>,
    hazards: Vec<Hazard>,
) -> Result<Pass, Error> {
    let mut walk = Run {
        text,
        map: ground.map,
        allowed: ground.allowed,
        budget: ground.budget,
        hazards,
        next: 0,
        at: Cursor::start(),
        pass: Pass::default(),
        gap: Gap::default(),
        pairing: Pairing::default(),
    };
    walk.walk()?;
    Ok(walk.pass)
}

/// What a match at one position yields: the source span and the match
/// it settled on, or nothing to rewrite here.
type Found<'a> = Result<Option<(&'a str, Match)>, Error>;

struct Run<'a> {
    text: &'a str,
    map: &'a Map,
    allowed: &'a dyn Fn(char) -> bool,
    budget: usize,
    /// This text's hazards, ascending, and the first one not yet passed.
    hazards: Vec<Hazard>,
    next: usize,
    at: Cursor,
    pass: Pass,
    /// Whether a word just written is kept off what follows
    /// (`src/fix/words:V51`).
    gap: Gap,
    /// Where the walk stands in a run of regional indicators, which pair
    /// from its start (`src/fix/emoji:V76`).
    pairing: Pairing,
}

impl Run<'_> {
    fn walk(&mut self) -> Result<(), Error> {
        while let Some(rest) = self.text.get(self.at.byte..) {
            let Some(ch) = rest.chars().next() else { break };
            let hazard = self.hazard_here();
            let found = self.matched(rest, hazard)?;
            self.pairing.step(found.is_none(), ch);
            match found {
                Some((source, found)) => self.rewrite(source, ch, found)?,
                None => self.keep(ch),
            }
        }
        Ok(())
    }

    /// The hazard at the cursor, if any (V104). The list is ascending and
    /// the cursor only moves forward, so this is one walk of the list.
    fn hazard_here(&mut self) -> Option<Hazard> {
        let byte = self.at.byte;
        while self.hazards.get(self.next).is_some_and(|h| h.byte < byte) {
            self.next = self.next.saturating_add(1);
        }
        self.hazards
            .get(self.next)
            .filter(|h| h.byte == byte)
            .copied()
    }

    /// The span to rewrite here, if any: what the map makes of it, else,
    /// for a hazard that carries no visible text, its deletion (V104).
    fn matched<'b>(&self, rest: &'b str, hazard: Option<Hazard>) -> Found<'b> {
        if self.pairing.closes(rest) {
            return Ok(None);
        }
        if let Some(found) = self.mapped(rest, hazard)? {
            return Ok(Some(found));
        }
        Ok(hazard.filter(|h| h.delete).and_then(|_| deleted(rest)))
    }

    /// What the map makes of the span here, if anything. A zero-length
    /// match is refused: it would leave the cursor where it is. So is a
    /// match whose replacement, rewritten to its fixed point, still holds
    /// a character the file may not: that would WRITE a violation, so the
    /// character stays and is reported (V4, B40).
    ///
    /// A hazard is matched as though its set did not grant it, so a map
    /// entry for it fires where the file allows it too (V104).
    fn mapped<'b>(&self, rest: &'b str, hazard: Option<Hazard>) -> Found<'b> {
        let here = rest.chars().next().filter(|_| hazard.is_some());
        let allowed = |c: char| Some(c) != here && (self.allowed)(c);
        let Some(found) = self.map.resolve_at(rest, &allowed)? else {
            return Ok(None);
        };
        let Some(source) = rest.get(..found.len).filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        let to = self.expand(&found.to)?;
        if violates(&to, self.allowed) {
            return Ok(None);
        }
        Ok(Some((source, Match { to, ..found })))
    }

    fn rewrite(
        &mut self,
        source: &str,
        ch: char,
        found: Match,
    ) -> Result<(), Error> {
        let to = Gap::spaced(&self.pass.output, found.to, found.word);
        let hit = Hit {
            position: self.at.position(),
            character: ch,
        };
        self.emit(&to);
        if found.word {
            self.gap.written(&to);
        }
        for c in source.chars() {
            self.at.advance(c);
        }
        let len = source.len();
        self.pass.spans.push(Span { hit, len, to });
        Ok(())
    }

    /// Write `text`, first closing a pending gap (`src/fix/words:V51`).
    /// The space belongs to the span that left the gap open -- the last
    /// one pushed, since nothing has been written after it -- so the V6
    /// guard still finds every untouched byte where it was.
    fn emit(&mut self, text: &str) {
        if self.gap.close(text) {
            self.pass.output.push(' ');
            if let Some(last) = self.pass.spans.last_mut() {
                last.to.push(' ');
            }
        }
        self.pass.output.push_str(text);
    }

    /// Rewrite the replacement itself, so what is emitted is already the
    /// fixed point of the map (V5). A map that never settles is cyclic, and
    /// a cyclic map is a config error rather than a silent half-rewrite.
    fn expand(&self, to: &str) -> Result<String, Error> {
        let Some(budget) = self.budget.checked_sub(1) else {
            return Err(Error::MapCycle);
        };
        let ground = Ground {
            map: self.map,
            allowed: self.allowed,
            budget,
        };
        Ok(run(to, ground, Vec::new())?.output)
    }

    fn keep(&mut self, ch: char) {
        if !(self.allowed)(ch) {
            self.pass.unmapped.push(Hit {
                position: self.at.position(),
                character: ch,
            });
        }
        self.emit(ch.encode_utf8(&mut [0; 4]));
        self.at.advance(ch);
    }
}

/// The first character of `rest`, deleted: a hazard no map entry covers
/// that carries no visible text (V104).
fn deleted(rest: &str) -> Option<(&str, Match)> {
    let ch = rest.chars().next()?;
    let source = rest.get(..ch.len_utf8())?;
    let found = Match {
        len: source.len(),
        to: String::new(),
        word: false,
    };
    Some((source, found))
}

/// Where the walk is, in the three units `Position` reports.
#[derive(Debug)]
struct Cursor {
    line: usize,
    column: usize,
    byte: usize,
}

impl Cursor {
    fn start() -> Self {
        Self {
            line: 1,
            column: 1,
            byte: 0,
        }
    }

    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
            byte: self.byte,
        }
    }

    fn advance(&mut self, ch: char) {
        self.byte = self.byte.saturating_add(ch.len_utf8());
        if ch == '\n' {
            self.line = self.line.saturating_add(1);
            self.column = 1;
        } else {
            self.column = self.column.saturating_add(1);
        }
    }
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
