//! The configuration a run is judged by, and its assembly.
//!
//! The three kinds each travel as a `Sources` -- the precedence chain
//! `src/rules:V19` fixes -- and nothing is parsed until a judge asks. HOW
//! a `Config` is filled (argv, dotfile discovery) is `src/cli`'s; what it
//! assembles into -- rules, a catalog, a map -- and whether it is valid at
//! all (V74) is this node's, so every verb and the guard read the same
//! answer from the same bytes.

use crate::charset::{self, SetCatalog, SetDefinition, builtin};
use crate::fix::{self as engine, Map};
use crate::rules::{self, Place, Rule, Sources};
use std::path::PathBuf;

/// One run's configuration, unparsed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Config {
    /// The run root: `-C <dir>` resolved against the working directory.
    pub(crate) root: PathBuf,
    pub(crate) rules: Sources,
    pub(crate) sets: Sources,
    pub(crate) map: Sources,
    /// `--strict`: warn counts as deny (`src/lint:V36`). A flag with no
    /// line twin, because a level is per rule and this is per RUN.
    pub(crate) strict: bool,
}

impl Config {
    /// Parse all three kinds once, whatever the verb will read (V74).
    ///
    /// A verb reads only the kinds it needs, so before this a broken map
    /// was refused by `fix` and `stats` and silently ignored by `check`,
    /// which then reported on a run whose configuration did not parse
    /// (B29). A configuration is one thing, and it is valid or it is not.
    ///
    /// # Errors
    ///
    /// The first line of any kind that does not parse, at its origin.
    pub(crate) fn validate(&self) -> Result<(), String> {
        let catalog = self.catalog()?;
        let map = self.map()?;
        for rule in self.rules()? {
            declared(&map, &rule)?;
            granted(&catalog, &rule)?;
        }
        Ok(())
    }

    /// The rules, in precedence order.
    ///
    /// # Errors
    ///
    /// A line that does not parse, named at its origin.
    pub(crate) fn rules(&self) -> Result<Vec<Rule>, String> {
        self.rules.rules().map_err(|bad| bad.to_string())
    }

    /// The sets a run resolves against: the intrinsic `ascii`, then every
    /// declared set in precedence order, a later one replacing an earlier
    /// one of the same name (`src/rules:V19`).
    ///
    /// A repo whose files hold characters no preset covers declares a set
    /// of its own rather than reaching for `any`, which is the difference
    /// between a grant somebody wrote down and a check switched off.
    ///
    /// # Errors
    ///
    /// A sets line that does not parse, named at its origin.
    ///
    /// The CLDR locale sets come in only for the names the rules use
    /// (`src/charset/locale:V61`), and only while the builtin sets are on.
    pub(crate) fn catalog(&self) -> Result<SetCatalog, String> {
        let mut catalog = self.declared()?;
        if self.sets.has_builtin() {
            let wanted = self.rules()?.into_iter().flat_map(|rule| rule.sets);
            charset::adopt(&mut catalog, wanted)
                .map_err(|bad| bad.to_string())?;
        }
        Ok(catalog)
    }

    /// The same with EVERY locale set in, which is what `sets` lists.
    ///
    /// # Errors
    ///
    /// As [`Config::catalog`].
    pub(crate) fn listing(&self) -> Result<SetCatalog, String> {
        let mut catalog = self.declared()?;
        if self.sets.has_builtin() {
            charset::adopt_all(&mut catalog).map_err(|bad| bad.to_string())?;
        }
        Ok(catalog)
    }

    /// `ascii` and every set the chain declares, without the locales.
    fn declared(&self) -> Result<SetCatalog, String> {
        let declared = self.sets.assemble(set_line);
        let declared = declared.map_err(|bad| bad.to_string())?;
        let mut catalog = builtin::intrinsic_catalog();
        for definition in declared {
            catalog.insert(definition);
        }
        Ok(catalog)
    }

    /// The map in force, each source layered over the last.
    ///
    /// A map is layered as WHOLE texts rather than entry by entry: a
    /// `family` line declares a tree a later line may use. A flag value
    /// still goes through `parse_flag` first, so `--map` refuses a second
    /// line exactly as `--rule` does (`src/rules:V18`).
    ///
    /// # Errors
    ///
    /// A map line that does not parse, or a tree that does not validate,
    /// named at the source it came from.
    pub(crate) fn map(&self) -> Result<Map, String> {
        let mut map = Map::default();
        for (place, text) in self.map.layers() {
            let line = one_line(place, text)?;
            let named = |bad| located(place, &bad);
            map = map.layer(&line, &|n| place.origin(n)).map_err(named)?;
        }
        Ok(map)
    }
}

/// A rule's family is one the map's family tree declares
/// (`src/rules:V29`, `src/fix:V27`). An undeclared one used to pass as a
/// name and resolve to the unlabelled members alone, so `--fidelity
/// emjoi` was a typo that quietly meant something (B30).
fn declared(map: &Map, rule: &Rule) -> Result<(), String> {
    let Some(family) = &rule.family else {
        return Ok(());
    };
    match map.tree().path(family) {
        Ok(_) => Ok(()),
        Err(bad) => Err(format!("{}: {bad}", rules::describe(&rule.origin))),
    }
}

/// Every set a rule names resolves against the catalog (V74), whether
/// or not any file matches the rule. Resolved only when a file matched,
/// `--rule '*.txt asci'` passed in a tree with no `.txt` file, and a
/// refusal that did come named the set but not the rule's origin (B47).
fn granted(catalog: &SetCatalog, rule: &Rule) -> Result<(), String> {
    let family = rule.family.as_deref().unwrap_or(rules::TEXT);
    match catalog.resolve_union(&rule.pattern, &rule.sets, family) {
        Ok(_) => Ok(()),
        Err(bad) => {
            let hint = suggested(catalog, &bad);
            Err(format!("{}: {bad}{hint}", rules::describe(&rule.origin)))
        }
    }
}

/// " (did you mean 'ascii'?)" for an unknown set within two edits of one
/// the run declares (`src/cli/usage:V121`), else nothing.
fn suggested(catalog: &SetCatalog, bad: &charset::ComposeError) -> String {
    let charset::ComposeError::Unknown(name) = bad else {
        return String::new();
    };
    super::nearest(name, catalog.names())
        .map(|near| format!(" (did you mean '{near}'?)"))
        .unwrap_or_default()
}

/// A source's text, or a flag's value once it has passed the one-line
/// rule. Blank and comment values pass through as the nothing they are.
fn one_line(place: Place<'_>, text: &str) -> Result<String, String> {
    let Place::Argument(index) = place else {
        return Ok(text.to_owned());
    };
    let kept = rules::parse_flag(text, index, |line, _| Ok(line.to_owned()));
    let kept = kept.map_err(|bad| bad.to_string())?;
    Ok(kept.unwrap_or_default())
}

/// A map refusal at its place, in the `<file>:<line>: ` shape a `.ctrm`
/// or `.ctrm-sets` refusal has (`src/cli/usage:V121`). A flag is one
/// line, so its origin carries no line number.
fn located(place: Place<'_>, bad: &engine::Error) -> String {
    match (place, bad.line()) {
        (Place::Argument(_), Some(_)) => {
            format!("{}: {}", at(place), bad.reason())
        }
        (_, Some(line)) => format!("{}:{line}: {}", at(place), bad.reason()),
        (_, None) => format!("{}: {bad}", at(place)),
    }
}

/// A source named the way `src/rules:V20` spells an origin, less the
/// line: a map refusal already carries its own line number.
fn at(place: Place<'_>) -> String {
    match place {
        Place::Builtin => String::from("builtin"),
        Place::File(path) => path.display().to_string(),
        Place::Argument(index) => format!("argv[{index}]"),
    }
}

/// One sets line, at the origin the chain gave it.
///
/// The grammar's parser belongs to `src/charset` and the precedence chain
/// to `src/rules`, and neither calls the other: the parser travels as an
/// argument, so this adapter is the one place their two error types meet.
fn set_line(
    line: &str,
    origin: rules::Origin,
) -> Result<SetDefinition, rules::ParseError> {
    match charset::parse_line(line) {
        Ok(Some(declared)) => unreserved(declared, origin),
        // The chain skips blank and comment lines before calling this, so
        // a line declaring nothing cannot arrive here.
        Ok(None) => Err(rules::error(origin, "declares no set")),
        Err(bad) => Err(rules::error(origin, bad.to_string())),
    }
}

/// A declared set may not take a name the tool itself relies on (V116):
/// `ascii`, the base every rule adds; `any`, the explicit opt-out; and the
/// `hazard*` classes. A `.ctrm-sets` line `ascii U+00E9` used to replace
/// the base, and every ASCII byte in the tree became a violation (B68).
/// Every other preset name stays replaceable (`src/rules:V19`), and the
/// builtin data, which DEFINES `any` and the hazard sets, is exempt.
fn unreserved(
    declared: SetDefinition,
    origin: rules::Origin,
) -> Result<SetDefinition, rules::ParseError> {
    let name = declared.name.as_str();
    let reserved =
        name == charset::ASCII || name == "any" || name.starts_with("hazard");
    if !reserved || matches!(origin, rules::Origin::Builtin { .. }) {
        return Ok(declared);
    }
    let why = format!(
        "set `{name}` is reserved and cannot be redeclared \
         (`ascii`, `any` and `hazard*` are the tool's own) -- pick another name"
    );
    Err(rules::error(origin, why))
}
