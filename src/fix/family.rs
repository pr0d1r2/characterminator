//! Character families: the fallback tree resolution walks (V27).
//!
//! A family is a name and its fallback parent, and every chain ends at
//! `ascii`. That is what makes resolution terminate, so a chain that does
//! not reach the root, or that comes back to a name it already used, is a
//! config error rather than a loop nobody notices.
//!
//! The tree is OPEN: a new family is one line plus members in classes.

use crate::fix::{Error, Family};

/// The intrinsic root: the `ascii` set's own name (`src/charset`). It is
/// code rather than data, so a zero-file run still has somewhere for every
/// chain to end (`src/rules:V21`).
pub(crate) use crate::charset::ASCII as ROOT;

/// The declared families. `ascii` is not among them: it is intrinsic, and
/// declaring a parent for it is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tree {
    families: Vec<Family>,
}

impl Default for Tree {
    /// The builtin tree, because V27 names it: `ascii` then `text` then
    /// `emoji`. A map file that mentions `text` should not have to restate
    /// what the spec already fixed.
    fn default() -> Self {
        Self::builtin()
    }
}

impl Tree {
    /// `ascii` then `text` then `emoji`, the tree V27 names.
    #[must_use]
    pub(crate) fn builtin() -> Self {
        let mut tree = Self::empty();
        tree.families.push(child("text", ROOT));
        tree.families.push(child("emoji", "text"));
        tree
    }

    /// No families at all, for a caller assembling its own.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            families: Vec::new(),
        }
    }

    /// Every declared family, in declaration order.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn families(&self) -> &[Family] {
        &self.families
    }

    /// Add a family. A later declaration wins over an earlier one for the
    /// same name, as a later map line does (`src/rules:V19`).
    pub(crate) fn declare(&mut self, family: Family) -> Result<(), Error> {
        if family.name == ROOT && family.parent.is_some() {
            return Err(Error::RootReparented);
        }
        self.families.retain(|held| held.name != family.name);
        self.families.push(family);
        Ok(())
    }

    /// The fallback path from `name` up to `ascii`, `name` first.
    ///
    /// This is the order V28 resolves in, and building it is also how the
    /// tree is validated: an unknown parent, a chain that never reaches the
    /// root, and a cycle are all reported here.
    pub(crate) fn path(&self, name: &str) -> Result<Vec<&str>, Error> {
        let mut chain: Vec<&str> = Vec::new();
        let mut at = name;
        while at != ROOT {
            let family = self.named(at)?;
            unseen(&chain, &family.name)?;
            chain.push(&family.name);
            at = parent_of(family)?;
        }
        chain.push(ROOT);
        Ok(chain)
    }

    /// Every declared family reaches the root, with no cycle on the way.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        for family in &self.families {
            self.path(&family.name)?;
        }
        Ok(())
    }

    fn named(&self, name: &str) -> Result<&Family, Error> {
        self.families
            .iter()
            .find(|family| family.name == name)
            .ok_or_else(|| Error::UnknownFamily {
                name: name.to_owned(),
            })
    }
}

fn child(name: &str, parent: &str) -> Family {
    Family {
        name: name.to_owned(),
        parent: Some(parent.to_owned()),
    }
}

/// A name already on the chain means the parent links have come back on
/// themselves. That is a config error, not a loop to discover at runtime.
fn unseen(chain: &[&str], name: &str) -> Result<(), Error> {
    if chain.contains(&name) {
        return Err(Error::FamilyCycle {
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn parent_of(family: &Family) -> Result<&str, Error> {
    family
        .parent
        .as_deref()
        .ok_or_else(|| Error::UnrootedFamily {
            name: family.name.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::{ROOT, Tree, child};
    use crate::fix::{Error, Family};

    fn with(name: &str, parent: &str) -> Tree {
        let mut tree = Tree::builtin();
        let _ = tree.declare(child(name, parent));
        tree
    }

    #[test]
    fn the_root_is_its_own_whole_path() {
        assert_eq!(Tree::builtin().path(ROOT), Ok(vec![ROOT]));
    }

    #[test]
    fn the_builtin_tree_falls_back_through_text() {
        let want = Ok(vec!["emoji", "text", ROOT]);
        assert_eq!(Tree::builtin().path("emoji"), want);
    }

    #[test]
    fn a_new_family_is_one_declaration() {
        let want = Ok(vec!["nerd", "emoji", "text", ROOT]);
        assert_eq!(with("nerd", "emoji").path("nerd"), want);
    }

    #[test]
    fn a_later_declaration_wins_for_the_same_name() {
        let mut tree = Tree::builtin();
        let _ = tree.declare(child("emoji", ROOT));
        assert_eq!(tree.path("emoji"), Ok(vec!["emoji", ROOT]));
        assert_eq!(tree.families().len(), 2);
    }

    #[test]
    fn an_unknown_parent_is_a_config_error() {
        let name = String::from("nerd");
        let tree = with("marks", "nerd");
        assert_eq!(tree.path("marks"), Err(Error::UnknownFamily { name }));
    }

    #[test]
    fn a_cycle_is_a_config_error_rather_than_a_loop() {
        let mut tree = Tree::empty();
        let _ = tree.declare(child("a", "b"));
        let _ = tree.declare(child("b", "a"));
        let name = String::from("a");
        assert_eq!(tree.path("a"), Err(Error::FamilyCycle { name }));
        assert!(tree.validate().is_err());
    }

    #[test]
    fn a_family_with_no_parent_never_reaches_the_root() {
        let mut tree = Tree::empty();
        let _ = tree.declare(Family {
            name: String::from("loose"),
            parent: None,
        });
        let name = String::from("loose");
        assert_eq!(tree.path("loose"), Err(Error::UnrootedFamily { name }));
    }

    #[test]
    fn the_root_cannot_be_given_a_parent() {
        let mut tree = Tree::builtin();
        assert_eq!(
            tree.declare(child(ROOT, "text")),
            Err(Error::RootReparented)
        );
    }

    #[test]
    fn the_builtin_tree_validates() {
        assert!(Tree::builtin().validate().is_ok());
        assert!(with("nerd", "emoji").validate().is_ok());
    }
}
