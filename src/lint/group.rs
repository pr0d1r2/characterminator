//! The group a lint belongs to, and the default level it carries.

/// The groups a lint can belong to. The group carries the default level:
/// hazard forbids, charset denies, pedantic allows until asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Hazard,
    Charset,
    Pedantic,
}
