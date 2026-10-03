//! Flag pairing (V76): regional indicators pair from the START of each
//! run, as UAX #29 reads them, so the second half of one pair never opens
//! a flag with the indicator after it (B35).

/// Whether the last character the walk kept opened a pair of regional
/// indicators, so the next one, if it is one, closes that pair and starts
/// no flag of its own.
#[derive(Debug, Default, Clone, Copy)]
pub(in crate::fix) struct Pairing {
    paired: bool,
}

impl Pairing {
    /// `rest` opens with the second half of a pair already started: no
    /// flag may be matched here.
    pub(in crate::fix) fn closes(self, rest: &str) -> bool {
        self.paired && rest.chars().next().is_some_and(regional)
    }

    /// The walk has decided at `ch`: a regional indicator it KEPT opens a
    /// pair, unless it was the one closing the pair before it.
    pub(in crate::fix) fn step(&mut self, kept: bool, ch: char) {
        self.paired = kept && !self.paired && regional(ch);
    }
}

/// A regional indicator, U+1F1E6 to U+1F1FF: half of a flag (V76).
fn regional(ch: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&ch)
}
