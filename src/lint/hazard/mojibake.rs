//! `double-encoded` (`src/lint/hazard:V142`): UTF-8 that was read as Latin-1
//! and saved as UTF-8 again, found as RUNS rather than as characters.
//!
//! A run is a lead U+00C2-U+00F4, then exactly as many U+0080-U+00BF as the
//! lead promises, that re-encode as Latin-1 into one valid UTF-8 character,
//! with at least one C1 control (U+0080-U+009F) among the continuations. The
//! C1 control is what makes the reading unambiguous: a real accented letter
//! is never followed by one, so e-acute (U+00E9) alone, or U+00C3 U+00A9 in a file
//! that really is Latin-1 prose, is no run.
//!
//! A finding points at the run's FIRST character and carries the character
//! the run decodes to, which is not in the file: the run is the damage, and
//! the character is what it was.

use crate::lint::{Group, Lint};
use crate::scan::{Hit, Position, located};

/// A run of code points that is a UTF-8 character read as Latin-1.
pub(crate) const DOUBLE_ENCODED: Lint =
    Lint::new("double-encoded", Group::Hazard);

/// One run: where it starts, how many bytes of the damaged text it covers,
/// and the character it decodes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Run {
    pub at: Position,
    pub len: usize,
    pub decoded: char,
}

impl Run {
    /// The finding's hit: the run's start, naming what it decodes to.
    pub(crate) const fn hit(&self) -> Hit {
        Hit {
            position: self.at,
            character: self.decoded,
        }
    }

    /// Whether the byte offset is inside the damaged text.
    pub(crate) const fn covers(&self, byte: usize) -> bool {
        match byte.checked_sub(self.at.byte) {
            Some(into) => into < self.len,
            None => false,
        }
    }
}

/// Whether `byte` lies inside any of `runs`, which are ascending.
pub(crate) fn inside(runs: &[Run], byte: usize) -> bool {
    let after = runs.partition_point(|run| run.at.byte <= byte);
    after
        .checked_sub(1)
        .and_then(|last| runs.get(last))
        .is_some_and(|run| run.covers(byte))
}

/// The continuation count a lead promises, if it is a lead at all.
const fn promised(lead: char) -> Option<usize> {
    match lead {
        '\u{C2}'..='\u{DF}' => Some(1),
        '\u{E0}'..='\u{EF}' => Some(2),
        '\u{F0}'..='\u{F4}' => Some(3),
        _ => None,
    }
}

const fn continuation(c: char) -> bool {
    matches!(c, '\u{80}'..='\u{BF}')
}

const fn control(c: char) -> bool {
    matches!(c, '\u{80}'..='\u{9F}')
}

/// A lead and the continuations read since, while a run is being walked.
struct Open {
    lead: Hit,
    promised: usize,
    tail: Vec<char>,
}

impl Open {
    fn of(hit: Hit) -> Option<Self> {
        let promised = promised(hit.character)?;
        Some(Self {
            lead: hit,
            promised,
            tail: Vec::new(),
        })
    }

    /// The run, if the characters read so far are one.
    fn closed(&self) -> Option<Run> {
        let complete = self.tail.len() == self.promised;
        let signed = self.tail.iter().any(|c| control(*c));
        let decoded = if complete && signed {
            self.decode()
        } else {
            None
        };
        Some(Run {
            at: self.lead.position,
            len: self.tail.len().saturating_add(1).saturating_mul(2),
            decoded: decoded?,
        })
    }

    /// The one character the Latin-1 bytes of the run are, if they are UTF-8.
    fn decode(&self) -> Option<char> {
        let latin1 = std::iter::once(self.lead.character)
            .chain(self.tail.iter().copied());
        let bytes: Vec<u8> = latin1
            .filter_map(|c| u8::try_from(u32::from(c)).ok())
            .collect();
        let mut chars = std::str::from_utf8(&bytes).ok()?.chars();
        let first = chars.next()?;
        chars.next().is_none().then_some(first)
    }
}

/// Every run in a text, ascending. Text without a C1 control holds none, so
/// the walk is skipped for it.
pub(crate) fn runs(text: &str) -> Vec<Run> {
    if text.is_ascii() || !text.chars().any(control) {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut open = None;
    for hit in located(text) {
        open = step(open, hit, &mut found);
    }
    found
}

/// One character: it extends the run being read, or starts one. A run that
/// has read all it promised is a find or nothing, and is closed either way.
fn step(open: Option<Open>, hit: Hit, found: &mut Vec<Run>) -> Option<Open> {
    match open {
        Some(mut run) if continuation(hit.character) => {
            run.tail.push(hit.character);
            if run.tail.len() < run.promised {
                return Some(run);
            }
            found.extend(run.closed());
            None
        }
        _ => Open::of(hit),
    }
}

#[cfg(test)]
mod tests {
    use super::{DOUBLE_ENCODED, inside, runs};
    use crate::lint::Lint;

    /// One run as `(line, column, byte, length, decoded character)`.
    type Seen = (usize, usize, usize, usize, char);

    /// What each run says.
    fn seen(text: &str) -> Vec<Seen> {
        runs(text)
            .iter()
            .map(|r| (r.at.line, r.at.column, r.at.byte, r.len, r.decoded))
            .collect()
    }

    #[test]
    fn the_lint_is_registered_as_a_hazard() {
        assert_eq!(Lint::named("double-encoded"), Some(DOUBLE_ENCODED));
    }

    #[test]
    fn the_issues_example_is_two_runs() {
        let text = "two runs must \u{e2}\u{8a}\u{a5} share a dir \
                    \u{e2}\u{80}\u{94} ok\n";
        let expected =
            vec![(1, 15, 14, 6, '\u{22A5}'), (1, 31, 33, 6, '\u{2014}')];
        assert_eq!(seen(text), expected);
    }

    #[test]
    fn a_two_byte_and_a_four_byte_character_are_runs() {
        assert_eq!(seen("\u{c2}\u{80}"), vec![(1, 1, 0, 4, '\u{80}')]);
        let pile = "x\u{f0}\u{9f}\u{91}\u{8d}";
        assert_eq!(seen(pile), vec![(1, 2, 1, 8, '\u{1F44D}')]);
    }

    #[test]
    fn an_accented_letter_is_no_run() {
        assert_eq!(seen("caf\u{e9}"), vec![]);
        assert_eq!(seen("\u{c3}\u{a9}"), vec![]);
        assert_eq!(seen("\u{c3}\u{a9} and \u{85}"), vec![]);
    }

    #[test]
    fn a_short_or_invalid_run_is_none() {
        assert_eq!(seen("\u{e2}\u{80}"), vec![]);
        assert_eq!(seen("\u{e2}\u{80}x"), vec![]);
        // overlong: C0 is no lead; E0 80 80 is not valid UTF-8
        assert_eq!(seen("\u{e0}\u{80}\u{80}"), vec![]);
    }

    #[test]
    fn a_stray_lead_does_not_hide_the_run_after_it() {
        assert_eq!(
            seen("\u{e2}\u{e2}\u{80}\u{94}"),
            vec![(1, 2, 2, 6, '\u{2014}')]
        );
    }

    #[test]
    fn inside_asks_the_damaged_bytes_only() {
        let found = runs("a\u{e2}\u{80}\u{94}b");
        assert!(!inside(&found, 0));
        assert!(inside(&found, 1) && inside(&found, 6));
        assert!(!inside(&found, 7));
    }
}
