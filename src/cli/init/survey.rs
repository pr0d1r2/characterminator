//! What the tracked files hold, by file type (V128): the characters each
//! type needs granted, the ones `fix` would rewrite anyway, and the
//! hazards, which no line of a `.ctrm` should ever grant.

use crate::fix::Map;
use crate::lint::{
    DOUBLE_ENCODED, Damage as Run, Hazards, double_encoded, inside,
};
use crate::scan::{Hit, Position, scan_str};
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
    /// Each hazard code point, every lint that names it (comma-joined), and
    /// the `path:line:col` places it was found at.
    pub hazards: BTreeMap<char, Hazard>,
    /// A joiner inside an RGI emoji sequence was seen: `fix` compresses it.
    pub sequences: bool,
}

/// One hazard code point: the lints that name it, and where it sits.
#[derive(Debug, Default)]
pub(super) struct Hazard {
    pub lints: String,
    /// The first [`PLACES`] only; `total` counts them all.
    pub places: Vec<String>,
    pub total: usize,
}

/// Most places kept per hazard code point; the rest are counted.
pub(super) const PLACES: usize = 8;

/// `path:line:col`, as `check` prints a finding's place.
fn place(shown: &str, at: Position) -> String {
    format!("{}:{}:{}", one_line(shown), at.line, at.column)
}

/// `shown` with each control character escaped (a newline above all): a
/// path lands in the `.ctrm`, and a raw line break would start a rule.
fn one_line(shown: &str) -> String {
    let mut path = String::new();
    for c in shown.chars() {
        if c.is_control() {
            path.extend(c.escape_default());
        } else {
            path.push(c);
        }
    }
    path
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

    /// One file's text, filed under its type. A double-encoded run is damage
    /// to fix, never characters the type needs (`src/lint/hazard:V142`): it is
    /// filed under what it decodes to, and the code points it is made of are
    /// left out.
    pub(super) fn add(&mut self, shown: &str, text: &str) {
        let exempt = self.hazards.exempt(text);
        let runs = double_encoded(text);
        let kind = self.kinds.entry(pattern_of(shown)).or_default();
        let mut found = kind.begin(&runs);
        for hit in needing(text, &runs) {
            if let Some(lint) = self.hazards.lint_at(hit, &exempt) {
                found.push((hit, lint.name));
            } else if exempt.binary_search(&hit.position.byte).is_ok() {
                kind.sequences = true;
            } else {
                let mapped = !self.keep && self.map.contains(&hit.character);
                kind.file(hit.character, mapped);
            }
        }
        kind.hazards_in(shown, found);
    }
}

impl Kind {
    /// A file of this type is read: its own tally of what `fix` rewrites,
    /// and its double-encoded runs, as hazards by the character each
    /// decodes to.
    fn begin(&mut self, runs: &[Run]) -> Vec<(Hit, &'static str)> {
        self.files = self.files.saturating_add(1);
        self.tallies.push(BTreeMap::new());
        let each = runs.iter().map(|run| (run.hit(), DOUBLE_ENCODED.name));
        each.collect()
    }

    /// One file's hazards, listed in the order `check` reports them: a run
    /// is not listed ahead of a literal hit that comes before it.
    fn hazards_in(&mut self, shown: &str, mut found: Vec<(Hit, &'static str)>) {
        found.sort_by_key(|(hit, _)| hit.position.byte);
        for (hit, lint) in found {
            self.hazard(hit, lint, shown);
        }
    }

    /// A hazard code point, with each place it was found. A character two
    /// lints name (a literal U+202E and a run decoding to it) keeps both.
    fn hazard(&mut self, hit: Hit, lint: &'static str, shown: &str) {
        let found = self.hazards.entry(hit.character).or_default();
        if !found.lints.split(", ").any(|name| name == lint) {
            if !found.lints.is_empty() {
                found.lints.push_str(", ");
            }
            found.lints.push_str(lint);
        }
        found.total = found.total.saturating_add(1);
        if found.places.len() < PLACES {
            found.places.push(place(shown, hit.position));
        }
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
/// with no `/` matches at any depth (`src/rules:V2`). One line whatever the
/// path holds: the draft prints it in comments and in the rule line.
pub(super) fn pattern_of(shown: &str) -> String {
    let name = shown.rsplit('/').next().unwrap_or(shown);
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => {
            format!("*.{}", one_line(ext))
        }
        _ => one_line(name),
    }
}

/// The characters of `text` outside `ascii`, less the double-encoded runs
/// (`src/lint/hazard:V142`), which are no character the file type needs.
fn needing(text: &str, runs: &[Run]) -> Vec<Hit> {
    let mut hits = scan_str(text, plain);
    hits.retain(|hit| !inside(runs, hit.position.byte));
    hits
}
