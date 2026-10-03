//! Which rule WINS for a path, and what that rule grants it.

use crate::lint::Level;
use crate::rules::{ASCII, LevelChoice, Rule, Sourced, TEXT};

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
pub(crate) trait PathMatcher {
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
pub(crate) struct Resolution<'a> {
    /// The set names granted, which a caller resolves against the charset
    /// node. Never empty: with no matching rule it is `ascii` (V1).
    pub sets: Vec<String>,
    /// Levels chosen for named lints and groups, later matching rules
    /// overriding earlier ones per target.
    pub levels: Vec<LevelChoice>,
    /// The level of the last matching rule that set a bare one.
    pub default_level: Option<Level>,
    /// The fidelity family, as a NAME. Never empty: `text` by default.
    pub family: String,
    /// The last matching rule that NAMED a set, or none. A rule that only
    /// moves levels (`docs/** !warn`) does not compete for the grant: it
    /// says how loud, not what is allowed, and letting it win would
    /// quietly narrow the path back to `ascii` (B5, V56).
    pub winner: Option<&'a Rule>,
    /// The last matching rule that NAMED a family, which is not always
    /// the winner: a rule may grant sets without expressing a
    /// preference, and it leaves the standing choice alone.
    pub fidelity: Option<&'a Rule>,
    /// `levels`, each with the origin of the line that set it last (V20).
    pub sourced: Vec<Sourced<'a, LevelChoice>>,
    /// The last matching rule that set a bare level: `default_level`'s
    /// line, which need not be the winner either (V56).
    pub leveller: Option<&'a Rule>,
}

/// Resolve `path` against `rules`, in the order the rules were assembled.
///
/// Later matching line wins (V2, the gitignore reading), so the search
/// runs FORWARD and keeps overwriting instead of stopping at the first
/// hit. That is what makes a per-file line placed after a per-type line
/// override it, with no notion of specificity to argue about.
pub(crate) fn resolve<'a, M: PathMatcher + ?Sized>(
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
    levels: Vec<Sourced<'a, LevelChoice>>,
    leveller: Option<&'a Rule>,
    fidelity: Option<&'a Rule>,
}

impl<'a> Found<'a> {
    /// Take in one matching rule. Every field is overwritten rather than
    /// merged, which IS the last-match-wins rule (V2) -- per field: the
    /// grant goes to the last rule that named a set, exactly as fidelity
    /// goes to the last rule that named a family (V56).
    fn absorb(&mut self, rule: &'a Rule) {
        if !rule.sets.is_empty() {
            self.winner = Some(rule);
        }
        let bare = rule.default_level.map(|_| rule);
        self.leveller = bare.or(self.leveller);
        self.fidelity = rule.family.as_ref().map(|_| rule).or(self.fidelity);
        merge_levels(&mut self.levels, rule);
    }

    fn finish(self) -> Resolution<'a> {
        Resolution {
            sets: granted(self.winner),
            levels: self.levels.iter().map(|l| l.value.clone()).collect(),
            default_level: self.leveller.and_then(|r| r.default_level),
            family: family_named(self.fidelity),
            winner: self.winner,
            fidelity: self.fidelity,
            sourced: self.levels,
            leveller: self.leveller,
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

/// The fidelity family in force, as a NAME.
///
/// `text` when no matching rule named one (V29). The name is carried,
/// never resolved: what a family CONTAINS, and which member of an
/// equivalence class it prefers, is `src/fix:V27`, and a name this node
/// does not recognise is not this node's error to raise.
fn family_named(source: Option<&Rule>) -> String {
    let named = source.and_then(|rule| rule.family.as_deref());
    named.unwrap_or(TEXT).to_string()
}

/// Fold one rule's level choices into those already in force.
///
/// Per TARGET rather than per rule: a later rule that speaks about one
/// lint says nothing about the others, in the same way V19 has a later
/// map entry win per character rather than per file.
///
/// Each choice keeps the origin of the line that set it, so a later rule
/// overriding one level takes over its origin too (V20).
fn merge_levels<'a>(into: &mut Vec<Sourced<'a, LevelChoice>>, from: &'a Rule) {
    let origin = &from.origin;
    for choice in &from.levels {
        let held = into.iter_mut().find(|h| h.value.target == choice.target);
        match held {
            Some(held) => {
                held.value.level = choice.level;
                held.origin = origin;
            }
            None => into.push(Sourced {
                value: choice.clone(),
                origin,
            }),
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

    /// B5: `*.bat !crlf=allow` after `* ascii+cr` narrowed `.bat` files
    /// back to `ascii`, so the exemption line itself made CR a violation.
    #[test]
    fn a_level_only_line_leaves_the_grant_alone() {
        let rules = rules(&["* caveman", "docs/* !warn"]);
        let resolution = resolved("docs/a.md", &rules);
        let want = vec!["ascii".to_string(), "caveman".to_string()];
        assert_eq!(resolution.sets, want);
        assert_eq!(resolution.default_level, Some(Level::Warn));
        let winner = resolution.winner.map(|rule| rule.pattern.as_str());
        assert_eq!(winner, Some("*"));
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

    /// B28: the line that set a level is named even when another line
    /// won the grant, and a later override takes over the origin.
    #[test]
    fn every_level_keeps_the_origin_of_the_line_that_set_it() {
        let lines =
            &["* caveman !pedantic=warn", "src/* !pedantic=allow !warn"];
        let rules = rules(lines);
        let found = resolved("src/main.rs", &rules);
        let at: Vec<&Origin> = found.sourced.iter().map(|l| l.origin).collect();
        assert_eq!(at, vec![&Origin::Argument { index: 1 }]);
        let bare = found.leveller.map(|rule| rule.origin.clone());
        assert_eq!(bare, Some(Origin::Argument { index: 1 }));
        let winner = found.winner.map(|rule| rule.origin.clone());
        assert_eq!(winner, Some(Origin::Argument { index: 0 }));
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
    fn the_default_family_is_text() {
        assert_eq!(resolved("x", &[]).family, "text");
        let rules = rules(&["* caveman"]);
        assert_eq!(resolved("x", &rules).family, "text");
    }

    #[test]
    fn the_last_matching_rule_naming_a_family_wins() {
        let rules = rules(&["* @text", "docs/* marks @emoji"]);
        assert_eq!(resolved("docs/a.md", &rules).family, "emoji");
    }

    #[test]
    fn a_later_rule_naming_no_family_leaves_the_choice_alone() {
        let rules = rules(&["docs/* marks @emoji", "docs/* box"]);
        let resolution = resolved("docs/a.md", &rules);
        assert_eq!(resolution.family, "emoji");
        let sets = resolution.sets;
        assert_eq!(sets, vec!["ascii".to_string(), "box".to_string()]);
    }

    #[test]
    fn explain_can_name_the_rule_that_chose_the_family() {
        let rules = rules(&["* @text", "docs/* @emoji"]);
        let chosen = resolved("docs/a.md", &rules).fidelity;
        assert_eq!(chosen.map(|rule| rule.pattern.as_str()), Some("docs/*"));
    }

    #[test]
    fn an_unknown_family_passes_through_as_a_name() {
        let rules = rules(&["* @nerd"]);
        assert_eq!(resolved("x", &rules).family, "nerd");
    }

    #[test]
    fn a_family_named_by_an_unmatched_rule_does_not_apply() {
        let rules = rules(&["docs/* @emoji"]);
        assert_eq!(resolved("src/main.rs", &rules).family, "text");
    }

    #[test]
    fn the_base_survives_a_rule_that_grants_something_else() {
        let rules = rules(&["* caveman", "src/* box"]);
        let sets = resolved("src/main.rs", &rules).sets;
        assert_eq!(sets, vec!["ascii".to_string(), "box".to_string()]);
    }
}
