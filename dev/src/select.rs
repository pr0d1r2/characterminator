//! Which outputs a change can have made stale (`dev:V136`). Each output
//! declares its INPUTS; the gate hands the changed files in, and only the
//! outputs an input of which changed are checked. Too broad costs a
//! re-render; too narrow passes a stale block -- so pre-commit runs this
//! scoped and pre-push runs it with no paths, which checks everything.

/// One generated block: the verb that renders it, the file and marker it
/// lives under, and every file whose change can make it stale.
#[derive(Debug)]
pub(crate) struct Output {
    pub(crate) name: &'static str,
    pub(crate) file: &'static str,
    pub(crate) marker: &'static str,
    pub(crate) inputs: &'static [&'static str],
}

/// Both outputs read `cargo tree`, whose answer `Cargo.lock` decides.
pub(crate) const OUTPUTS: [Output; 2] = [
    Output {
        name: "readme",
        file: "README.md",
        marker: "badges",
        inputs: &[
            "README.md",
            "Cargo.toml",
            "Cargo.lock",
            ".coverage",
            "dev/src/**",
        ],
    },
    Output {
        name: "notices",
        file: "docs/THIRD-PARTY-NOTICES.md",
        marker: "closure",
        inputs: &[
            "docs/THIRD-PARTY-NOTICES.md",
            "Cargo.toml",
            "Cargo.lock",
            "dev/src/**",
        ],
    },
];

/// A literal path, or `dir/**` for anything beneath `dir`.
fn matches(pattern: &str, path: &str) -> bool {
    let path = path.trim_start_matches("./");
    match pattern.strip_suffix("/**") {
        Some(dir) => path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/')),
        None => pattern == path,
    }
}

/// The outputs `paths` can have invalidated; no paths selects all of them.
pub(crate) fn selected(paths: &[String]) -> Vec<&'static Output> {
    OUTPUTS
        .iter()
        .filter(|o| {
            paths.is_empty()
                || paths.iter().any(|p| o.inputs.iter().any(|i| matches(i, p)))
        })
        .collect()
}

#[cfg(test)]
#[path = "select_test.rs"]
mod tests;
