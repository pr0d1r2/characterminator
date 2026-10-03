//! What a level directive points at.

use crate::lint::{Group, Lint};

/// The thing a rule's level suffix addresses: a whole group, or one named
/// lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Group(Group),
    Lint(Lint),
}

impl Target {
    /// What a BARE `!<level>` suffix addresses: the charset group (V36).
    ///
    /// It lives here rather than in the rules node so that the shorthand
    /// and the lints it moves are stated in one place; the parser hands
    /// over the level and does not have to know which group is meant.
    pub(crate) const BARE: Self = Self::Group(Group::Charset);

    /// The token in `!<lint|group>=<level>`. Groups are tried first, which
    /// costs nothing because no lint may share a group's name.
    pub(crate) fn named(word: &str) -> Option<Self> {
        Group::named(word)
            .map(Self::Group)
            .or_else(|| Lint::named(word).map(Self::Lint))
    }

    /// Whether a directive aimed here speaks about this lint. A group
    /// directive covers every lint in it, including lints registered
    /// later; a lint directive covers exactly one.
    pub(crate) fn covers(self, lint: Lint) -> bool {
        match self {
            Self::Group(group) => group == lint.group,
            Self::Lint(named) => named.name == lint.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Target;
    use crate::lint::{Group, Lint};

    fn lint(name: &'static str) -> Lint {
        Lint::named(name).unwrap_or(Lint {
            name,
            group: Group::Pedantic,
        })
    }

    #[test]
    fn a_bare_level_suffix_addresses_the_charset_group() {
        assert_eq!(Target::BARE, Target::Group(Group::Charset));
    }

    #[test]
    fn a_token_names_a_group_or_a_lint() {
        let hazard = Target::Group(Group::Hazard);
        assert_eq!(Target::named("hazard"), Some(hazard));
        let crlf = Target::Lint(lint("crlf"));
        assert_eq!(Target::named("crlf"), Some(crlf));
    }

    #[test]
    fn an_unknown_token_addresses_nothing() {
        assert_eq!(Target::named("charsets"), None);
        assert_eq!(Target::named("outside_set"), None);
    }

    #[test]
    fn a_group_covers_every_lint_in_it_and_no_other() {
        let group = Target::Group(Group::Pedantic);
        assert!(group.covers(lint("crlf")));
        assert!(!group.covers(lint("outside-set")));
    }

    #[test]
    fn a_lint_target_covers_exactly_one_lint() {
        let one = Target::Lint(lint("crlf"));
        assert!(one.covers(lint("crlf")));
        assert!(!one.covers(lint("confusable")));
    }
}
