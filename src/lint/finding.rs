//! One reportable finding.

use crate::lint::{Level, Lint};
use crate::scan::Hit;

/// One reportable finding: what was found, which lint found it, and how
/// loudly it is being said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub hit: Hit,
    pub lint: Lint,
    pub level: Level,
}
