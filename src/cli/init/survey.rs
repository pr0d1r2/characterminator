//! What the tracked files hold, by file type (V128): the characters each
//! type needs granted, the ones `fix` would rewrite anyway, and the
//! hazards, which no line of a `.ctrm` should ever grant.

use crate::fix::Map;
use crate::lint::Hazards;
use crate::scan::scan_str;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// One file type's characters, as the draft explains them.
#[derive(Debug, Default)]
pub(super) struct Kind {
    pub files: usize,
    /// Outside `ascii`, not a hazard, not rewritten by the builtin map.
    pub needed: BTreeSet<char>,
    /// Rewritten to ASCII by the builtin map: no grant needed. One tally
    /// per file, each character with how often it occurs there.
    tallies: Vec<BTreeMap<char, usize>>,
    /// Each hazard code point, its lint, and the first path it was in.
    pub hazards: BTreeMap<char, (&'static str, String)>,
    /// A joiner inside an RGI emoji sequence was seen: `fix` compresses it.
    pub sequences: bool,
}

/// Every file type, keyed by the pattern its `.ctrm` line would use.
pub(super) struct Survey {
    pub kinds: BTreeMap<String, Kind>,
    pub skipped: usize,
    map: HashSet<char>,
    /// `--keep-fixable`: what `fix` rewrites is granted instead.
    keep: bool,
    hazards: Hazards,
}

impl Survey {
    pub(super) fn new(keep: bool) -> Self {
        Self {
            kinds: BTreeMap::new(),
            skipped: 0,
            keep,
            map: to_ascii(&Map::builtin()),
            hazards: Hazards::builtin(),
        }
    }

    /// One file's text, filed under its type.
    pub(super) fn add(&mut self, shown: &str, text: &str) {
        let exempt = self.hazards.exempt(text);
        let kind = self.kinds.entry(pattern_of(shown)).or_default();
        kind.begin();
        for hit in scan_str(text, plain) {
            let c = hit.character;
            if let Some(lint) = self.hazards.lint_at(hit, &exempt) {
                let first = (lint.name, shown.to_owned());
                kind.hazards.entry(c).or_insert(first);
            } else if exempt.binary_search(&hit.position.byte).is_ok() {
                kind.sequences = true;
            } else {
                kind.file(c, !self.keep && self.map.contains(&c));
            }
        }
    }
}

impl Kind {
    /// A file of this type is read: its own tally of what `fix` rewrites.
    fn begin(&mut self) {
        self.files = self.files.saturating_add(1);
        self.tallies.push(BTreeMap::new());
    }

    /// A character that is no hazard: rewritten by `fix`, or needed.
    fn file(&mut self, c: char, mapped: bool) {
        match self.tallies.last_mut() {
            Some(tally) if mapped => {
                let seen = tally.entry(c).or_insert(0);
                *seen = seen.saturating_add(1);
            }
            _ => {
                self.needed.insert(c);
            }
        }
    }

    /// Every character the builtin map would rewrite, in any file.
    pub(super) fn fixable(&self) -> BTreeSet<char> {
        self.tallies
            .iter()
            .flat_map(|tally| tally.keys().copied())
            .collect()
    }

    /// What `check` reports until `fix` runs, findings and files, on the
    /// characters the drafted line does not grant: a granted one is no
    /// finding, and `fix` leaves it alone (`src/fix:V6`).
    pub(super) fn unfixed(
        &self,
        granted: &dyn Fn(char) -> bool,
    ) -> (usize, usize) {
        let each = self.tallies.iter().map(|tally| {
            let left = tally.iter().filter(|(c, _)| !granted(**c));
            left.fold(0_usize, |sum, (_, n)| sum.saturating_add(*n))
        });
        each.filter(|n| *n > 0).fold((0, 0), |(all, files), n| {
            (all.saturating_add(n), files.saturating_add(1))
        })
    }
}

/// The single code points the builtin map rewrites to ASCII (`src/fix:V26`).
fn to_ascii(map: &Map) -> HashSet<char> {
    let each = map.entries().iter().filter_map(|entry| {
        let mut chars = entry.from.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return None;
        };
        entry.to.is_ascii().then_some(c)
    });
    each.collect()
}

/// What `ascii` grants (`src/rules:V21`).
fn plain(c: char) -> bool {
    c == '\t' || c == '\n' || (' '..='~').contains(&c)
}

/// `*.<ext>` for a file with an extension, else its own name: a pattern
/// with no `/` matches at any depth (`src/rules:V2`).
pub(super) fn pattern_of(shown: &str) -> String {
    let name = shown.rsplit('/').next().unwrap_or(shown);
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => {
            format!("*.{ext}")
        }
        _ => name.to_owned(),
    }
}
