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
            Self::Syntax { line } => format!("map line {line}"),
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
    const REFUSED_TOUCH: &'static str =
        "would have touched an allowed byte, so it was refused (V6)";

    /// V5, checked the same way.
    const REFUSED_UNSETTLED: &'static str =
        "would not settle when run twice, so it was refused (V5)";

    /// Why it refused.
    ///
    /// Each names the RULE, because half of these are this node catching
    /// ITSELF: a reader who sees one needs to know whether their config
    /// is wrong or this crate is.
    const fn reason(&self) -> &'static str {
        match self {
            Self::Syntax { .. } => "is not a declared form",
            Self::MapCycle => "rewrites in a cycle and never settles",
            Self::UnknownFamily { .. } => "is not declared",
            Self::UnknownMap { .. } => "is not a builtin map (V51)",
            Self::FamilyCycle { .. } => "is its own ancestor (V27)",
            Self::UnrootedFamily { .. } => "never reaches `ascii` (V27)",
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
