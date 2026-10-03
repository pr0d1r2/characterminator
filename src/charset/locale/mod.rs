//! The CLDR locale letter presets and their generated data. See
//! `src/charset/locale/SPEC.md`.
//!
//! Composes, does not implement (`src:C`). One file, `lookup`, resolves a
//! locale name lazily against the compiled-in `locales.ctrm-sets`, which
//! `cldr-letters.sh` beside it generates. The module itself is private to
//! `src/charset`, which re-exports these two to its siblings (`src:V39`).

mod lookup;

pub(crate) use lookup::{adopt, adopt_all};
