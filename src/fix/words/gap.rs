//! Keeping a word off its neighbours (V51): a `word` entry's replacement
//! that would land against a letter or a digit gets a space between the
//! two, so U+22A5 before `owns` becomes `not owns` rather than `notowns`.
//!
//! The space is part of the REPLACEMENT, never a changed byte of the text
//! around it, so the engine's `src/fix:V6` guard still finds every
//! untouched byte where it was.

/// Whether the last thing written was a word ending in a letter or a
/// digit, so the next thing to open with one is kept apart from it.
#[derive(Debug, Default, Clone, Copy)]
pub(in crate::fix) struct Gap {
    open: bool,
}

impl Gap {
    /// A word that would land against a letter or a digit already
    /// `written` gets a space in front.
    pub(in crate::fix) fn spaced(
        written: &str,
        to: String,
        word: bool,
    ) -> String {
        let after = written.chars().next_back().is_some_and(joins);
        if word && after && to.chars().next().is_some_and(joins) {
            return format!(" {to}");
        }
        to
    }

    /// A word `to` was just written: the gap opens if it ends in a letter
    /// or a digit.
    pub(in crate::fix) fn written(&mut self, to: &str) {
        self.open = to.chars().next_back().is_some_and(joins);
    }

    /// `text` is about to be written: whether a space must go first, to
    /// close a gap a word left open. The space belongs to the span that
    /// left it, which the caller extends.
    ///
    /// Writing nothing keeps the gap open: a delete between a word and a
    /// letter must not fuse them either.
    pub(in crate::fix) fn close(&mut self, text: &str) -> bool {
        let space = self.open && text.chars().next().is_some_and(joins);
        self.open = self.open && text.is_empty();
        space
    }
}

/// Whether a character on one side of a word boundary would read as part
/// of ONE word with its neighbour: a letter, a digit or the underscore,
/// the `\w` a reader's `grep '\bnot\b'` uses -- in any script, since a
/// symbol before a Polish word fuses as badly as before an English one.
fn joins(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}
