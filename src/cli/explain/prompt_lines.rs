//! The sections of `explain --as prompt` after the intro (V129): members,
//! replacements, the hazard line and the pedantic line. Each reads the
//! node that owns its answer, so the prompt cannot drift from `check`.

use super::prompt::Scope;
use crate::charset::{CharRange, CharSet, SetCatalog, builtin};
use crate::fix::{self as engine, Map};
use crate::lint::{Group, Hazards, LINTS, Level, Levels};
use crate::render::{Format, codepoint, sets};
use crate::rules;
use std::collections::BTreeMap;

/// A set at most this many code points long is listed glyph by glyph.
const SMALL: u32 = 64;

/// Every named set and its members: a small one as `U+XXXX "g"` pairs on
/// one line, so a model sees the letters themselves; a large one as
/// ranges, which a model reads as well as a person does.
pub(super) fn members(
    scope: &Scope,
    catalog: &SetCatalog,
) -> Result<Vec<String>, String> {
    let mut out =
        vec![String::from("\nSet members (U+XXXX, ranges inclusive):")];
    for name in &scope.names {
        let set = catalog.resolve(name, &scope.family);
        out.push(member_line(&set.map_err(|bad| bad.to_string())?));
    }
    Ok(out)
}

fn member_line(set: &CharSet) -> String {
    if size(&set.ranges).is_none_or(|n| n > SMALL) {
        return sets(Format::Human, std::slice::from_ref(set));
    }
    let each: Vec<String> = points_of(&set.ranges).map(glyphed).collect();
    format!("{} {}", set.name, each.join(" "))
}

/// How many code points the ranges hold, `None` past `u32`.
fn size(ranges: &[CharRange]) -> Option<u32> {
    ranges.iter().try_fold(0_u32, |sum, range| {
        let width = u32::from(range.end).checked_sub(u32::from(range.start))?;
        sum.checked_add(width)?.checked_add(1)
    })
}

fn points_of(ranges: &[CharRange]) -> impl Iterator<Item = char> + '_ {
    ranges.iter().flat_map(|range| range.start..=range.end)
}

/// `U+0105 "\u{105}"`, the glyph quoted; a character with no glyph to
/// show (a space, a control) is its code point alone.
fn glyphed(point: char) -> String {
    if point.is_whitespace() || point.is_control() {
        return codepoint(point);
    }
    format!("{} \"{point}\"", codepoint(point))
}

/// What a writer plausibly TYPES that a stricter set refuses: dashes,
/// quotes (the low-9 ones a locale opens with included), ellipsis,
/// no-break spaces, apostrophes. The builtin map also deletes emoji
/// modifiers and hazards; those are not typed, so they are not listed.
const TYPED: &[(char, char)] = &[
    ('\u{00A0}', '\u{00A0}'),
    ('\u{00AB}', '\u{00AB}'),
    ('\u{00BB}', '\u{00BB}'),
    ('\u{02BC}', '\u{02BC}'),
    ('\u{2010}', '\u{2015}'),
    ('\u{2018}', '\u{201F}'),
    ('\u{2026}', '\u{2026}'),
    ('\u{202F}', '\u{202F}'),
    ('\u{2039}', '\u{203A}'),
    ('\u{2212}', '\u{2212}'),
];

/// The replacements `fix` would make for what a writer types, one line
/// each, sorted by code point, each once. The map is ASKED through the
/// engine `fix` runs, so the answer cannot drift from what `fix` does.
/// A line the user's own map declares is listed whatever it is: they
/// asked for it.
pub(super) fn replacements(
    map: &Map,
    allowed: &CharSet,
) -> Result<Vec<String>, String> {
    let hazards = Hazards::builtin();
    let mut sorted: BTreeMap<char, String> = BTreeMap::new();
    for point in sources(map).into_iter().filter(|c| !hazards.contains(*c)) {
        if let Some(line) = replacement(map, allowed, point)? {
            sorted.insert(point, line);
        }
    }
    let mut out = vec![String::from(
        "\nInstead of these, write the replacement (what `ctrm fix` would do):",
    )];
    out.extend(sorted.into_values());
    out.extend(sequences(map, allowed));
    Ok(out)
}

/// One source's line, or none when `fix` would leave it as it is.
fn replacement(
    map: &Map,
    allowed: &CharSet,
    point: char,
) -> Result<Option<String>, String> {
    let source = point.to_string();
    let fixed = engine::fix(&source, map, |c| allowed.contains(c));
    let fixed = fixed.map_err(|bad| format!("{}: {bad}", codepoint(point)))?;
    Ok((fixed.output != source)
        .then(|| format!("{} -> {:?}", codepoint(point), fixed.output)))
}

/// The single code points worth listing: typed ones, and any the user's
/// own map (a file or a flag) declares.
fn sources(map: &Map) -> Vec<char> {
    let typed = |c: char| TYPED.iter().any(|(lo, hi)| (*lo..=*hi).contains(&c));
    let declared = map.entries().iter().filter_map(|entry| {
        let mut chars = entry.from.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return None;
        };
        let own = !matches!(entry.origin, rules::Origin::Builtin { .. });
        (own || typed(c)).then_some(c)
    });
    let classes = map.classes().iter().flat_map(|class| &class.members);
    let members =
        classes.filter_map(|m| m.text.chars().next().filter(|c| typed(*c)));
    declared.chain(members).collect()
}

/// The emoji sequences, said as one line when `fix` rewrites them: they
/// run to nearly two thousand. A set granting the joiner (`any`) keeps a
/// sequence whole, so there it would be a false sentence.
fn sequences(map: &Map, allowed: &CharSet) -> Option<String> {
    let any = map
        .entries()
        .iter()
        .any(|e| e.from.chars().nth(1).is_some());
    (any && !allowed.contains('\u{200D}')).then(|| {
        String::from(
            "Write one emoji, not a sequence: a keycap, flag or ZWJ sequence \
             becomes its digit, its region code (`PL`), or one emoji.",
        )
    })
}

/// The hazard classes, ONCE, as one line of what each is, read from the
/// descriptions beside the data the check itself reads
/// (`src/lint/hazard:V34`).
pub(super) fn hazards(allowed: &CharSet) -> String {
    let classes: Vec<String> = builtin::described()
        .filter(|(name, _)| name.starts_with("hazard-"))
        .map(|(name, said)| format!("{said} ({name})"))
        .collect();
    let joiner = if allowed.contains('\u{200D}') {
        "; a joiner or variation selector inside an RGI emoji sequence is fine"
    } else {
        ""
    };
    format!(
        "\nNever write these, whatever a set allows; they always fail: {}. \
         U+FEFF only as the very first character of a file{joiner}.",
        classes.join("; ")
    )
}

/// One line naming the pedantic lints in force, when any is.
pub(super) fn pedantic(levels: &Levels) -> Option<String> {
    let on: Vec<&str> = LINTS
        .iter()
        .filter(|lint| lint.group == Group::Pedantic)
        .filter(|lint| levels.level_of(**lint) != Level::Allow)
        .map(|lint| lint.name)
        .collect();
    (!on.is_empty()).then(|| {
        format!(
            "\nPedantic lints are on, so these fail too: {}.",
            on.join(", ")
        )
    })
}
