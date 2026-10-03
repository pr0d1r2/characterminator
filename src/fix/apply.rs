//! Applying a map: the `fix` pass and the `--check` reporting mode.
//!
//! One pass walks the text and rewrites a span ONLY when the span covers a
//! character the caller's predicate disallows, so every other byte is copied
//! through untouched (V6). A character with no mapping is copied through and
//! reported rather than dropped (V4). A replacement is itself run through
//! the map before it is emitted, so a chain (skin tone to thumbs up to `+1`)
//! lands on its fixed point in one pass and a second run changes nothing
//! (V5) -- unless a rewrite revealed a sequence the scan had passed, and
//! then the passes repeat until one changes nothing (V65).
//!
//! Whether a character is ALLOWED is not decided here. It arrives as a
//! predicate, because that answer belongs to the charset and rules nodes.

use crate::fix::map::{Map, Match};
use crate::fix::{Error, Rewrite};
use crate::scan::{Hit, Position, located};

/// What `fix` found, with no text to write: the `fix --check` answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
}

/// Rewrite every disallowed span of `text` that the map covers.
///
/// The V6 and V5 guards run BEFORE this returns, so a caller that writes
/// what it gets back cannot write a file that failed either.
///
/// Passes repeat until one changes nothing (V65), each guarded by V6 on
/// its own input. Every row is reported in the ORIGINAL text's
/// coordinates: a later pass read text an earlier one rewrote, so its
/// positions are mapped back through each earlier pass (B24). A text that
/// settles in one pass -- every text before the sequence map -- maps
/// through nothing and is reported exactly as before.
pub fn fix(
    text: &str,
    map: &Map,
    allowed: &dyn Fn(char) -> bool,
) -> Result<Fixed, Error> {
    let mut seen = History::new(text);
    let mut pass = guarded(text, map, allowed)?;
    for _ in 0..SETTLE {
        let again = guarded(&pass.output, map, allowed)?;
        if again.output == pass.output {
            return Ok(seen.settled(pass));
        }
        seen.record(pass);
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
    layers: Vec<Layer>,
}

impl<'t> History<'t> {
    const fn new(original: &'t str) -> Self {
        Self {
            original,
            report: Report {
                rewrites: Vec::new(),
                unmapped: Vec::new(),
            },
            layers: Vec::new(),
        }
    }

    /// A pass that changed its input: its rewrites join the report, and
    /// its spans become the layer later positions are mapped through.
    fn record(&mut self, pass: Pass) {
        let hits: Vec<Hit> = pass.spans.iter().map(|span| span.hit).collect();
        let found = self.origins(&hits);
        let rewrites = pass.spans.iter().zip(found);
        let rows = rewrites.map(|(span, hit)| span.rewrite_at(hit));
        self.report.rewrites.extend(rows.collect::<Vec<_>>());
        self.layers.push(Layer::of(&pass.spans));
    }

    /// The pass whose output is the fixed point. Its unmapped characters
    /// are the ones left, located where they sit in the original.
    fn settled(mut self, pass: Pass) -> Fixed {
        self.report.unmapped = self.origins(&pass.unmapped);
        let output = pass.output.clone();
        self.record(pass);
        self.report.rewrites.sort_by_key(|r| r.hit.position.byte);
        Fixed {
            output,
            report: self.report,
        }
    }

    /// Where each hit read by the NEXT pass sits in the original: back
    /// through every recorded layer, newest first, then re-located in the
    /// original so line and column agree with the byte.
    ///
    /// ONE forward walk of the original for all of them, in byte order:
    /// a walk per hit made a large file with a second pass quadratic.
    fn origins(&self, hits: &[Hit]) -> Vec<Hit> {
        let mut out = hits.to_vec();
        if self.layers.is_empty() {
            return out;
        }
        let mut order: Vec<(usize, usize)> = hits
            .iter()
            .enumerate()
            .map(|(at, hit)| (self.back(hit.position.byte), at))
            .collect();
        order.sort_unstable();
        relocate(self.original, &order, &mut out);
        out
    }

    /// A byte of the newest pass's output, as a byte of the original.
    fn back(&self, byte: usize) -> usize {
        self.layers
            .iter()
            .rev()
            .fold(byte, |at, layer| layer.back(at))
    }
}

/// Give each `(byte, index)` -- sorted by byte -- the position that byte
/// has in `original`, in one walk. A byte that starts no character keeps
/// the position it came with.
fn relocate(original: &str, order: &[(usize, usize)], out: &mut [Hit]) {
    let mut walk = located(original).peekable();
    for &(byte, at) in order {
        while walk.next_if(|seen| seen.position.byte < byte).is_some() {}
        let here = walk.peek().filter(|seen| seen.position.byte == byte);
        if let (Some(slot), Some(seen)) = (out.get_mut(at), here) {
            slot.position = seen.position;
        }
    }
}

/// One pass's spans, each with where its replacement sits in that pass's
/// output, so mapping a byte back is a binary search, not a walk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Layer {
    marks: Vec<Mark>,
}

/// One span: where it started and ended in the input, and where its
/// replacement starts and ends in the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mark {
    input: usize,
    input_end: usize,
    output: usize,
    output_end: usize,
}

impl Layer {
    fn of(spans: &[Span]) -> Self {
        let mut cut = Cut::default();
        let mark = |span: &Span| {
            let output = cut.start_of(span);
            cut.skip(span);
            Mark {
                input: span.hit.position.byte,
                input_end: cut.input,
                output,
                output_end: cut.output,
            }
        };
        Self {
            marks: spans.iter().map(mark).collect(),
        }
    }

    /// A byte of this pass's OUTPUT, as a byte of its input. Outside every
    /// span, the offset shifts by what the spans before it changed; in a
    /// span's replacement, it is that span's start, the nearest place in
    /// the input the text came from.
    fn back(&self, byte: usize) -> usize {
        let after = self.marks.partition_point(|mark| mark.output <= byte);
        let last = after.checked_sub(1).and_then(|at| self.marks.get(at));
        let Some(mark) = last else {
            return byte;
        };
        if byte < mark.output_end {
            return mark.input;
        }
        mark.input_end
            .saturating_add(byte.saturating_sub(mark.output_end))
    }
}

/// How many passes past the first `fix` may take to settle (V65). One
/// rewrite can REVEAL a sequence: deleting a stray VS16 or skin tone out
/// of `woman U+FE0F ZWJ laptop` leaves the RGI `woman ZWJ laptop`, which
/// the scan had already walked past (B14). Each later pass only ever
/// shortens what the last one revealed, so two settle anything the
/// builtin map can produce; the bound is what turns a map that never
/// settles into a refusal rather than a loop.
const SETTLE: usize = 3;

/// One pass, refused if it touched a byte outside a violation (V6).
fn guarded(
    text: &str,
    map: &Map,
    allowed: &dyn Fn(char) -> bool,
) -> Result<Pass, Error> {
    let pass = run(text, map, allowed, map.budget())?;
    if untouched_bytes_match(text, &pass) {
        Ok(pass)
    } else {
        Err(Error::TouchedAllowedBytes)
    }
}

/// Report what `fix` would do and hand back nothing to write (`--check`).
pub fn check(
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
    /// This span as the public row, at `hit` rather than its own.
    fn rewrite_at(&self, hit: Hit) -> Rewrite {
        Rewrite {
            hit,
            to: self.to.clone(),
        }
    }
}

fn run<'a>(
    text: &'a str,
    map: &'a Map,
    allowed: &'a dyn Fn(char) -> bool,
    budget: usize,
) -> Result<Pass, Error> {
    let mut walk = Run {
        text,
        map,
        allowed,
        budget,
        at: Cursor::start(),
        pass: Pass::default(),
        gap: false,
        paired: false,
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
    at: Cursor,
    pass: Pass,
    /// The last thing written was a word ending in a letter or a digit, so
    /// the next thing to open with one is kept apart from it (V51).
    gap: bool,
    /// The last character kept opened a pair of regional indicators, so
    /// the next one, if it is one, closes that pair and starts no flag of
    /// its own (V76): a run pairs from its start, as UAX #29 reads it.
    paired: bool,
}

impl Run<'_> {
    fn walk(&mut self) -> Result<(), Error> {
        while let Some(rest) = self.text.get(self.at.byte..) {
            let Some(ch) = rest.chars().next() else { break };
            let found = self.matched(rest)?;
            self.paired = found.is_none() && !self.paired && regional(ch);
            match found {
                Some((source, found)) => self.rewrite(source, ch, found)?,
                None => self.keep(ch),
            }
        }
        Ok(())
    }

    /// The span to rewrite here, if any. A zero-length match is refused: it
    /// would leave the cursor where it is.
    fn matched<'b>(&self, rest: &'b str) -> Found<'b> {
        if self.paired && rest.chars().next().is_some_and(regional) {
            return Ok(None);
        }
        let Some(found) = self.map.resolve_at(rest, self.allowed)? else {
            return Ok(None);
        };
        let Some(source) = rest.get(..found.len).filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        Ok(Some((source, found)))
    }

    fn rewrite(
        &mut self,
        source: &str,
        ch: char,
        found: Match,
    ) -> Result<(), Error> {
        let to = self.spaced(self.expand(&found.to)?, found.word);
        let hit = Hit {
            position: self.at.position(),
            character: ch,
        };
        self.emit(&to);
        if found.word {
            self.gap = to.chars().next_back().is_some_and(joins);
        }
        for c in source.chars() {
            self.at.advance(c);
        }
        let len = source.len();
        self.pass.spans.push(Span { hit, len, to });
        Ok(())
    }

    /// A word that would land against a letter or a digit already written
    /// gets a space in front (V51). The space is part of the REPLACEMENT,
    /// so the V6 guard counts it inside this span and never as a changed
    /// byte of the text around it.
    fn spaced(&self, to: String, word: bool) -> String {
        let after = self.pass.output.chars().next_back().is_some_and(joins);
        if word && after && to.chars().next().is_some_and(joins) {
            return format!(" {to}");
        }
        to
    }

    /// Write `text`, first closing a pending gap (V51): the word before it
    /// ended in a letter or a digit, and `text` opens with one. The space
    /// belongs to the span that left the gap open -- the last one pushed,
    /// since nothing has been written after it -- so the V6 guard still
    /// finds every untouched byte where it was.
    ///
    /// Writing nothing keeps the gap open: a delete between a word and a
    /// letter must not fuse them either.
    fn emit(&mut self, text: &str) {
        if self.gap && text.chars().next().is_some_and(joins) {
            self.pass.output.push(' ');
            if let Some(last) = self.pass.spans.last_mut() {
                last.to.push(' ');
            }
        }
        self.gap = self.gap && text.is_empty();
        self.pass.output.push_str(text);
    }

    /// Rewrite the replacement itself, so what is emitted is already the
    /// fixed point of the map (V5). A map that never settles is cyclic, and
    /// a cyclic map is a config error rather than a silent half-rewrite.
    fn expand(&self, to: &str) -> Result<String, Error> {
        let Some(budget) = self.budget.checked_sub(1) else {
            return Err(Error::MapCycle);
        };
        Ok(run(to, self.map, self.allowed, budget)?.output)
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

/// Whether a character on one side of a word boundary would read as part
/// of ONE word with its neighbour: a letter, a digit or the underscore,
/// the `\w` a reader's `grep '\bnot\b'` uses -- in any script, since a
/// symbol before a Polish word fuses as badly as before an English one.
fn joins(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// A regional indicator, U+1F1E6 to U+1F1FF: half of a flag (V76).
fn regional(ch: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&ch)
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

    /// Where `span`'s replacement begins in the output.
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
