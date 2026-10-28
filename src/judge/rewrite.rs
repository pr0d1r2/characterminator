//! A judge's rewrite: `fix` under one file's law (`src/fix:V104`).
//!
//! A child of `checker`, so it reads the judge's own hazard verdict --
//! the one `check` reports -- rather than a second definition of it.

use super::Judge;
use crate::fix::{
    Error as FixError, Fixed, Hazard, Law, Map, Restore, fix_under,
};
use crate::lint::{Levels, carries_no_text, double_encoded};
use crate::scan::{Hit, scan_str};

impl Judge<'_> {
    /// `fix` over `text`, under this file's law: what the set grants, and
    /// the hazards `check` would fire here -- the V57 excuse, the V63
    /// sequence exemption and the byte-0 signature all applied, because
    /// they are THIS judge's -- rewritten whatever the set grants. A hazard
    /// the path is GRANTED by name (`hazard:tags`, `src/rules:V139`) is
    /// left as it is: deleting the artefact the grant exists to keep would
    /// defeat it. `fix` and `stats` both rewrite through this, so the text
    /// one writes is the text the other counts.
    pub(crate) fn fix(
        &self,
        text: &str,
        map: &Map,
        levels: &Levels,
    ) -> Result<Fixed, FixError> {
        let allowed = |c: char| self.set.contains(c) || self.granted(c, levels);
        let hazards = |text: &str| self.hazards_in(text, levels);
        let kept = |text: &str| self.selectors(text);
        let restores = |text: &str| self.restores(text);
        let law = Law {
            allowed: &allowed,
            hazards: &hazards,
            kept: &kept,
            restores: &restores,
        };
        fix_under(text, map, law)
    }

    /// The offsets of the selectors this file's set lets stand after their
    /// base character (`src/lint/hazard:V63`).
    pub(super) fn selectors(&self, text: &str) -> Vec<usize> {
        self.hazards.selectors(text, |c| self.set.contains(c))
    }

    /// Whether the path is granted the hazard class `character` is in.
    fn granted(&self, character: char, levels: &Levels) -> bool {
        let class = self.hazards.class_of(character);
        class.is_some_and(|lint| levels.is_granted(lint))
    }

    /// Every hazard in `text` as `check` judges it, ascending, each with
    /// whether `fix` may delete it (`src/lint/hazard:V49` classes), less
    /// the ones the path is granted.
    fn hazards_in(&self, text: &str, levels: &Levels) -> Vec<Hazard> {
        let hits = scan_str(text, |c| !self.hazard(c));
        if hits.is_empty() {
            return Vec::new();
        }
        let mut exempt = self.hazards.exempt(text);
        exempt.extend(self.selectors(text));
        exempt.sort_unstable();
        let hazard = |hit: Hit| {
            let lint = self.hazards.lint_at(hit, &exempt)?;
            (!levels.is_granted(lint)).then_some(Hazard {
                byte: hit.position.byte,
                delete: carries_no_text(lint),
            })
        };
        hits.into_iter().filter_map(hazard).collect()
    }
}

impl Judge<'_> {
    /// The double-encoded runs of `text` as `check` reports them
    /// (`src/lint/hazard:V142`): `fix` re-decodes each, whatever the set
    /// grants, then judges what it restored like any other character.
    fn restores(&self, text: &str) -> Vec<Restore> {
        let restore = |run: crate::lint::Damage| Restore {
            byte: run.at.byte,
            len: run.len,
            to: run.decoded,
        };
        double_encoded(text).into_iter().map(restore).collect()
    }
}
