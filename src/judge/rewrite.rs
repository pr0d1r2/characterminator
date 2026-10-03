//! A judge's rewrite: `fix` under one file's law (`src/fix:V104`).
//!
//! A child of `checker`, so it reads the judge's own hazard verdict --
//! the one `check` reports -- rather than a second definition of it.

use super::Judge;
use crate::fix::{Error as FixError, Fixed, Hazard, Law, Map, fix_under};
use crate::lint::carries_no_text;
use crate::scan::{Hit, scan_str};

impl Judge<'_> {
    /// `fix` over `text`, under this file's law: what the set grants, and
    /// the hazards `check` would fire here -- the V57 excuse, the V63
    /// sequence exemption and the byte-0 signature all applied, because
    /// they are THIS judge's -- rewritten whatever the set grants. `fix`
    /// and `stats` both rewrite through this, so the text one writes is
    /// the text the other counts.
    pub(crate) fn fix(&self, text: &str, map: &Map) -> Result<Fixed, FixError> {
        let allowed = |c: char| self.set.contains(c);
        let hazards = |text: &str| self.hazards_in(text);
        let law = Law {
            allowed: &allowed,
            hazards: &hazards,
        };
        fix_under(text, map, law)
    }

    /// Every hazard in `text` as `check` judges it, ascending, each with
    /// whether `fix` may delete it (`src/lint/hazard:V49` classes).
    fn hazards_in(&self, text: &str) -> Vec<Hazard> {
        let hits = scan_str(text, |c| !self.hazard(c));
        if hits.is_empty() {
            return Vec::new();
        }
        let exempt = self.hazards.exempt(text);
        let hazard = |hit: Hit| {
            let lint = self.hazards.lint_at(hit, &exempt)?;
            Some(Hazard {
                byte: hit.position.byte,
                delete: carries_no_text(lint),
            })
        };
        hits.into_iter().filter_map(hazard).collect()
    }
}
