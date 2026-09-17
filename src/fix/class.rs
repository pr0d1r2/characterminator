//! Equivalence classes: characters that mean the same thing (V28).
//!
//! A class line labels every member with the family it belongs to, and the
//! FIRST member of a family is that family's preferred spelling, because
//! declaration order is the author's ranking.
//!
//! A disallowed member resolves to the preferred member of the rule's
//! fidelity family, and if that is disallowed too, onward along the
//! fallback path until `ascii` (V27). If nothing on the path is allowed the
//! character has no mapping at all, so it is kept and reported (V4) rather
//! than dropped.

use crate::fix::codepoint::decode;
use crate::fix::family::Tree;
use crate::fix::map::{named, violates};
use crate::fix::{Class, Error, Member};

/// Parse the body of a `= <class> <family>:<member>[,<member>...] ...`
/// line -- everything after the `=`.
pub(crate) fn parse_line(rest: &str) -> Option<Class> {
    let mut words = rest.split_whitespace();
    let name = named(words.next()?)?;
    let mut members = Vec::new();
    for word in words {
        group(word, &mut members)?;
    }
    if members.is_empty() {
        return None;
    }
    Some(Class { name, members })
}

/// What this class resolves to for one fidelity family, or `None` when no
/// member anywhere on the fallback path is allowed.
///
/// An unknown or unrooted fidelity family is an error rather than a quiet
/// fallback: the caller asked for a family that cannot be resolved.
pub fn resolve(
    class: &Class,
    tree: &Tree,
    fidelity: &str,
    allowed: &dyn Fn(char) -> bool,
) -> Result<Option<String>, Error> {
    for family in tree.path(fidelity)? {
        if let Some(text) = preferred(class, family, allowed) {
            return Ok(Some(text));
        }
    }
    Ok(None)
}

/// The first member of `family` the caller allows.
fn preferred(
    class: &Class,
    family: &str,
    allowed: &dyn Fn(char) -> bool,
) -> Option<String> {
    class
        .members
        .iter()
        .filter(|member| member.family == family)
        .find(|member| !violates(&member.text, allowed))
        .map(|member| member.text.clone())
}

/// One `<family>:<member>[,<member>...]` group.
fn group(word: &str, members: &mut Vec<Member>) -> Option<()> {
    let (family, list) = word.split_once(':')?;
    let family = named(family)?;
    if family.is_empty() {
        return None;
    }
    for token in list.split(',') {
        members.push(member(&family, token)?);
    }
    Some(())
}

/// One member. An empty member is refused: a class cannot say that the way
/// to write a thing is to write nothing, which is what a delete entry is
/// for (V4).
fn member(family: &str, token: &str) -> Option<Member> {
    let text = decode(token)?;
    if text.is_empty() {
        return None;
    }
    Some(Member {
        family: family.to_owned(),
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_line, resolve};
    use crate::fix::family::Tree;
    use crate::fix::{Class, Error};

    /// The `marks` shape from `src/charset:I.sets`, plus a member in a
    /// family that is not builtin.
    const LINE: &str = "tick ascii:[x] text:U+2713 emoji:U+2705 nerd:U+F00C";

    fn tree() -> Tree {
        let mut tree = Tree::builtin();
        let _ = tree.declare(crate::fix::Family {
            name: String::from("nerd"),
            parent: Some(String::from("emoji")),
        });
        tree
    }

    fn class() -> Class {
        parse_line(LINE).unwrap_or(Class {
            name: String::new(),
            members: Vec::new(),
        })
    }

    fn ascii(ch: char) -> bool {
        ch.is_ascii()
    }

    fn anything(_: char) -> bool {
        true
    }

    fn found(fidelity: &str, allowed: &dyn Fn(char) -> bool) -> Option<String> {
        resolve(&class(), &tree(), fidelity, allowed).unwrap_or_default()
    }

    #[test]
    fn reads_every_member_with_its_family() {
        let parsed = class();
        assert_eq!(parsed.name, "tick");
        assert_eq!(parsed.members.len(), 4);
        let first = parsed.members.first().map(|m| m.family.clone());
        assert_eq!(first, Some(String::from("ascii")));
    }

    #[test]
    fn reads_a_member_of_several_code_points() {
        let parsed = parse_line("hand emoji:U+1F44D+U+1F3FD,U+1F44D");
        let members = parsed.map(|class| class.members).unwrap_or_default();
        let first = members.first().map(|m| m.text.clone());
        assert_eq!(first, Some(String::from("\u{1F44D}\u{1F3FD}")));
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn prefers_the_fidelity_family_when_it_is_allowed() {
        assert_eq!(found("emoji", &anything), Some(String::from("\u{2705}")));
        assert_eq!(found("text", &anything), Some(String::from("\u{2713}")));
    }

    #[test]
    fn falls_back_along_the_path_to_ascii() {
        assert_eq!(found("emoji", &ascii), Some(String::from("[x]")));
        assert_eq!(found("nerd", &ascii), Some(String::from("[x]")));
    }

    #[test]
    fn a_declared_family_resolves_like_any_other() {
        let nerd = |ch: char| ch.is_ascii() || ch == '\u{F00C}';
        assert_eq!(found("nerd", &nerd), Some(String::from("\u{F00C}")));
    }

    #[test]
    fn nothing_allowed_anywhere_is_no_mapping_at_all() {
        let parsed = parse_line("tick text:U+2713 emoji:U+2705");
        let class = parsed.unwrap_or(Class {
            name: String::new(),
            members: Vec::new(),
        });
        let found = resolve(&class, &tree(), "emoji", &ascii);
        assert_eq!(found, Ok(None));
    }

    #[test]
    fn an_unknown_fidelity_family_is_a_config_error() {
        let name = String::from("runic");
        let found = resolve(&class(), &tree(), "runic", &ascii);
        assert_eq!(found, Err(Error::UnknownFamily { name }));
    }

    #[test]
    fn rejects_a_line_it_cannot_read() {
        assert_eq!(parse_line("tick"), None);
        assert_eq!(parse_line("tick emoji"), None);
        assert_eq!(parse_line("tick :x"), None);
        assert_eq!(parse_line("tick emoji:"), None);
        assert_eq!(parse_line("tick emoji:U+ZZZZ"), None);
    }
}
