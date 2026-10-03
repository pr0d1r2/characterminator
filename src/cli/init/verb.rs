//! `init [--print]` itself (V128): survey, cover, write -- or print.

use super::{cover, draft, survey};
use crate::judge::{Config, File, files};
use std::path::Path;

/// The file `init` writes, at the root the run was asked about.
const CTRM: &str = ".ctrm";

/// Survey the tracked files and write the draft, or print it. An existing
/// `.ctrm` is NEVER overwritten: it holds decisions, and a draft is not one.
///
/// # Errors
///
/// An existing `.ctrm` (unless `print`), a fileset that cannot be listed,
/// or a draft that cannot be written.
pub(crate) fn run(config: &Config, print: bool) -> Result<String, String> {
    let target = config.root.join(CTRM);
    if !print && target.exists() {
        return Err(format!(
            "`{CTRM}` exists and init never overwrites it; \
             `ctrm init --print` writes the draft to stdout instead"
        ));
    }
    let text = drafted(&config.root)?;
    if print {
        return Ok(text.trim_end().to_owned());
    }
    std::fs::write(&target, &text).map_err(|e| format!("{CTRM}: {e}"))?;
    Ok(format!("wrote {CTRM}: review it, then `ctrm check`"))
}

/// The draft for the tracked files under `root`.
fn drafted(root: &Path) -> Result<String, String> {
    let mut found = survey::Survey::new();
    for file in files(root, &[])? {
        let File { shown, text, .. } = file?;
        match text {
            Ok(text) => found.add(&shown, &text),
            Err(_) => found.skipped = found.skipped.saturating_add(1),
        }
    }
    Ok(draft::draft(&found, &cover::Candidates::builtin()))
}
