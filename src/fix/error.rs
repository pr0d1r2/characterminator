//! Why a map cannot be used, or a fix cannot be trusted -- and how each
//! refusal reads. Every message names its SUBJECT, then the rule it broke.

/// Why a map cannot be used, or a fix cannot be trusted.
///
/// Every variant is a refusal to produce text, never a partial rewrite: a
/// config error the caller turns into exit 2, or an invariant this node
/// checked on itself before anything could be written.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A map line that is not one of the declared forms, by 1-based number.
    Syntax { line: usize },
    /// A map line whose source every set grants, so it can never be
    /// rewritten (`src/judge:V116`).
    NeverRewritten { line: usize },
    /// A replacement is rewritten again without ever settling, so the map
    /// chains back to a source it already used.
    MapCycle,
    /// A family was named that nothing declares.
    UnknownFamily { name: String },
    /// A `use` line named a map this crate does not ship (`src/fix/words:V51`).
    UnknownMap { name: String },
    /// A parent chain comes back to a family it already visited (V27).
    FamilyCycle { name: String },
    /// A family has no parent, so its chain never reaches `ascii` (V27).
    UnrootedFamily { name: String },
    /// `ascii` is the intrinsic root and cannot be given a parent (V27).
    RootReparented,
    /// `fix` would have changed a byte outside a violation (V6).
    TouchedAllowedBytes,
    /// `fix(fix(x))` would differ from `fix(x)` (V5).
    NotIdempotent,
}

impl Error {
    /// What the refusal is ABOUT: the line, the family, or nothing when
    /// the fault belongs to the map as a whole.
    fn subject(&self) -> String {
        match self {
            Self::Syntax { line } | Self::NeverRewritten { line } => {
                format!("map line {line}")
            }
            Self::UnknownFamily { name }
            | Self::FamilyCycle { name }
            | Self::UnrootedFamily { name } => format!("family '{name}'"),
            Self::UnknownMap { name } => format!("map '{name}'"),
            Self::MapCycle
            | Self::RootReparented
            | Self::TouchedAllowedBytes
            | Self::NotIdempotent => String::from("the map"),
        }
    }

    /// V6, which this node checks on itself before anything is written.
    /// No spec id in the words (`src/cli/usage:V121`): the reader has no
    /// spec, so the message says whose fault it is instead.
    const REFUSED_TOUCH: &'static str = "would have touched an allowed byte, \
         so nothing was written -- this is a ctrm bug, please report it";

    /// V5, checked the same way.
    const REFUSED_UNSETTLED: &'static str = "would not settle when run twice, \
         so nothing was written -- this is a ctrm bug, please report it";

    /// The forms a map line may take, for a line that is none of them.
    const FORMS: &'static str = "is not a map line -- expected `<from> [<to>]`, \
         `word <from> <to>`, `family <name> <parent>`, \
         `= <class> <family>:<member>,...` or `use <name>`";

    /// A source no set withholds (`src/judge:V116`).
    const NEVER: &'static str = "maps a source made only of `ascii`, which \
         every set grants, so it would never be rewritten -- a source needs \
         a character outside `ascii`";

    /// The 1-based line a refusal is about, when it is about one.
    #[must_use]
    pub(crate) const fn line(&self) -> Option<usize> {
        match self {
            Self::Syntax { line } | Self::NeverRewritten { line } => {
                Some(*line)
            }
            _ => None,
        }
    }

    /// Why it refused.
    ///
    /// A config fault says what is expected instead; this node catching
    /// ITSELF says so, so a reader knows whether their config is wrong or
    /// this crate is.
    #[must_use]
    pub(crate) const fn reason(&self) -> &'static str {
        match self {
            Self::Syntax { .. } => Self::FORMS,
            Self::NeverRewritten { .. } => Self::NEVER,
            Self::MapCycle => "rewrites in a cycle and never settles",
            Self::UnknownFamily { .. } => "is not declared",
            Self::UnknownMap { .. } => "is not a builtin map",
            Self::FamilyCycle { .. } => "is its own ancestor",
            Self::UnrootedFamily { .. } => "never reaches `ascii`",
            Self::RootReparented => "takes no parent: `ascii` is the root",
            Self::TouchedAllowedBytes => Self::REFUSED_TOUCH,
            Self::NotIdempotent => Self::REFUSED_UNSETTLED,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.subject(), self.reason())
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::Error;

    /// `src/cli/usage:V121`: no refusal carries a spec id; its reader
    /// has no spec to look it up in.
    #[test]
    fn no_refusal_names_a_spec_id() {
        let name = || String::from("x");
        let named = [
            Error::UnknownFamily { name: name() },
            Error::UnknownMap { name: name() },
            Error::FamilyCycle { name: name() },
            Error::UnrootedFamily { name: name() },
        ];
        let bare = [Error::MapCycle, Error::RootReparented];
        let own = [Error::TouchedAllowedBytes, Error::NotIdempotent];
        let lines =
            [Error::Syntax { line: 1 }, Error::NeverRewritten { line: 1 }];
        for refusal in named.into_iter().chain(bare).chain(own).chain(lines) {
            let said = refusal.to_string();
            assert!(!said.contains("(V"), "{said}");
        }
    }
}
