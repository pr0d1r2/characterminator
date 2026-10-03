//! Composition: a set member may name another set (V25).
//!
//! This is what makes a user preset a COMPOSITION rather than a copy --
//! `spec caveman box marks` names three sets instead of restating their
//! members -- and it is why a set can reach itself. A cycle is an error
//! that names the cycle, reported by the CLI as a usage failure, exit 2.
//!
//! See `src/charset/SPEC.md`.

use super::{CharRange, CharSet, SetDefinition, SetMember};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Why a name could not be resolved into a set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComposeError {
    /// A member named a set nothing declares.
    ///
    /// An error rather than an empty set: a misspelled preset would
    /// otherwise grant nothing and read as a file that simply needed no
    /// extra characters, which is the one mistake this tool cannot afford
    /// to make quietly.
    Unknown(String),
    /// A set reaches itself.
    ///
    /// Holds the cycle in the order it was walked, with the repeated name
    /// at both ends, so the message can show the whole loop rather than
    /// only the name it happened to notice twice.
    Cycle(Vec<String>),
}

impl fmt::Display for ComposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => {
                write!(f, "no set named '{name}' is declared")
            }
            Self::Cycle(names) => {
                write!(f, "set composition cycle: {}", names.join(" -> "))
            }
        }
    }
}

impl std::error::Error for ComposeError {}

/// Every declared set, by name.
///
/// A sorted map, not a hash map: `ctrm sets` lists what is declared, and a
/// listing whose order changes between runs of an offline, deterministic
/// tool is a diff nobody asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetCatalog {
    defs: BTreeMap<String, SetDefinition>,
}

/// One resolution in progress.
///
/// `path` is the chain currently being expanded, so a name met while still
/// on it closes a cycle. `done` is every name already fully expanded.
///
/// Skipping a name in `done` is safe ONLY because sets compose by union
/// and union is idempotent (V3): its members are already in `ranges`, so a
/// second visit could add nothing. The same shortcut under a subtraction
/// operator would be wrong, which is one more reason V3 says union only.
struct Walk<'a> {
    path: Vec<String>,
    done: BTreeSet<String>,
    ranges: Vec<CharRange>,
    /// The fidelity in force, which decides whether a labelled member is
    /// granted (V41). Carried on the walk rather than passed down, so a
    /// set reached through three others is judged by the same family as
    /// the one the rule named.
    family: &'a str,
}

impl<'a> Walk<'a> {
    /// A walk at one fidelity.
    fn at(family: &'a str) -> Self {
        Self {
            path: Vec::new(),
            done: BTreeSet::new(),
            ranges: Vec::new(),
            family,
        }
    }

    /// The cycle closed by arriving at `name`, or `None` if there is none.
    fn cycle_from(&self, name: &str) -> Option<Vec<String>> {
        let at = self.path.iter().position(|seen| seen == name)?;
        let mut cycle = self.path.get(at..)?.to_vec();
        cycle.push(name.to_owned());
        Some(cycle)
    }
}

impl SetCatalog {
    /// A catalog declaring nothing.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Declare a set, replacing any earlier declaration of the same name.
    ///
    /// Replacing is what `src/rules:V19` means by "set per name, later
    /// wins": the caller inserts in precedence order, lowest first, and
    /// this node holds no opinion about what that order is.
    pub(crate) fn insert(&mut self, definition: SetDefinition) {
        self.defs.insert(definition.name.clone(), definition);
    }

    /// The declaration behind a name, unresolved.
    #[must_use]
    pub(crate) fn get(&self, name: &str) -> Option<&SetDefinition> {
        self.defs.get(name)
    }

    /// Every declared name, in a stable order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.defs.keys().map(String::as_str)
    }

    /// Resolve one name into the set of code points it grants, at one
    /// fidelity.
    ///
    /// The family is a parameter rather than a property of the catalog
    /// because two paths in one run may resolve the SAME preset at
    /// different fidelities (`src/rules:V29`), and a catalog that held one
    /// family would have to be rebuilt per path to say so.
    ///
    /// # Errors
    ///
    /// [`ComposeError::Unknown`] if the name or any name it reaches is not
    /// declared, [`ComposeError::Cycle`] if it reaches itself.
    pub fn resolve(
        &self,
        name: &str,
        family: &str,
    ) -> Result<CharSet, ComposeError> {
        let mut walk = Walk::at(family);
        self.expand(name, &mut walk)?;
        Ok(CharSet::new(name.to_owned(), walk.ranges))
    }

    /// Resolve several names into their union, under one name (V3).
    ///
    /// This is the effective set of a rule that grants more than one set.
    /// One walk covers them all, so a cycle is caught across the whole
    /// union rather than only within one branch of it.
    ///
    /// # Errors
    ///
    /// As [`SetCatalog::resolve`].
    pub fn resolve_union(
        &self,
        name: &str,
        members: &[String],
        family: &str,
    ) -> Result<CharSet, ComposeError> {
        let mut walk = Walk::at(family);
        for member in members {
            self.expand(member, &mut walk)?;
        }
        Ok(CharSet::new(name.to_owned(), walk.ranges))
    }

    /// Begin expanding `name`, or report why it cannot be expanded.
    ///
    /// `Ok(None)` means it was already expanded and there is nothing left
    /// to do.
    fn enter<'a>(
        &'a self,
        name: &str,
        walk: &mut Walk,
    ) -> Result<Option<&'a SetDefinition>, ComposeError> {
        if walk.done.contains(name) {
            return Ok(None);
        }
        if let Some(cycle) = walk.cycle_from(name) {
            return Err(ComposeError::Cycle(cycle));
        }
        let found = self
            .defs
            .get(name)
            .ok_or_else(|| ComposeError::Unknown(name.to_owned()))?;
        walk.path.push(name.to_owned());
        Ok(Some(found))
    }

    /// Add everything `name` grants, directly or through other sets.
    fn expand(&self, name: &str, walk: &mut Walk) -> Result<(), ComposeError> {
        let Some(definition) = self.enter(name, walk)? else {
            return Ok(());
        };
        for member in &definition.members {
            self.absorb(member, walk)?;
        }
        walk.path.pop();
        walk.done.insert(name.to_owned());
        Ok(())
    }

    /// Add one member: a character and a span go straight in, a name is
    /// followed, and a labelled one is weighed against the fidelity.
    fn absorb(
        &self,
        member: &SetMember,
        walk: &mut Walk,
    ) -> Result<(), ComposeError> {
        match member {
            SetMember::Literal(point) => {
                walk.ranges.push(CharRange::single(*point));
                Ok(())
            }
            SetMember::Range(range) => {
                walk.ranges.push(*range);
                Ok(())
            }
            SetMember::Named(other) => self.expand(other, walk),
            SetMember::Labelled { family, member } => {
                self.absorb_labelled(family, member, walk)
            }
        }
    }

    /// A labelled member is granted at ITS family and nowhere else (V41).
    ///
    /// No walk up the family tree: that tree is declared in the map
    /// (`src/fix:V27`), so following it here would make this node depend
    /// on `src/fix`. A file wanting both spellings grants the variants
    /// explicitly, which is what `src/rules:V29` already says to do.
    fn absorb_labelled(
        &self,
        family: &str,
        member: &SetMember,
        walk: &mut Walk,
    ) -> Result<(), ComposeError> {
        if family == walk.family {
            self.absorb(member, walk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ComposeError, SetCatalog};
    use crate::charset::parse_line;

    /// The default fidelity (`src/rules:V29`), spelled here rather than
    /// imported: this node treats a family as an opaque name (V41).
    const TEXT: &str = "text";

    /// Build a catalog from lines of the sets grammar, in the order given.
    fn catalog(lines: &[&str]) -> SetCatalog {
        let mut built = SetCatalog::new();
        for line in lines {
            if let Ok(Some(definition)) = parse_line(line) {
                built.insert(definition);
            }
        }
        built
    }

    #[test]
    fn a_flat_set_resolves_to_its_members() {
        let sets = catalog(&["dash U+2014 U+2013"]);
        let holds = sets
            .resolve("dash", TEXT)
            .map(|set| set.contains('\u{2014}'));
        assert_eq!(holds, Ok(true));
    }

    #[test]
    fn a_member_may_name_another_set() {
        let sets = catalog(&["dash U+2014", "spec dash U+00A7"]);
        let resolved = sets.resolve("spec", TEXT);
        let holds = resolved.map(|set| set.contains('\u{2014}'));
        assert_eq!(holds, Ok(true));
    }

    #[test]
    fn composition_reaches_through_several_sets() {
        let sets = catalog(&["aa U+2014", "bb aa", "cc bb"]);
        let holds =
            sets.resolve("cc", TEXT).map(|set| set.contains('\u{2014}'));
        assert_eq!(holds, Ok(true));
    }

    #[test]
    fn a_shared_set_is_not_a_cycle() {
        let lines = ["aa bb cc", "bb dd", "cc dd", "dd U+2014"];
        let sets = catalog(&lines);
        let spans = sets.resolve("aa", TEXT).map(|set| set.ranges.len());
        assert_eq!(spans, Ok(1));
    }

    #[test]
    fn an_undeclared_member_is_an_error() {
        let sets = catalog(&["spec missing"]);
        let err = Err(ComposeError::Unknown("missing".to_owned()));
        assert_eq!(sets.resolve("spec", TEXT), err);
    }

    #[test]
    fn an_undeclared_name_is_an_error() {
        let sets = catalog(&["spec U+2014"]);
        let err = Err(ComposeError::Unknown("other".to_owned()));
        assert_eq!(sets.resolve("other", TEXT), err);
    }

    #[test]
    fn a_set_naming_itself_is_a_cycle() {
        let sets = catalog(&["loop loop"]);
        let cycle = vec!["loop".to_owned(), "loop".to_owned()];
        assert_eq!(sets.resolve("loop", TEXT), Err(ComposeError::Cycle(cycle)));
    }

    #[test]
    fn an_indirect_cycle_names_every_set_in_it() {
        let sets = catalog(&["aa bb", "bb cc", "cc aa"]);
        let names = ["aa", "bb", "cc", "aa"].map(str::to_owned).to_vec();
        assert_eq!(sets.resolve("aa", TEXT), Err(ComposeError::Cycle(names)));
    }

    #[test]
    fn the_cycle_message_names_the_cycle() {
        let sets = catalog(&["aa bb", "bb cc", "cc aa"]);
        let message = sets.resolve("aa", TEXT).err().map(|err| err.to_string());
        let expected = "set composition cycle: aa -> bb -> cc -> aa";
        assert_eq!(message, Some(expected.to_owned()));
    }

    #[test]
    fn a_cycle_below_a_sound_set_is_still_caught() {
        let sets = catalog(&["aa U+2014 bb", "bb cc", "cc bb"]);
        let names = ["bb", "cc", "bb"].map(str::to_owned).to_vec();
        assert_eq!(sets.resolve("aa", TEXT), Err(ComposeError::Cycle(names)));
    }

    #[test]
    fn a_union_grants_every_named_set() {
        let sets = catalog(&["dash U+2014", "legal U+00A9"]);
        let names = ["dash".to_owned(), "legal".to_owned()];
        let union = sets.resolve_union("both", &names, TEXT);
        assert_eq!(union.map(|set| set.ranges.len()), Ok(2));
    }

    #[test]
    fn a_union_of_no_names_grants_nothing() {
        let sets = catalog(&["dash U+2014"]);
        let union = sets.resolve_union("none", &[], TEXT);
        assert_eq!(union.map(|set| set.is_empty()), Ok(true));
    }

    #[test]
    fn a_later_declaration_replaces_an_earlier_one() {
        let sets = catalog(&["dash U+2014", "dash U+00A9"]);
        let holds = sets
            .resolve("dash", TEXT)
            .map(|set| set.contains('\u{2014}'));
        assert_eq!(holds, Ok(false));
    }

    #[test]
    fn names_are_listed_in_a_stable_order() {
        let sets = catalog(&["zz U+2014", "aa U+00A9"]);
        let listed = sets.names().collect::<Vec<_>>();
        assert_eq!(listed, vec!["aa", "zz"]);
    }

    #[test]
    fn an_undeclared_name_has_a_message_naming_it() {
        let sets = SetCatalog::new();
        let message =
            sets.resolve("gone", TEXT).err().map(|err| err.to_string());
        assert_eq!(message, Some("no set named 'gone' is declared".to_owned()));
    }
}
