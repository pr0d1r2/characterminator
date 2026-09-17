//! The named checks this build knows.

use crate::lint::Group;

/// A named check. The name is what a rule line and the json output carry,
/// so it is the stable identifier rather than the message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lint {
    pub name: &'static str,
    pub group: Group,
}
