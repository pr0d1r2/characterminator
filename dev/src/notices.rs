//! The `closure` block of `docs/THIRD-PARTY-NOTICES.md` (`dev:T70`): every
//! package a consumer's build can link, on any target (`dev:V132`), with
//! its version and licence, and a tally of the licence expressions.

use std::collections::{BTreeMap, BTreeSet};

use crate::owners::Pkg;

fn table(closure: &BTreeSet<Pkg>) -> String {
    let rows: String = closure
        .iter()
        .map(|p| format!("| `{}` | {} | {} |\n", p.name, p.version, p.licence))
        .collect();
    format!("| package | version | licence |\n|---|---|---|\n{rows}")
}

/// Each licence expression and how many packages carry it, most first.
fn tally(closure: &BTreeSet<Pkg>) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for p in closure {
        let n = counts.entry(p.licence.as_str()).or_insert(0);
        *n = n.saturating_add(1);
    }
    let mut rows: Vec<(&str, usize)> = counts.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    rows.iter()
        .map(|(lic, n)| format!("- `{lic}` ({n})\n"))
        .collect()
}

/// The block body. `direct` is `[dependencies]`'s count; a closure smaller
/// than it means the two owners disagree, which is an error, not a number.
pub(crate) fn render(
    closure: &BTreeSet<Pkg>,
    direct: usize,
) -> Result<String, String> {
    let total = closure.len();
    let transitive = total
        .checked_sub(direct)
        .ok_or_else(|| format!("Cargo.toml lists {direct} direct dependencies but `cargo tree` found {total} packages"))?;
    Ok(format!(
        "## The closure: {total} packages\n\n{direct} direct, {transitive} transitive, on every target platform (`cargo tree -e normal --target all`).\n\n{}\nLicence expressions, by how many packages carry each:\n\n{}",
        table(closure),
        tally(closure)
    ))
}

#[cfg(test)]
#[path = "notices_test.rs"]
mod tests;
