//! `sets` (V127): what a reader or an agent may name, curated first.
//!
//! The question used to be answered with every set the run could name,
//! alphabetically: 1,800 CLDR locale lines ahead of the presets a typical
//! file needs one of, none saying what it was for. Now the curated presets
//! lead, each with its description (`#:` lines beside the data,
//! `src/charset:V22`), then what the repository declared; the locales are
//! counted unless `--locales` asks for them, and a name or `--containing`
//! narrows the list to what was asked.

use crate::charset::{CharSet, builtin, code_points, locale_names};
use crate::judge::{Checker, Config};
use crate::render::{Format, Listed, Listing, SetKind, listing};
use crate::rules;
use std::collections::{HashMap, HashSet};

/// What `sets` was asked: names to list, the locales, a code point.
pub(crate) struct Asked<'a> {
    pub names: &'a [String],
    pub locales: bool,
    pub containing: Option<&'a str>,
}

/// `sets [<name>...] [--locales] [--containing <c>]`.
///
/// Every set is resolved at the family the run's rules give the whole
/// repo (`src/rules:V29`), as before: a labelled preset holds different
/// characters at another family (B30).
///
/// # Errors
///
/// A name nothing declares, a `--containing` that is not one character,
/// or a declared set that cannot be resolved.
pub(crate) fn sets(
    config: &Config,
    format: Format,
    asked: &Asked<'_>,
) -> Result<String, String> {
    let point = asked.containing.map(point_of).transpose()?;
    let all = every_set(config)?;
    let entries = entries(&all);
    let holds = |item: &&Listed<'_>| point.is_none_or(|p| item.set.contains(p));
    let chosen = chosen(&entries, asked)?.into_iter().filter(holds);
    let counted = asked.names.is_empty() && !asked.locales && point.is_none();
    let locales = entries.iter().filter(|e| e.kind == SetKind::Locale);
    let unlisted_locales = if counted { locales.count() } else { 0 };
    let sets = chosen.map(copied).collect();
    let answer = Listing {
        sets,
        containing: point,
        unlisted_locales,
    };
    Ok(listing(format, &answer))
}

fn every_set(config: &Config) -> Result<Vec<CharSet>, String> {
    let checker = Checker::listing(config)?;
    let family = rules::resolve("", &config.rules()?, &rules::matches).family;
    checker.declared(&family)
}

fn copied<'a>(item: &Listed<'a>) -> Listed<'a> {
    Listed {
        set: item.set,
        kind: item.kind,
        description: item.description,
    }
}

/// `U+22A5` or the character itself, and exactly one code point.
fn point_of(asked: &str) -> Result<char, String> {
    let text = code_points(asked).unwrap_or_else(|| asked.to_owned());
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(point), None) => Ok(point),
        _ => Err(format!(
            "--containing takes one character or U+XXXX, not `{asked}`"
        )),
    }
}

/// Every nameable set, in listing order: presets as the data file orders
/// them, then the declared, then the locales.
fn entries(all: &[CharSet]) -> Vec<Listed<'_>> {
    let by_name: HashMap<&str, &CharSet> =
        all.iter().map(|set| (set.name.as_str(), set)).collect();
    let find = |name: &str| by_name.get(name).copied();
    let presets = builtin::described().filter_map(|(name, said)| {
        Some(entry(find(name)?, SetKind::Preset, Some(said)))
    });
    let mut out: Vec<Listed<'_>> = presets.collect();
    out.extend(own(all).map(|set| entry(set, SetKind::Declared, None)));
    let locales = locale_names().filter_map(find);
    out.extend(locales.map(|set| entry(set, SetKind::Locale, None)));
    out
}

/// The sets neither a curated preset nor a CLDR locale: the repository's.
fn own(all: &[CharSet]) -> impl Iterator<Item = &CharSet> {
    let builtin = builtin::described().map(|(name, _)| name);
    let known: HashSet<&str> = locale_names().chain(builtin).collect();
    all.iter()
        .filter(move |set| !known.contains(set.name.as_str()))
}

const fn entry<'a>(
    set: &'a CharSet,
    kind: SetKind,
    description: Option<&'a str>,
) -> Listed<'a> {
    Listed {
        set,
        kind,
        description,
    }
}

/// What was asked for: the named sets in the order named, else the
/// locales, else everything when a code point is the question, else the
/// presets and the declared.
fn chosen<'e, 'a>(
    entries: &'e [Listed<'a>],
    asked: &Asked<'_>,
) -> Result<Vec<&'e Listed<'a>>, String> {
    if !asked.names.is_empty() {
        return named(entries, asked.names);
    }
    let wanted = |kind: SetKind| match (asked.locales, asked.containing) {
        (true, _) => kind == SetKind::Locale,
        (false, Some(_)) => true,
        (false, None) => kind != SetKind::Locale,
    };
    Ok(entries.iter().filter(|e| wanted(e.kind)).collect())
}

/// The named sets, each once, in the order named. A name nothing declares
/// is REFUSED: `sets a.md` reads as an answer about `a.md` otherwise
/// (`src/cli:B39`).
fn named<'e, 'a>(
    entries: &'e [Listed<'a>],
    names: &[String],
) -> Result<Vec<&'e Listed<'a>>, String> {
    let mut out: Vec<&Listed<'a>> = Vec::new();
    for name in names {
        let found = entries.iter().find(|entry| entry.set.name == *name);
        let found = found.ok_or_else(|| format!("unknown set `{name}`"))?;
        if !out.iter().any(|kept| kept.set.name == *name) {
            out.push(found);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "listing_test.rs"]
mod tests;
