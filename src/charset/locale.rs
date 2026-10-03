//! The CLDR locale letter presets (V30), resolved LAZILY (V61).
//!
//! `locales.ctrm-sets` covers every locale in `cldr-misc-full` and is over
//! ten times the size of every other preset together. The builtin text
//! ([`super::builtin::SETS`]) is parsed whole by every run, so the locale
//! file is kept OUT of it and read on demand instead: a name the catalog
//! misses is looked up here by its first token, and only that line is
//! parsed. A run naming no locale parses none.
//!
//! Only a MISS is looked up, so a set a user declares under a locale's
//! name still wins (`src/rules:V19`), exactly as it would over a preset.

use super::builtin::{ASCII, ascii_definition};
use super::{ParseError, SetCatalog, SetDefinition, SetMember, parse_line};
use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

/// The generated locale data, compiled in (V22) but not parsed up front.
pub const LOCALES: &str = include_str!("locales.ctrm-sets");

/// Each line of [`LOCALES`] by its first token, built once per process on
/// first use (V88), so a lookup no longer walks 180 KB of text. A run
/// naming no locale never builds it. The FIRST line per name is kept,
/// which is the line a forward scan would have found.
static INDEX: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut index = HashMap::new();
        for line in LOCALES.lines() {
            if let Some((head, _)) = line.split_once(' ') {
                index.entry(head).or_insert(line);
            }
        }
        index
    });

/// The one line of [`LOCALES`] declaring `name`, found without parsing
/// any other. A comment's first token is `#`, which no name is.
fn line_of(name: &str) -> Option<&'static str> {
    INDEX.get(name).copied()
}

/// The locale line declaring `name`, parsed, with every name it composes
/// in replaced by that name's members from THIS file (V64).
///
/// An alias line (`pt-BR pt`, `en ascii`) names its parent, and resolving
/// that name through the run's catalog would let a user set called `pt`
/// stand in for the CLDR parent: `--set 'pt U+0161'` would turn `pt-BR`
/// into a grant of U+0161. So the parent is read here, from the
/// compiled-in data, and `ascii` is the intrinsic one.
fn compiled(name: &str) -> Result<Option<SetDefinition>, ParseError> {
    match line_of(name).map(parse_line).transpose()?.flatten() {
        Some(definition) => inlined(definition).map(Some),
        None => Ok(None),
    }
}

/// `definition` with each named member swapped for its compiled-in one.
fn inlined(definition: SetDefinition) -> Result<SetDefinition, ParseError> {
    let mut members = Vec::new();
    for member in definition.members {
        members.extend(parent_of(member)?);
    }
    Ok(SetDefinition {
        name: definition.name,
        members,
    })
}

/// One member, a named one replaced by its compiled-in members.
fn parent_of(member: SetMember) -> Result<Vec<SetMember>, ParseError> {
    Ok(match member {
        SetMember::Named(name) if name == ASCII => ascii_definition().members,
        SetMember::Named(name) => match compiled(&name)? {
            Some(found) => found.members,
            None => vec![SetMember::Named(name)],
        },
        other => vec![other],
    })
}

/// The name a member composes in, through a fidelity label too.
fn named(member: &SetMember) -> Option<&String> {
    match member {
        SetMember::Named(name) => Some(name),
        SetMember::Labelled { member, .. } => named(member),
        SetMember::Literal(_) | SetMember::Range(_) => None,
    }
}

/// Declare every locale set `wanted` reaches that `catalog` lacks.
///
/// Walks the names, and the names their definitions compose in, so a user
/// set naming `pl` brings `pl`. An alias line (`pt-BR pt`) brings no
/// second name: its parent's members are read from this file into it. A name neither declared nor a locale stays missing, for
/// resolution to report as unknown exactly as before.
///
/// # Errors
///
/// A locale line that does not parse: a defect in this crate, which the
/// tests below keep from shipping.
pub fn adopt<I>(catalog: &mut SetCatalog, wanted: I) -> Result<(), ParseError>
where
    I: IntoIterator<Item = String>,
{
    let mut queue: Vec<String> = wanted.into_iter().collect();
    let mut seen = BTreeSet::new();
    while let Some(name) = queue.pop() {
        if catalog.get(&name).is_none()
            && let Some(definition) = compiled(&name)?
        {
            catalog.insert(definition);
        }
        let members = catalog.get(&name).map(|d| d.members.iter());
        let reached = members.into_iter().flatten().filter_map(named);
        queue.extend(reached.filter(|n| !seen.contains(*n)).cloned());
        seen.insert(name);
    }
    Ok(())
}

/// Declare EVERY locale set `catalog` lacks: what `ctrm sets` lists, since
/// its question is "what may I name here".
///
/// # Errors
///
/// As [`adopt`].
pub fn adopt_all(catalog: &mut SetCatalog) -> Result<(), ParseError> {
    for line in LOCALES.lines() {
        if let Some(definition) = parse_line(line)?
            && catalog.get(&definition.name).is_none()
        {
            catalog.insert(inlined(definition)?);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "locale_test.rs"]
mod tests;
