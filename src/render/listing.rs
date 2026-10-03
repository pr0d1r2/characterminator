//! `sets`, as a reader and an agent ask it (`src/cli/explain:V127`): the
//! curated presets first, each with the line that says what it is for,
//! then what the repository declared, and the CLDR locales counted rather
//! than listed unless asked for.
//!
//! Its own file rather than a branch of `human.rs` and `json.rs`: the
//! listing carries a kind and a description the bare [`super::sets`] form
//! has no room for, and the prompt still reads that bare form.

use crate::charset::CharSet;
use crate::render::Format;
use crate::render::escape::string;
use crate::render::name::codepoint;
use crate::render::value::{array, field, number, object, optional};

/// Where a listed set comes from, which is also the order it lists in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SetKind {
    /// A builtin with a description: the curated presets and hazards.
    Preset,
    /// Declared by the repository or a flag (`.ctrm-sets`, `--set`).
    Declared,
    /// One CLDR locale's letters (`src/charset/locale:V30`).
    Locale,
}

impl SetKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Preset => "preset",
            Self::Declared => "declared",
            Self::Locale => "locale",
        }
    }
}

/// One set as listed.
pub(crate) struct Listed<'a> {
    pub set: &'a CharSet,
    pub kind: SetKind,
    pub description: Option<&'a str>,
}

/// The whole answer: what is listed, what was asked for, and how many
/// locale sets were left out of it.
pub(crate) struct Listing<'a> {
    pub sets: Vec<Listed<'a>>,
    /// The `--containing` code point, when one was asked about.
    pub containing: Option<char>,
    /// Locale sets NOT listed: counted in one line, so a reader learns
    /// they exist without 1,800 lines of them.
    pub unlisted_locales: usize,
}

/// The listing in the asked-for form.
pub(crate) fn listing(format: Format, listing: &Listing<'_>) -> String {
    match format {
        Format::Human => human(listing),
        Format::Json | Format::Sarif => json(listing),
    }
}

fn human(listing: &Listing<'_>) -> String {
    let mut lines: Vec<String> = listing.sets.iter().map(human_set).collect();
    if let (true, Some(point)) = (lines.is_empty(), listing.containing) {
        lines.push(format!("no set holds {}", codepoint(point)));
    }
    if listing.unlisted_locales > 0 {
        lines.push(format!(
            "{} CLDR locale sets: `ctrm sets --locales`, or name one: \
             `ctrm sets pl`",
            listing.unlisted_locales
        ));
    }
    lines.join("\n")
}

/// `name -- description`, then the ranges indented under it.
fn human_set(item: &Listed<'_>) -> String {
    let said = item.description.unwrap_or(match item.kind {
        SetKind::Locale => "one locale's letters, from CLDR",
        _ => "declared by this repository",
    });
    format!("{} -- {said}\n  {}", item.set.name, ranges(item.set))
}

fn ranges(set: &CharSet) -> String {
    let each = set.ranges.iter().map(|range| {
        if range.start == range.end {
            return codepoint(range.start);
        }
        format!("{}-{}", codepoint(range.start), codepoint(range.end))
    });
    let all: Vec<String> = each.collect();
    if all.is_empty() {
        return String::from("none");
    }
    all.join(" ")
}

fn json(listing: &Listing<'_>) -> String {
    let sets: Vec<String> = listing.sets.iter().map(json_set).collect();
    let asked = listing.containing.map(codepoint);
    object(&[
        field("verb", &string("sets")),
        field("containing", &optional(asked.as_deref())),
        field("sets", &array(&sets)),
        field("unlisted_locales", &number(listing.unlisted_locales)),
    ])
}

fn json_set(item: &Listed<'_>) -> String {
    let each = item.set.ranges.iter().map(|range| {
        object(&[
            field("start", &string(&codepoint(range.start))),
            field("end", &string(&codepoint(range.end))),
        ])
    });
    let ranges: Vec<String> = each.collect();
    object(&[
        field("name", &string(&item.set.name)),
        field("kind", &string(item.kind.name())),
        field("description", &optional(item.description)),
        field("ranges", &array(&ranges)),
    ])
}

#[cfg(test)]
mod tests {
    use super::{Listed, Listing, SetKind, listing};
    use crate::charset::{CharRange, CharSet};
    use crate::render::Format;

    fn caveman() -> CharSet {
        let arrow = CharRange::single('\u{2192}');
        CharSet::new(String::from("caveman"), vec![arrow])
    }

    fn one(set: &CharSet, unlisted: usize) -> Listing<'_> {
        let item = Listed {
            set,
            kind: SetKind::Preset,
            description: Some("arrows"),
        };
        Listing {
            sets: vec![item],
            containing: None,
            unlisted_locales: unlisted,
        }
    }

    #[test]
    fn a_preset_lists_with_its_description_and_the_locales_are_counted() {
        let set = caveman();
        let said = listing(Format::Human, &one(&set, 1831));
        assert_eq!(
            said,
            "caveman -- arrows\n  U+2192\n1831 CLDR locale sets: \
             `ctrm sets --locales`, or name one: `ctrm sets pl`"
        );
    }

    /// The json mirrors it: kind, description, and the count of what was
    /// left out, every key present (`src/render:V95`).
    #[test]
    fn the_json_carries_kind_description_and_the_unlisted_count() {
        let set = caveman();
        let expected = concat!(
            r#"{"verb":"sets","containing":null,"sets":[{"name":"caveman","#,
            r#""kind":"preset","description":"arrows","ranges":"#,
            r#"[{"start":"U+2192","end":"U+2192"}]}],"unlisted_locales":0}"#
        );
        assert_eq!(listing(Format::Json, &one(&set, 0)), expected);
    }

    #[test]
    fn a_code_point_no_set_holds_is_said_so() {
        let asked = Listing {
            sets: Vec::new(),
            containing: Some('\u{2295}'),
            unlisted_locales: 0,
        };
        assert_eq!(listing(Format::Human, &asked), "no set holds U+2295");
    }
}
