//! `explain --as prompt` (T34): the configuration as an instruction an
//! agent can draft against, so text arrives compliant instead of being
//! fixed afterwards (`V32`, V129).
//!
//! DETERMINISTIC, and built from nothing but the configuration: the same
//! run renders the same bytes, so the prompt can be cached, diffed and
//! committed. No model is involved (`.:C`).
//!
//! It says what is allowed (the rules and sets, resolved), what to write
//! instead of what a writer plausibly types (the map, asked rather than
//! restated), the fidelity, the hazards -- which no grant reaches
//! (`src/lint/hazard:V34`), so they are one flat prohibition -- and the
//! pedantic lints when they are on.
//!
//! Code points are written `U+XXXX`. The ONE exception is the members line
//! of a small set, which shows each glyph beside its code point: a model
//! writing Polish has to see that the letters are allowed, and the prompt
//! is OUTPUT, judged by whoever stores it, not source this repo's ASCII
//! rule governs. Every other line stays ASCII.

use super::prompt_lines::{hazards, members, pedantic, replacements};
use crate::charset::{CharSet, SetCatalog, builtin};
use crate::judge::{Checker, Config};
use crate::rules::{self, ASCII, Origin, Rule, TEXT, describe};

/// What the prompt is about: one path, or the whole repository.
pub(super) struct Scope {
    /// The path asked about, `""` for the whole repository.
    pub path: String,
    /// How the prompt names what it covers.
    pub label: String,
    /// One line per grant in force.
    pub grants: Vec<String>,
    /// The sets whose members are listed, in first-named order.
    pub names: Vec<String>,
    /// The family labelled members resolve at.
    pub family: String,
    /// How the prompt states the fidelity.
    pub fidelity: String,
    /// What `fix` judges a replacement against.
    pub allowed: CharSet,
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
    out.push(hazards(&scope.allowed));
    let (_, levels) = Checker::configured(config)?.shared_law(&scope.path)?;
    out.extend(pedantic(&levels));
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
    let why = found.winner.map_or_else(no_rule, decided_by);
    let allowed = catalog.resolve_union(&names, &found.sets, &found.family);
    Ok(Scope {
        path: path.to_owned(),
        label: format!("`{path}`"),
        grants: vec![format!("- {names}{why}")],
        names: found.sets.clone(),
        family: found.family.clone(),
        fidelity: found.family.clone(),
        allowed: allowed.map_err(|bad| bad.to_string())?,
    })
}

fn no_rule() -> String {
    String::from(" (no rule matches, so the default)")
}

/// The rule behind a grant: its pattern, and the file line when there is
/// one. An argv position (`argv[6]`) is left out: it names a word of a
/// command line the model never saw, so it means nothing to it.
fn decided_by(rule: &Rule) -> String {
    match rule.origin {
        Origin::File { .. } => {
            format!(" (`{}` at {})", rule.pattern, describe(&rule.origin))
        }
        Origin::Argument { .. } | Origin::Builtin { .. } => {
            format!(" (pattern `{}`)", rule.pattern)
        }
    }
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
        path: String::new(),
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

#[cfg(test)]
#[path = "prompt_test.rs"]
mod tests;
