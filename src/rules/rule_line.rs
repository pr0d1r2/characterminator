//! The `.ctrm` line, and nothing else.
//!
//! Grammar, as the node's interface section fixes it:
//!
//! ```text
//! <glob|path> <set>[+<set>...] [@<family>] [!<level>] [!<lint|group>=<level>]
//! ```
//!
//! Fields are separated by blanks and told apart by their first character:
//! a commercial at introduces the fidelity family, an exclamation mark a
//! level, and anything else is the set list. That is why the parse needs
//! no lookahead and no dependency.

use crate::lint::Level;
use crate::rules::line::{ParseError, error};
use crate::rules::{LevelChoice, Origin, Rule};

impl Rule {
    /// A rule that matches `pattern` and grants nothing yet.
    fn bare(pattern: String, origin: Origin) -> Self {
        Rule {
            pattern,
            sets: Vec::new(),
            family: None,
            levels: Vec::new(),
            default_level: None,
            origin,
        }
    }
}

/// Parse one `.ctrm` line, which is also the value of one `--rule` flag.
///
/// The line is taken already trimmed and known not to be blank or a
/// comment: finding the entry-bearing lines is the shared skeleton's job.
pub fn parse_rule(text: &str, origin: Origin) -> Result<Rule, ParseError> {
    let mut fields = text.split_ascii_whitespace();
    let Some(pattern) = fields.next() else {
        return Err(error(origin, "a rule line needs a path or a glob"));
    };
    let mut rule = Rule::bare(pattern.to_string(), origin.clone());
    for field in fields {
        add_field(&mut rule, field, &origin)?;
    }
    says_something(rule)
}

/// A rule must grant, name a family or set a level (V75). `--rule 'a.md'`
/// used to parse as a rule that matched and changed nothing, which reads
/// exactly like a rule that worked.
fn says_something(rule: Rule) -> Result<Rule, ParseError> {
    let silent = rule.sets.is_empty()
        && rule.family.is_none()
        && rule.levels.is_empty()
        && rule.default_level.is_none();
    if silent {
        let why = "a rule names a set, an `@family` or a `!level`";
        return Err(error(rule.origin, why));
    }
    Ok(rule)
}

/// Route one field by its first character.
fn add_field(
    rule: &mut Rule,
    field: &str,
    origin: &Origin,
) -> Result<(), ParseError> {
    if let Some(name) = field.strip_prefix('@') {
        return set_family(rule, name, origin);
    }
    if let Some(term) = field.strip_prefix('!') {
        return add_level(rule, term, origin);
    }
    add_sets(rule, field, origin)
}

/// The fidelity suffix, held as a NAME.
///
/// What a family CONTAINS, and which member of a class it prefers, is the
/// fix node's question (`src/fix:V27`). A rule only has to say which one
/// it wants, so this node never resolves the name.
fn set_family(
    rule: &mut Rule,
    name: &str,
    origin: &Origin,
) -> Result<(), ParseError> {
    if name.is_empty() {
        return Err(error(origin.clone(), "`@` needs a family name"));
    }
    if rule.family.is_some() {
        return Err(error(origin.clone(), "one family per rule"));
    }
    rule.family = Some(name.to_string());
    Ok(())
}

/// The set list, `a+b+c`.
///
/// Sets compose by union only (`src/charset:V3`), so the plus sign is the
/// only operator and there is no precedence to get wrong. Which NAMES
/// exist is the charset node's question; a rule only names them.
fn add_sets(
    rule: &mut Rule,
    field: &str,
    origin: &Origin,
) -> Result<(), ParseError> {
    if !rule.sets.is_empty() {
        return Err(error(origin.clone(), "one set list per rule"));
    }
    for name in field.split('+') {
        if name.is_empty() {
            return Err(error(origin.clone(), "empty set name in the list"));
        }
        rule.sets.push(name.to_string());
    }
    Ok(())
}

/// `!<lint|group>=<level>` names a target; bare `!<level>` does not.
fn add_level(
    rule: &mut Rule,
    term: &str,
    origin: &Origin,
) -> Result<(), ParseError> {
    let Some((target, name)) = term.split_once('=') else {
        return set_default_level(rule, term, origin);
    };
    if target.is_empty() {
        return Err(error(origin.clone(), "`!=` needs a lint or a group"));
    }
    let level = level_named(name, origin)?;
    rule.levels.push(LevelChoice {
        target: target.to_string(),
        level,
    });
    Ok(())
}

/// The rule's own level, the one a named target falls back to.
fn set_default_level(
    rule: &mut Rule,
    name: &str,
    origin: &Origin,
) -> Result<(), ParseError> {
    if rule.default_level.is_some() {
        return Err(error(origin.clone(), "one bare level per rule"));
    }
    rule.default_level = Some(level_named(name, origin)?);
    Ok(())
}

/// The four level names.
///
/// The LEVELS are the lint node's type and are used as such; the SPELLING
/// is part of this node's line grammar, and these four are the rustc and
/// clippy names, which is the vocabulary a reader already has.
fn level_named(name: &str, origin: &Origin) -> Result<Level, ParseError> {
    match name {
        "allow" => Ok(Level::Allow),
        "warn" => Ok(Level::Warn),
        "deny" => Ok(Level::Deny),
        "forbid" => Ok(Level::Forbid),
        _ => Err(error(origin.clone(), format!("unknown level `{name}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> Origin {
        Origin::Argument { index: 1 }
    }

    fn parse(text: &str) -> Option<Rule> {
        parse_rule(text, origin()).ok()
    }

    /// V75: a pattern alone grants and levels nothing, so it is refused at
    /// its origin rather than accepted as a rule that did nothing.
    #[test]
    fn a_pattern_alone_is_refused() {
        let failure = parse_rule("docs/**", Origin::Argument { index: 3 });
        let failure = failure.err().map(|bad| bad.to_string());
        let said = failure.unwrap_or_default();
        assert!(
            said.contains("argv[3]") && said.contains("@family"),
            "{said}"
        );
        assert!(parse("docs/** @text").is_some());
        assert!(parse("docs/** !warn").is_some());
    }

    #[test]
    fn the_set_list_splits_on_the_plus_sign() {
        let rule = parse("*.md ascii+caveman+box");
        let sets = rule.map(|rule| rule.sets).unwrap_or_default();
        assert_eq!(sets, vec!["ascii", "caveman", "box"]);
    }

    #[test]
    fn the_family_suffix_is_kept_as_a_name() {
        let rule = parse("docs/** marks @emoji");
        assert_eq!(
            rule.and_then(|rule| rule.family),
            Some("emoji".to_string())
        );
    }

    #[test]
    fn a_family_needs_no_set_list_beside_it() {
        let rule = parse("* @text");
        let family = rule.as_ref().and_then(|rule| rule.family.clone());
        assert_eq!(family, Some("text".to_string()));
        assert_eq!(rule.map(|rule| rule.sets), Some(Vec::new()));
    }

    #[test]
    fn a_bare_level_is_the_rules_own() {
        let rule = parse("vendor/** any !allow");
        let level = rule.and_then(|rule| rule.default_level);
        assert_eq!(level, Some(Level::Allow));
    }

    #[test]
    fn a_named_level_carries_its_target() {
        let rule = parse("*.md caveman !pedantic=warn");
        let levels = rule.map(|rule| rule.levels).unwrap_or_default();
        let first = levels.first().map(|choice| choice.target.as_str());
        assert_eq!(first, Some("pedantic"));
        assert_eq!(
            levels.first().map(|choice| choice.level),
            Some(Level::Warn)
        );
    }

    #[test]
    fn every_field_may_appear_at_once() {
        let rule = parse("docs/** ascii+marks @emoji !deny !pedantic=allow");
        let rule = rule.unwrap_or_else(|| Rule::bare(String::new(), origin()));
        assert_eq!(rule.sets.len(), 2);
        assert_eq!(rule.family, Some("emoji".to_string()));
        assert_eq!(rule.default_level, Some(Level::Deny));
        assert_eq!(rule.levels.len(), 1);
    }

    #[test]
    fn a_second_set_list_is_refused() {
        assert!(parse_rule("*.md caveman box", origin()).is_err());
    }

    #[test]
    fn a_second_family_is_refused() {
        assert!(parse_rule("*.md @text @emoji", origin()).is_err());
    }

    #[test]
    fn an_unknown_level_is_refused_rather_than_ignored() {
        assert!(parse_rule("*.md caveman !loud", origin()).is_err());
    }

    #[test]
    fn an_empty_set_name_is_refused() {
        assert!(parse_rule("*.md caveman+", origin()).is_err());
    }

    #[test]
    fn a_parse_error_names_its_origin() {
        let failure = parse_rule("*.md !loud", Origin::Argument { index: 4 });
        let shown = failure.err().map(|error| error.to_string());
        assert_eq!(shown, Some("argv[4]: unknown level `loud`".to_string()));
    }
}
