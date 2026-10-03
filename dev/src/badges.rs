//! The README `badges` block (`dev:T69`). Every number is read from its
//! owner by the caller (`dev:V131`); this module only lays the facts out.
//! The badges that state no number -- the gate, the flake, how the tool
//! was built -- are fixed text, because nothing measures them.

/// Everything the block says, each value already read from its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Facts {
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) licence: String,
    pub(crate) edition: String,
    pub(crate) msrv: String,
    pub(crate) direct: usize,
    pub(crate) closure: usize,
    pub(crate) coverage: String,
    pub(crate) unsafe_level: String,
}

const FIXED: &str = "\
[![gate hk](https://img.shields.io/badge/gate-hk-6E4AFF)](hk.pkl)
[![nix flake](https://img.shields.io/badge/nix-flake-5277C3?logo=nixos&logoColor=white)](flake.nix)

[![built with Claude Code](https://img.shields.io/badge/built_with-Claude_Code-D97757)](https://claude.com/claude-code)
[![built with Opus 5 and 5.5](https://img.shields.io/badge/built_with-Opus_5_%26_5.5-D97757)](https://www.anthropic.com/claude)
[![built with SDD](https://img.shields.io/badge/built_with-spec--driven_development-D97757)](SPEC.md)
";

/// Text as a shields.io path segment: `-` and `_` doubled, space as `_`.
fn shield(text: &str) -> String {
    text.replace('-', "--").replace('_', "__").replace(' ', "_")
}

fn identity(f: &Facts) -> String {
    let (name, slug, lic) = (&f.name, &f.slug, &f.licence);
    let lic_shield = shield(lic);
    format!(
        "[![ci](https://github.com/{slug}/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/{slug}/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/{name}.svg)](https://crates.io/crates/{name})
[![docs.rs](https://img.shields.io/docsrs/{name})](https://docs.rs/{name})
[![License: {lic}](https://img.shields.io/badge/license-{lic_shield}-blue.svg)](LICENSE)
"
    )
}

fn toolchain(f: &Facts) -> String {
    let (ed, msrv) = (shield(&f.edition), shield(&f.msrv));
    format!(
        "[![edition {}](https://img.shields.io/badge/edition-{ed}-000000?logo=rust&logoColor=white)](Cargo.toml)
[![MSRV {}](https://img.shields.io/badge/MSRV-{msrv}-000000?logo=rust&logoColor=white)](Cargo.toml)
",
        f.edition, f.msrv
    )
}

fn measured(f: &Facts) -> String {
    let (direct, closure, cov) = (f.direct, f.closure, &f.coverage);
    format!(
        "[![direct dependencies {direct}](https://img.shields.io/badge/direct_dependencies-{direct}-brightgreen)](docs/THIRD-PARTY-NOTICES.md)
[![runtime closure {closure}](https://img.shields.io/badge/runtime_closure-{closure}-brightgreen)](docs/THIRD-PARTY-NOTICES.md)
[![coverage {cov}%](https://img.shields.io/badge/coverage-{cov}%25-brightgreen)](.coverage)
"
    )
}

/// `forbid` is the claim worth a green badge; any other level is shown as
/// it is, in a colour that does not read as a pass.
fn unsafety(f: &Facts) -> String {
    let (word, colour) = match f.unsafe_level.as_str() {
        "forbid" => ("forbidden".to_owned(), "brightgreen"),
        other => (other.to_owned(), "orange"),
    };
    let text = shield(&word);
    format!(
        "[![unsafe {word}](https://img.shields.io/badge/unsafe-{text}-{colour})](Cargo.toml)\n"
    )
}

/// The whole block body, ending in a newline (`dev:V134`).
pub(crate) fn render(f: &Facts) -> String {
    [
        identity(f),
        toolchain(f),
        measured(f),
        unsafety(f),
        FIXED.to_owned(),
    ]
    .concat()
}

#[cfg(test)]
#[path = "badges_test.rs"]
mod tests;
