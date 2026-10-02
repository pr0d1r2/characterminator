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

use super::{ParseError, SetCatalog, SetMember, parse_line};
use std::collections::BTreeSet;

/// The generated locale data, compiled in (V22) but not parsed up front.
pub const LOCALES: &str = include_str!("locales.ctrm-sets");

/// The one line of [`LOCALES`] declaring `name`, found without parsing
/// any other. A comment's first token is `#`, which no name is.
fn line_of(name: &str) -> Option<&'static str> {
    LOCALES
        .lines()
        .find(|line| line.split_once(' ').is_some_and(|(head, _)| head == name))
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
/// Walks the names, and the names their definitions compose in, so an
/// alias line (`pt-BR pt`) brings its parent and a user set naming `pl`
/// brings `pl`. A name neither declared nor a locale stays missing, for
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
            && let Some(parsed) = line_of(&name).map(parse_line)
        {
            parsed?.into_iter().for_each(|d| catalog.insert(d));
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
            catalog.insert(definition);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LOCALES, adopt, adopt_all};
    use crate::charset::builtin::{SETS, catalog};
    use crate::charset::{CharSet, SetCatalog, parse_line};

    const TEXT: &str = "text";

    fn builtin() -> SetCatalog {
        let built = catalog();
        assert!(built.is_ok(), "the compiled-in presets must parse");
        built.unwrap_or_default()
    }

    /// The builtin catalog plus the locales `wanted` reaches.
    fn adopted(wanted: &[&str]) -> SetCatalog {
        let mut sets = builtin();
        let names = wanted.iter().map(|name| (*name).to_owned());
        assert!(adopt(&mut sets, names).is_ok());
        sets
    }

    fn everything() -> SetCatalog {
        let mut sets = builtin();
        assert!(adopt_all(&mut sets).is_ok());
        sets
    }

    fn resolved(sets: &SetCatalog, name: &str) -> CharSet {
        let set = sets.resolve(name, TEXT);
        assert!(set.is_ok(), "{name} must resolve");
        set.unwrap_or_else(|_| CharSet::new(name.to_owned(), Vec::new()))
    }

    fn locale(name: &str) -> CharSet {
        resolved(&adopted(&[name]), name)
    }

    fn locale_names() -> Vec<String> {
        let parsed = LOCALES.lines().map(parse_line);
        let names = parsed.filter_map(|line| line.ok().flatten());
        names.map(|definition| definition.name).collect()
    }

    /// V61's LAZY half: the eager builtin text holds no locale, and a run
    /// naming none adds none.
    #[test]
    fn a_run_naming_no_locale_parses_none() {
        assert!(!SETS.contains("\npl "));
        let before: Vec<String> = builtin().names().map(String::from).collect();
        let sets = adopted(&["ascii", "caveman", "hazard", "nonesuch"]);
        let after: Vec<String> = sets.names().map(String::from).collect();
        assert_eq!(before, after);
    }

    /// An alias brings its parent and nothing else.
    #[test]
    fn a_named_locale_brings_only_its_chain() {
        let before = builtin().names().count();
        let sets = adopted(&["pt-BR"]);
        assert!(sets.get("pt-BR").is_some() && sets.get("pt").is_some());
        assert!(sets.get("pl").is_none());
        assert_eq!(sets.names().count(), before.saturating_add(2));
    }

    #[test]
    fn a_variant_resolves_through_its_parent() {
        assert_eq!(locale("pt-BR").ranges, locale("pt").ranges);
        assert_eq!(locale("es-MX").ranges, locale("es").ranges);
        assert!(locale("pt-BR").contains('\u{00E7}'));
        assert!(locale("sr-Latn").contains('\u{0107}'));
        assert!(locale("sr").contains('\u{0436}'));
    }

    /// A user set named like a locale wins: only a miss is looked up.
    #[test]
    fn a_declared_set_wins_over_the_locale() {
        let mut sets = builtin();
        let mine = parse_line("pl U+2261").ok().flatten();
        mine.into_iter()
            .for_each(|definition| sets.insert(definition));
        assert!(adopt(&mut sets, ["pl".to_owned()]).is_ok());
        assert!(resolved(&sets, "pl").contains('\u{2261}'));
        assert!(!resolved(&sets, "pl").contains('\u{0105}'));
    }

    /// V61: CJK, Indic and RTL are in, as are the 1B Latin locales; and a
    /// loan letter sits in `-aux`, not in the main set.
    const HELD: [(&str, char); 11] = [
        ("ja", '\u{3042}'),
        ("zh", '\u{4E2D}'),
        ("zh-Hant", '\u{570B}'),
        ("ko", '\u{AC00}'),
        ("hi", '\u{0915}'),
        ("ar", '\u{0628}'),
        ("he", '\u{05D0}'),
        ("pl", '\u{0104}'),
        ("de", '\u{00DF}'),
        ("tr", '\u{0130}'),
        ("pl-aux", '\u{00E4}'),
    ];

    #[test]
    fn every_script_is_covered() {
        for (name, letter) in HELD {
            assert!(locale(name).contains(letter), "{name} {letter}");
        }
        assert!(!locale("pl").contains('\u{00E4}'));
    }

    #[test]
    fn every_locale_resolves_grants_letters_and_no_ascii() {
        let sets = everything();
        let names = locale_names();
        assert_eq!(names.len(), 1563);
        for name in names {
            let set = resolved(&sets, &name);
            assert!(!set.is_empty(), "{name} grants nothing");
            assert!(set.ranges.iter().all(|r| r.start > '\u{7F}'), "{name}");
        }
    }

    /// The generator subtracts the hazard file (V59); this says it did,
    /// for every locale, range against range.
    #[test]
    fn no_locale_holds_a_hazard() {
        let sets = everything();
        let hazard = resolved(&sets, "hazard");
        for name in locale_names() {
            let set = resolved(&sets, &name);
            let hit = set.ranges.iter().any(|r| {
                hazard
                    .ranges
                    .iter()
                    .any(|h| r.start <= h.end && h.start <= r.end)
            });
            assert!(!hit, "{name} holds a hazard");
        }
    }

    /// A locale never shadows a preset, which would make `greek` mean
    /// two things depending on whether a run had read the locale file.
    #[test]
    fn no_locale_is_named_like_a_preset() {
        let presets = builtin();
        for name in locale_names() {
            assert!(presets.get(&name).is_none(), "{name} is a preset");
        }
    }

    #[test]
    fn the_locale_file_is_ascii_and_out_of_the_eager_text() {
        assert!(LOCALES.is_ascii());
        assert!(!SETS.contains(LOCALES));
    }
}
