//! The smallest grant that covers a file type (V128): a greedy set cover
//! over the curated presets, then ONE CLDR locale if letters are left,
//! then `any`, said as such.

use crate::charset::{CharSet, SetCatalog, locale_names};
use crate::rules::TEXT;
use std::collections::BTreeSet;

/// The function presets a draft may grant, in the order a tie goes to:
/// the presets `src/charset:V23` sizes for code and docs. The coarse
/// blocks and scripts are left out on purpose: V23 recommends them only
/// for multi-language data, which a locale answers better.
const PRESETS: &[&str] = &[
    "caveman",
    "typography",
    "math",
    "legal",
    "marks",
    "box",
    "emoji",
    "cr",
];

/// What a draft grants one file type.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Grant {
    /// Presets (and maybe one locale), each with what it covers.
    Sets(Vec<(String, Vec<char>)>),
    /// Nothing curated covers these; `any` is the honest grant.
    Any(Vec<char>),
}

/// The builtin sets, resolved once: the presets and every locale.
pub(super) struct Candidates {
    presets: Vec<CharSet>,
    locales: Vec<CharSet>,
}

impl Candidates {
    pub(super) fn builtin() -> Self {
        let catalog = SetCatalog::builtin();
        let resolve = |name: &str| catalog.resolve(name, TEXT).ok();
        Self {
            presets: PRESETS.iter().filter_map(|name| resolve(name)).collect(),
            locales: locale_names().filter_map(resolve).collect(),
        }
    }

    /// The grant for `needed`: presets by greatest gain, then the smallest
    /// locale covering what is left, else `any`.
    pub(super) fn cover(&self, needed: &BTreeSet<char>) -> Grant {
        let mut left = needed.clone();
        let mut chosen = Vec::new();
        while let Some(best) = best(&self.presets, &left) {
            chosen.push(taken(best, &mut left));
        }
        if left.is_empty() {
            return Grant::Sets(chosen);
        }
        match smallest_covering(&self.locales, &left) {
            Some(locale) => {
                chosen.push(taken(locale, &mut left));
                Grant::Sets(chosen)
            }
            None => Grant::Any(left.into_iter().collect()),
        }
    }
}

/// The preset covering the most of `left`; a tie goes to the smaller set,
/// then to the earlier in [`PRESETS`]. `None` once nothing gains.
fn best<'a>(sets: &'a [CharSet], left: &BTreeSet<char>) -> Option<&'a CharSet> {
    let gain =
        |set: &CharSet| left.iter().filter(|c| set.contains(**c)).count();
    let mut found: Option<(&CharSet, usize)> = None;
    for set in sets {
        let won = gain(set);
        let better = found.is_none_or(|(held, had)| {
            won > had || (won == had && size(set) < size(held))
        });
        if won > 0 && better {
            found = Some((set, won));
        }
    }
    found.map(|(set, _)| set)
}

/// The locale a draft tries FIRST: English's loan letters (`cafe` with
/// U+00E9, `naive` with U+00EF), the accents an English-language
/// repository actually holds. Without it the smallest covering set wins,
/// which for one U+00E9 is a language nobody in the repository writes.
const LOAN_LETTERS: &str = "en-aux";

/// `en-aux` when it holds all of `left`, else the best-ranked locale that
/// does ([`rank`]).
fn smallest_covering<'a>(
    locales: &'a [CharSet],
    left: &BTreeSet<char>,
) -> Option<&'a CharSet> {
    let covers = |set: &&CharSet| left.iter().all(|c| set.contains(*c));
    let covering = locales.iter().filter(covers);
    let loan = covering.clone().find(|set| set.name == LOAN_LETTERS);
    loan.or_else(|| covering.min_by(|a, b| rank(a).cmp(&rank(b))))
}

/// `set`'s name and what it takes out of `left`.
fn taken(set: &CharSet, left: &mut BTreeSet<char>) -> (String, Vec<char>) {
    let covered: Vec<char> =
        left.iter().copied().filter(|c| set.contains(*c)).collect();
    left.retain(|c| !set.contains(*c));
    (set.name.clone(), covered)
}

/// A base locale (`zh`) before a variant (`yue-Hans`), then the smaller,
/// then by name: deterministic, and the code a reader would write.
fn rank(set: &CharSet) -> (bool, u32, &str) {
    (set.name.contains('-'), size(set), set.name.as_str())
}

/// How many code points a set holds, saturating.
fn size(set: &CharSet) -> u32 {
    set.ranges.iter().fold(0_u32, |sum, range| {
        let width = u32::from(range.end).saturating_sub(u32::from(range.start));
        sum.saturating_add(width).saturating_add(1)
    })
}
