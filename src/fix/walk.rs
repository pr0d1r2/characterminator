//! One pass of `fix`: the walk over one text.
//!
//! The walk rewrites a span ONLY when the span covers a character the
//! caller's predicate disallows, or starts at a hazard the caller's law
//! names (V104), and copies every other character through. It asks the
//! map what a span becomes, the `words` node whether a word needs a space
//! (`src/fix/words:V51`), and the `emoji` node whether a flag may start
//! here (`src/fix/emoji:V76`). What it produced is guarded and folded
//! into a report by `apply`, which is the only caller.

use crate::fix::emoji::{Extent, Pairing};
use crate::fix::map::{Map, Match, violates};
use crate::fix::words::Gap;
use crate::fix::{Error, Hazard, Rewrite};
use crate::scan::{Hit, Position};

/// One walk over one text.
#[derive(Debug, Default)]
pub(super) struct Pass {
    pub(super) output: String,
    pub(super) spans: Vec<Span>,
    pub(super) unmapped: Vec<Hit>,
}

/// A rewritten span. The public `Rewrite` names the first character; the
/// byte length stays here, because it is what the V6 guard needs.
#[derive(Debug)]
pub(super) struct Span {
    pub(super) hit: Hit,
    pub(super) len: usize,
    pub(super) to: String,
}

impl Span {
    /// What the trail needs of this span (`src/fix/emoji:V65`).
    pub(super) fn extent(&self) -> Extent {
        Extent {
            byte: self.hit.position.byte,
            len: self.len,
            to: self.to.len(),
        }
    }

    /// This span as the public row, its replacement MOVED rather than
    /// copied: a pass's spans are spent once its layer is built (R18).
    pub(super) fn into_rewrite(self) -> Rewrite {
        Rewrite {
            hit: self.hit,
            to: self.to,
        }
    }
}

/// What a walk reads besides its text: the map, the set's predicate, and
/// how deep a replacement may still be rewritten (V5).
#[derive(Clone, Copy)]
pub(super) struct Ground<'a> {
    pub(super) map: &'a Map,
    pub(super) allowed: &'a dyn Fn(char) -> bool,
    pub(super) budget: usize,
}

pub(super) fn run(
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
