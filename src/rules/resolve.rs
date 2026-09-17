//! Which rule WINS for a path, and what that rule grants it.

use crate::lint::Level;
use crate::rules::{ASCII, LevelChoice, Rule};

/// How a pattern is tested against a path.
///
/// Whether this repo takes a glob dependency or writes its own matcher is
/// an OPEN QUESTION in the root spec, and rule resolution does not need
/// the answer: it needs a yes or a no per pattern. So the matcher is a
/// parameter, and either choice drops in without this file changing.
///
/// The one thing resolution DOES require of it: per-type globs and
/// per-file paths share one grammar (V2), so a plain path is a pattern
/// that matches itself.
pub trait PathMatcher {
    fn matches(&self, pattern: &str, path: &str) -> bool;
}

/// Any two-argument predicate is a matcher, so a caller can pass a closure
/// without declaring a type for it.
impl<F: Fn(&str, &str) -> bool> PathMatcher for F {
    fn matches(&self, pattern: &str, path: &str) -> bool {
        self(pattern, path)
    }
}

/// What applies to one path, and WHY.
///
/// The winning rule is kept, not just its effects, because `explain` must
/// be able to name the winner and print the config line behind it (V2),
/// and the rule carries its own origin (V20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution<'a> {
    /// The set names granted, which a caller resolves against the charset
    /// node. Never empty: with no matching rule it is `ascii` (V1).
    pub sets: Vec<String>,
    /// Levels chosen for named lints and groups, later matching rules
    /// overriding earlier ones per target.
    pub levels: Vec<LevelChoice>,
    /// The level of the last matching rule that set a bare one.
    pub default_level: Option<Level>,
    /// The last matching rule, or none when nothing matched.
    pub winner: Option<&'a Rule>,
}

/// Resolve `path` against `rules`, in the order the rules were assembled.
///
/// Later matching line wins (V2, the gitignore reading), so the search
/// runs FORWARD and keeps overwriting instead of stopping at the first
/// hit. That is what makes a per-file line placed after a per-type line
/// override it, with no notion of specificity to argue about.
pub fn resolve<'a, M: PathMatcher + ?Sized>(
    path: &str,
    rules: &'a [Rule],
    matcher: &M,
) -> Resolution<'a> {
    let mut found = Found::default();
    for rule in rules {
        if matcher.matches(&rule.pattern, path) {
            found.absorb(rule);
        }
    }
    found.finish()
}

/// What the walk has seen so far.
///
/// A named accumulator rather than three loose variables: it keeps the
/// "later wins" step in ONE place, where it can be read next to the rule
/// it implements.
#[derive(Default)]
struct Found<'a> {
    winner: Option<&'a Rule>,
    levels: Vec<LevelChoice>,
    default_level: Option<Level>,
}

impl<'a> Found<'a> {
    /// Take in one matching rule. Every field is overwritten rather than
    /// merged, which IS the last-match-wins rule (V2).
    fn absorb(&mut self, rule: &'a Rule) {
        self.winner = Some(rule);
        self.default_level = rule.default_level.or(self.default_level);
        merge_levels(&mut self.levels, &rule.levels);
    }

    fn finish(self) -> Resolution<'a> {
        Resolution {
            sets: granted(self.winner),
            levels: self.levels,
            default_level: self.default_level,
            winner: self.winner,
        }
    }
}

/// The set names in force.
///
/// `ascii` is the implicit BASE of every rule (V24), so `*.md caveman`
/// and `*.md ascii+caveman` are the same grant. A rule therefore cannot
/// make a path stricter than the default by naming a set, which is the
/// point: sets compose by union only (`src/charset:V3`), and a grant
/// that could also take something away would need a second operator and
/// a rule about their order.
///
/// A path that no rule matches gets the base alone (V1): strict by
/// default, an extended set being an explicit grant and never an
/// accident. The name is a constant in code, so both readings hold in a
/// run with no config file at all (V21).
///
/// An explicit `ascii+` stays legal and is not repeated -- a union is
/// not a multiset -- so the two spellings produce the same list, not
/// merely the same membership.
fn granted(winner: Option<&Rule>) -> Vec<String> {
    let named = winner.map(|rule| rule.sets.as_slice()).unwrap_or_default();
    let mut sets = vec![ASCII.to_string()];
    for name in named {
        if !sets.iter().any(|held| held == name) {
            sets.push(name.clone());
        }
    }
    sets
}

/// Fold one rule's level choices into those already in force.
///
/// Per TARGET rather than per rule: a later rule that speaks about one
/// lint says nothing about the others, in the same way V19 has a later
/// map entry win per character rather than per file.
fn merge_levels(into: &mut Vec<LevelChoice>, from: &[LevelChoice]) {
    for choice in from {
        match into.iter_mut().find(|held| held.target == choice.target) {
            Some(held) => held.level = choice.level,
            None => into.push(choice.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Origin;
    use crate::rules::rule_line::parse_rule;

    /// A deliberately crude stand-in for the undecided matcher: a trailing
    /// star matches a prefix, `*` matches everything, anything else is an
    /// exact path. Enough to exercise resolution, and it is a PARAMETER
    /// precisely so the real one can differ.
    fn matches(pattern: &str, path: &str) -> bool {
        match pattern.strip_suffix('*') {
            Some(prefix) => path.starts_with(prefix),
            None => pattern == path,
        }
    }

    fn rules(lines: &[&str]) -> Vec<Rule> {
        let numbered = lines.iter().enumerate();
        numbered
            .filter_map(|(index, line)| {
                parse_rule(line, Origin::Argument { index }).ok()
            })
            .collect()
    }

    fn resolved<'a>(path: &str, rules: &'a [Rule]) -> Resolution<'a> {
        resolve(path, rules, &matches)
    }

    #[test]
    fn an_unmatched_path_gets_ascii() {
        let rules = rules(&["docs/* caveman"]);
        let resolution = resolved("src/main.rs", &rules);
        assert_eq!(resolution.sets, vec!["ascii".to_string()]);
        assert!(resolution.winner.is_none());
    }

    #[test]
    fn with_no_rules_at_all_a_path_still_gets_ascii() {
        assert_eq!(resolved("x", &[]).sets, vec!["ascii".to_string()]);
    }

    #[test]
    fn the_last_matching_line_wins() {
        let rules = rules(&["* caveman", "SPEC.md box"]);
        let sets = resolved("SPEC.md", &rules).sets;
        assert_eq!(sets, vec!["ascii".to_string(), "box".to_string()]);
    }

    #[test]
    fn an_earlier_line_cannot_override_a_later_one() {
        let rules = rules(&["SPEC.md box", "* caveman"]);
        let sets = resolved("SPEC.md", &rules).sets;
        assert_eq!(sets, vec!["ascii".to_string(), "caveman".to_string()]);
    }

    #[test]
    fn explain_can_name_the_winner_and_its_origin() {
        let rules = rules(&["* caveman", "SPEC.md box"]);
        let resolution = resolved("SPEC.md", &rules);
        let winner = resolution.winner.map(|rule| rule.pattern.as_str());
        assert_eq!(winner, Some("SPEC.md"));
        let origin = resolution.winner.map(|rule| rule.origin.clone());
        assert_eq!(origin, Some(Origin::Argument { index: 1 }));
    }

    #[test]
    fn levels_accumulate_across_matching_rules() {
        let rules = rules(&["* caveman !pedantic=warn", "src/* ascii"]);
        let levels = resolved("src/main.rs", &rules).levels;
        assert_eq!(levels.len(), 1);
        assert_eq!(levels.first().map(|held| held.level), Some(Level::Warn));
    }

    #[test]
    fn a_later_rule_overrides_one_level_and_leaves_the_rest() {
        let lines = &["* !pedantic=warn !hazard=deny", "src/* !pedantic=allow"];
        let levels = resolved("src/main.rs", &rules(lines)).levels;
        let pedantic = levels.first().map(|held| held.level);
        assert_eq!(pedantic, Some(Level::Allow));
        assert_eq!(levels.len(), 2);
    }

    #[test]
    fn the_bare_level_comes_from_the_last_rule_that_set_one() {
        let rules = rules(&["* !deny", "src/* ascii"]);
        let level = resolved("src/main.rs", &rules).default_level;
        assert_eq!(level, Some(Level::Deny));
    }

    #[test]
    fn every_rule_grants_ascii_whether_it_says_so_or_not() {
        let implicit = rules(&["docs/* caveman"]);
        let explicit = rules(&["docs/* ascii+caveman"]);
        let sets = resolved("docs/a.md", &implicit).sets;
        assert_eq!(sets, vec!["ascii".to_string(), "caveman".to_string()]);
        assert_eq!(sets, resolved("docs/a.md", &explicit).sets);
    }

    #[test]
    fn an_explicit_ascii_is_not_repeated() {
        let rules = rules(&["docs/* ascii+ascii+box"]);
        let sets = resolved("docs/a.md", &rules).sets;
        assert_eq!(sets, vec!["ascii".to_string(), "box".to_string()]);
    }

    #[test]
    fn a_rule_naming_only_a_family_still_grants_ascii() {
        let rules = rules(&["* @emoji"]);
        assert_eq!(resolved("x", &rules).sets, vec!["ascii".to_string()]);
    }

    #[test]
    fn the_base_survives_a_rule_that_grants_something_else() {
        let rules = rules(&["* caveman", "src/* box"]);
        let sets = resolved("src/main.rs", &rules).sets;
        assert_eq!(sets, vec!["ascii".to_string(), "box".to_string()]);
    }
}
