//! The multi-pass fix a sequence needs (V65): how many passes past the
//! first it may take, and the trail that maps a later pass's positions
//! back to the original text (B24).
//!
//! The engine in `src/fix` runs the passes; this module only remembers
//! where each one moved the text. It sees a pass as a list of
//! [`Extent`]s, so nothing here reaches into the engine's own types.

use crate::scan::{Hit, located};

/// How many passes past the first `fix` may take to settle (V65). One
/// rewrite can REVEAL a sequence: deleting a stray VS16 or skin tone out
/// of `woman U+FE0F ZWJ laptop` leaves the RGI `woman ZWJ laptop`, which
/// the scan had already walked past (B14). Each later pass only ever
/// shortens what the last one revealed, so two settle anything the
/// builtin map can produce; the bound is what turns a map that never
/// settles into a refusal rather than a loop.
pub(in crate::fix) const SETTLE: usize = 3;

/// One rewritten span of a pass, as the trail needs it: where it starts
/// in the pass's input, how many bytes it covered, and how many bytes
/// its replacement wrote.
#[derive(Debug, Clone, Copy)]
pub(in crate::fix) struct Extent {
    pub(in crate::fix) byte: usize,
    pub(in crate::fix) len: usize,
    pub(in crate::fix) to: usize,
}

/// The layers of the passes that changed their input, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::fix) struct Trail {
    layers: Vec<Layer>,
}

impl Trail {
    /// No layers yet: a text that settles in one pass maps through nothing.
    pub(in crate::fix) const fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add the layer of the newest pass that changed its input.
    pub(in crate::fix) fn push(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Where each hit read AFTER these layers sits in `original`: back
    /// through every layer, newest first, then re-located in the original
    /// so line and column agree with the byte.
    ///
    /// ONE forward walk of the original for all of them, in byte order:
    /// a walk per hit made a large file with a second pass quadratic.
    pub(in crate::fix) fn origins(
        &self,
        original: &str,
        hits: &[Hit],
    ) -> Vec<Hit> {
        let mut out = hits.to_vec();
        self.place(original, &mut out, itself);
        out
    }

    /// [`Trail::origins`] IN PLACE, over anything that holds a hit: a
    /// caller with millions of rows moves each one's hit where it sits
    /// rather than holding a second list of them (`src/fix:R18`).
    pub(in crate::fix) fn place<T>(
        &self,
        original: &str,
        items: &mut [T],
        hit: HitOf<T>,
    ) {
        if self.layers.is_empty() {
            return;
        }
        let mut order: Vec<(usize, usize)> = items
            .iter_mut()
            .enumerate()
            .map(|(at, item)| (self.back(hit(item).position.byte), at))
            .collect();
        order.sort_unstable();
        relocate(original, &order, items, hit);
    }

    /// A byte of the newest layer's output, as a byte of the original.
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
fn relocate<T>(
    original: &str,
    order: &[(usize, usize)],
    out: &mut [T],
    hit: HitOf<T>,
) {
    let mut walk = located(original).peekable();
    for &(byte, at) in order {
        while walk.next_if(|seen| seen.position.byte < byte).is_some() {}
        let here = walk.peek().filter(|seen| seen.position.byte == byte);
        if let (Some(slot), Some(seen)) = (out.get_mut(at), here) {
            hit(slot).position = seen.position;
        }
    }
}

/// How [`Trail::place`] reaches the hit inside a row.
pub(in crate::fix) type HitOf<T> = fn(&mut T) -> &mut Hit;

/// A hit, for [`Trail::place`].
pub(in crate::fix) const fn itself(hit: &mut Hit) -> &mut Hit {
    hit
}

/// One pass's spans, each with where its replacement sits in that pass's
/// output, so mapping a byte back is a binary search, not a walk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::fix) struct Layer {
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
    /// The layer of one pass, from its spans in the order it wrote them.
    pub(in crate::fix) fn of(spans: impl Iterator<Item = Extent>) -> Self {
        let mut read = Reading::default();
        Self {
            marks: spans.map(|span| read.mark(span)).collect(),
        }
    }

    /// A byte of this pass's OUTPUT, as a byte of its input. Outside every
    /// span, the offset shifts by what the spans before it changed; in a
    /// span's replacement, it is that span's start, the nearest place in
    /// the input the text came from.
    pub(in crate::fix) fn back(&self, byte: usize) -> usize {
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

/// How far [`Layer::of`] has read, in the pass's input and its output.
#[derive(Debug, Default)]
struct Reading {
    input: usize,
    output: usize,
}

impl Reading {
    /// The mark of the next span, and the reading moved past it.
    fn mark(&mut self, span: Extent) -> Mark {
        let gap = span.byte.saturating_sub(self.input);
        let start = self.output.saturating_add(gap);
        self.input = span.byte.saturating_add(span.len);
        self.output = start.saturating_add(span.to);
        Mark {
            input: span.byte,
            input_end: self.input,
            output: start,
            output_end: self.output,
        }
    }
}
