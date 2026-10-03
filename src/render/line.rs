//! What the writers read: one violation, BORROWED, and a path spelled
//! once per file rather than once per finding.
//!
//! A report of two million findings is the case this file exists for
//! (R17). Every finding in a file shares its path and its set, so the
//! writers borrow both rather than copy them, and a path's escaped form
//! is worked out when the path CHANGES -- which, in report order, is once
//! per file.

use crate::lint::Finding;
use crate::render::Violation;
use std::borrow::Cow;

/// One violation as a writer reads it: nothing owned, so producing one
/// costs three pointers.
#[derive(Debug, Clone, Copy)]
pub(super) struct Line<'a> {
    pub path: &'a str,
    pub set: &'a str,
    pub finding: &'a Finding,
}

impl<'a> From<&'a Violation<'_>> for Line<'a> {
    fn from(item: &'a Violation<'_>) -> Self {
        Self {
            path: item.path,
            set: item.set,
            finding: &item.finding,
        }
    }
}

/// How one form spells a path: `<U+XXXX>` escapes for the human line, a
/// json literal for the documents, a URI for SARIF.
pub(super) type Spelling = for<'p> fn(&'p str) -> Cow<'p, str>;

/// A path's spelling, kept until the path changes.
pub(super) struct Spelled<'a> {
    spell: Spelling,
    raw: Option<&'a str>,
    said: Cow<'a, str>,
}

impl<'a> Spelled<'a> {
    pub(super) const fn new(spell: Spelling) -> Self {
        Self {
            spell,
            raw: None,
            said: Cow::Borrowed(""),
        }
    }

    /// `raw`, spelled. Recomputed only when it differs from the last.
    pub(super) fn of(&mut self, raw: &'a str) -> &str {
        if self.raw != Some(raw) {
            self.said = (self.spell)(raw);
            self.raw = Some(raw);
        }
        &self.said
    }
}

#[cfg(test)]
mod tests {
    use super::Spelled;
    use std::borrow::Cow;
    use std::cell::Cell;

    thread_local! {
        static CALLS: Cell<usize> = const { Cell::new(0) };
    }

    fn counted(raw: &str) -> Cow<'_, str> {
        CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        Cow::Owned(raw.to_uppercase())
    }

    /// R17: a path is spelled when it changes, not once per finding.
    #[test]
    fn a_path_is_spelled_once_per_run_of_findings() {
        let mut spelled = Spelled::new(counted);
        let said: Vec<String> = ["a", "a", "a", "b", "b", "a"]
            .into_iter()
            .map(|raw| spelled.of(raw).to_owned())
            .collect();
        assert_eq!(said, ["A", "A", "A", "B", "B", "A"]);
        assert_eq!(CALLS.with(Cell::get), 3);
    }
}
