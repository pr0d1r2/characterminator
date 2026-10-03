//! `explain --as prompt` (T34): the configuration as an instruction an
//! agent can draft against, so text arrives compliant instead of being
//! fixed afterwards (`V32`).
//!
//! DETERMINISTIC, and built from nothing but the configuration: the same
//! run renders the same bytes, so the prompt can be cached, diffed and
//! committed. No model is involved (`.:C`).
//!
//! It says four things, each read from the node that owns it: which
//! characters are allowed (the rules and the sets, resolved), what `fix`
//! would rewrite and to what (the map, asked rather than restated), the
//! fidelity, and the hazards -- which no grant reaches (`src/lint:V34`),
//! so they are said as a flat prohibition.
//!
//! Code points are written `U+XXXX` and never as the characters
//! themselves. A prompt that printed them would itself fail the `ascii`
//! check it is teaching, and the code point is the unambiguous name.

use crate::charset::{CharSet, SetCatalog, builtin};
use crate::cli::config::Config;
use crate::fix::{self as engine, Map};
use crate::render::{Format, sets as render_sets};
use crate::rules::{self, ASCII, Rule, TEXT, describe};

/// What the prompt is about: one path, or the whole repository.
struct Scope {
    /// How the prompt names what it covers.
    label: String,
    /// One line per grant in force.
    grants: Vec<String>,
    /// The sets whose members are listed, in first-named order.
    names: Vec<String>,
    /// The family labelled members resolve at.
    family: String,
    /// How the prompt states the fidelity.
    fidelity: String,
    /// What `fix` judges a replacement against.
    allowed: CharSet,
}

/// The whole prompt.
///
/// # Errors
///
/// A configuration that does not parse, a set that does not resolve, or
/// a map rewrite the engine refuses to trust.
pub(super) fn render(
    config: &Config,
    path: Option<&str>,
) -> Result<String, String> {
    let rules = config.rules()?;
    let catalog = config.catalog()?;
    let scope = match path {
        Some(path) => one_path(path, &rules, &catalog)?,
        None => whole(&rules),
    };
    let mut out = vec![intro(&scope)];
    out.extend(members(&scope, &catalog)?);
    out.extend(replacements(&config.map()?, &scope.allowed)?);
    out.extend(hazards()?);
    Ok(out.join("\n"))
}

fn intro(scope: &Scope) -> String {
    let grants = scope.grants.join("\n");
    format!(
        "Write text that `ctrm check` accepts in {}. Use only the \
         characters the sets below allow; any other character fails the \
         check.\n\nAllowed:\n{grants}\n\nFidelity: {}.",
        scope.label, scope.fidelity
    )
}

/// The scope of one path: the rule that decided its set, and the set.
fn one_path(
    path: &str,
    rules: &[Rule],
    catalog: &SetCatalog,
) -> Result<Scope, String> {
    let found = rules::resolve(path, rules, &rules::matches);
    let names = found.sets.join("+");
    let why = found.winner.map_or_else(
        || String::from("no rule matches, so the default"),
        |rule| format!("`{}` at {}", rule.pattern, describe(&rule.origin)),
    );
    let allowed = catalog.resolve_union(&names, &found.sets, &found.family);
    Ok(Scope {
        label: format!("`{path}`"),
        grants: vec![format!("- {names} ({why})")],
        names: found.sets.clone(),
        family: found.family.clone(),
        fidelity: found.family.clone(),
        allowed: allowed.map_err(|bad| bad.to_string())?,
    })
}

/// The scope of the whole repository: every rule, in order, since the
/// last matching one wins, and fixes judged against the default `ascii`.
///
/// A rule naming no set is left out of the grants: it decides no set
/// (`src/rules:V56`); a family it names is in the fidelity line.
fn whole(rules: &[Rule]) -> Scope {
    let granting: Vec<&Rule> =
        rules.iter().filter(|rule| !rule.sets.is_empty()).collect();
    let mut grants = vec![format!("- a path no pattern matches: {ASCII}")];
    grants.extend(granting.iter().map(|rule| grant_line(rule)));
    grants.push(String::from("(the last matching pattern wins)"));
    Scope {
        names: named(&granting),
        label: String::from("this repository"),
        grants,
        family: String::from(TEXT),
        fidelity: fidelities(rules),
        allowed: builtin::ascii(),
    }
}

/// One rule as a grant line: its pattern and its union.
fn grant_line(rule: &Rule) -> String {
    format!("- `{}`: {}", rule.pattern, granted(rule))
}

/// Every set the granting rules name, `ascii` first, each once.
fn named(granting: &[&Rule]) -> Vec<String> {
    let mut names = vec![String::from(ASCII)];
    for set in granting.iter().flat_map(|rule| &rule.sets) {
        if !names.contains(set) {
            names.push(set.clone());
        }
    }
    names
}

/// The fidelity across the repository: the default, and every pattern
/// that names a family -- a set-less one included, since a family is
/// chosen by the last rule naming one (`src/rules:V29`), not by the rule
/// that decided the set.
fn fidelities(rules: &[Rule]) -> String {
    let named: Vec<String> = rules
        .iter()
        .filter_map(|rule| {
            let family = rule.family.as_ref()?;
            Some(format!("`{}` @{family}", rule.pattern))
        })
        .collect();
    if named.is_empty() {
        return String::from(TEXT);
    }
    format!("{TEXT}; by pattern, last match wins: {}", named.join(", "))
}

/// A rule's grant as a union: `ascii` is the base of every rule
/// (`src/rules:V24`), written once.
fn granted(rule: &Rule) -> String {
    let named = rule.sets.iter().filter(|set| *set != ASCII);
    let all: Vec<&str> = [ASCII]
        .into_iter()
        .chain(named.map(String::as_str))
        .collect();
    all.join("+")
}

/// Every named set and its members, as `sets` lists them.
fn members(scope: &Scope, catalog: &SetCatalog) -> Result<Vec<String>, String> {
    let resolved: Result<Vec<CharSet>, _> = scope
        .names
        .iter()
        .map(|name| catalog.resolve(name, &scope.family))
        .collect();
    let listed =
        render_sets(Format::Human, &resolved.map_err(|e| e.to_string())?);
    Ok(vec![
        String::from("\nSet members (U+XXXX, ranges inclusive):"),
        listed,
    ])
}

/// Every source the map would rewrite under `allowed`, and what to write
/// instead. The map is ASKED, through the same engine `fix` runs, so the
/// list cannot drift from what `fix` does: classes, families and the
/// word spacing included. The replacement is written as a quoted,
/// escaped string, so a quote, a space and a deletion all read plainly.
fn replacements(map: &Map, allowed: &CharSet) -> Result<Vec<String>, String> {
    let mut out = vec![String::from(
        "\nInstead of these, write the replacement (what `ctrm fix` would do):",
    )];
    let (sequences, listed): (Vec<_>, Vec<_>) = sources(map)
        .into_iter()
        .partition(|source| builtin_sequence(map, source));
    for source in listed {
        out.extend(replacement(map, allowed, &source)?);
    }
    if !sequences.is_empty() {
        out.push(String::from(EMOJI_SEQUENCES));
    }
    Ok(out)
}

/// One source's line, or none when `fix` would leave it as it is.
fn replacement(
    map: &Map,
    allowed: &CharSet,
    source: &str,
) -> Result<Option<String>, String> {
    let fixed = engine::fix(source, map, |c| allowed.contains(c));
    let fixed = fixed.map_err(|bad| format!("{}: {bad}", points(source)))?;
    Ok((fixed.output != source)
        .then(|| format!("{} -> {:?}", points(source), fixed.output)))
}

/// The builtin emoji sequences (`src/fix:V62`), said as ONE line rather
/// than listed: they run to nearly two thousand, and most targets are
/// emoji, which a prompt kept to ASCII could only spell as code points.
const EMOJI_SEQUENCES: &str = "Emoji sequences (keycap, flag, ZWJ) become \
    their digit, their region code (`PL`), or one emoji.";

/// Whether `source` is a sequence the BUILTIN map declares. The
/// hand-written builtin holds single code points only (V26, V60), so a
/// longer builtin source is one of the generated emoji sequences.
fn builtin_sequence(map: &Map, source: &str) -> bool {
    source.chars().nth(1).is_some()
        && map.entries().iter().any(|entry| {
            entry.from == source
                && matches!(entry.origin, rules::Origin::Builtin { .. })
        })
}

/// Every source text the map declares, once: its entries, then the
/// members of its classes.
fn sources(map: &Map) -> Vec<String> {
    let entries = map.entries().iter().map(|entry| entry.from.clone());
    let classes = map.classes().iter().flat_map(|class| &class.members);
    let all = entries.chain(classes.map(|member| member.text.clone()));
    let mut once: Vec<String> = Vec::new();
    for source in all {
        if !once.contains(&source) {
            once.push(source);
        }
    }
    once
}

/// A source as code points, `+` between the points of a sequence, the
/// spelling the map grammar itself uses.
fn points(text: &str) -> String {
    let each: Vec<String> = text
        .chars()
        .map(|c| format!("U+{:04X}", u32::from(c)))
        .collect();
    each.join("+")
}

/// The hazard classes, listed from the compiled-in data the check itself
/// reads (`src/lint:V34`), so the prohibition cannot drift from the rule.
fn hazards() -> Result<Vec<String>, String> {
    let catalog = builtin::hazard_catalog().map_err(|bad| bad.to_string())?;
    let classes: Result<Vec<CharSet>, _> = catalog
        .names()
        .filter(|name| *name != "hazard")
        .map(|name| catalog.resolve(name, TEXT))
        .collect();
    let listed =
        render_sets(Format::Human, &classes.map_err(|e| e.to_string())?);
    Ok(vec![
        String::from(
            "\nNever write these, whatever a set allows; they always fail:",
        ),
        listed,
        String::from("(U+FEFF only as the very first character of a file)"),
    ])
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::cli::config::{Config, from_argv};
    use std::path::Path;

    fn loaded(words: &[&str]) -> Config {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let words: Vec<String> = ["explain", "--no-files"]
            .iter()
            .chain(words)
            .map(|w| (*w).to_owned())
            .collect();
        from_argv(root, &words).unwrap_or_default()
    }

    fn prompt(words: &[&str], path: Option<&str>) -> String {
        render(&loaded(words), path).unwrap_or_else(|why| why)
    }

    /// The same configuration renders the same bytes, every time.
    #[test]
    fn the_prompt_is_deterministic() {
        let words = ["--rule", "*.md caveman", "--map", "use words"];
        assert_eq!(prompt(&words, None), prompt(&words, None));
        assert_eq!(prompt(&words, Some("a.md")), prompt(&words, Some("a.md")));
    }

    /// For one path: the set in force, the rule and origin behind it, its
    /// members, and the fidelity.
    #[test]
    fn a_path_prompt_names_its_sets_rule_and_fidelity() {
        let said = prompt(&["--rule", "*.md caveman @emoji"], Some("a.md"));
        assert!(
            said.contains("- ascii+caveman (`*.md` at argv[4])"),
            "{said}"
        );
        assert!(said.contains("\ncaveman U+"), "{said}");
        assert!(said.contains("Fidelity: emoji."), "{said}");
    }

    /// The replacements are what `fix` does, asked of the engine: the em
    /// dash becomes two hyphens under `ascii`, and the opt-in words map
    /// appears only once asked for.
    #[test]
    fn the_replacements_are_what_fix_would_do() {
        let plain = prompt(&[], None);
        assert!(plain.contains("U+2014 -> \"--\""), "{plain}");
        assert!(!plain.contains("U+22A5"), "{plain}");
        let words = prompt(&["--map", "use words"], None);
        assert!(words.contains("U+22A5 -> \"not\""), "{words}");
    }

    /// A character the path's set grants is not "replaced": the prompt
    /// tells the agent to write it, not to avoid it.
    #[test]
    fn a_granted_character_is_not_listed_as_a_replacement() {
        let said = prompt(&["--rule", "*.md typography"], Some("a.md"));
        assert!(!said.contains("U+2014 ->"), "{said}");
        assert!(said.contains("typography"), "{said}");
    }

    /// The hazard rule is stated whatever the grant, even under `any`.
    #[test]
    fn the_hazards_are_always_forbidden() {
        let said = prompt(&["--rule", "* any"], Some("a.md"));
        assert!(said.contains("Never write these"), "{said}");
        assert!(said.contains("hazard-bidi U+"), "{said}");
        assert!(said.contains("U+202A-U+202E"), "{said}");
    }

    /// The prompt is ASCII, so it passes the check it teaches.
    #[test]
    fn the_prompt_is_ascii() {
        let said =
            prompt(&["--rule", "*.md caveman", "--map", "use words"], None);
        assert!(said.is_ascii());
        assert!(said.contains("- `*.md`: ascii+caveman"), "{said}");
    }
}
