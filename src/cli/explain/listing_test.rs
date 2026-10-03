//! The tests of `listing.rs` (V127), in a file of their own so the module
//! reads as code. Still its child: `super` is `listing`.

use super::{Asked, sets};
use crate::cli::testkit::fixture;
use crate::judge::Config;
use crate::render::Format;
use std::path::Path;

const NONE: Asked<'static> = Asked {
    names: &[],
    locales: false,
    containing: None,
};

fn shown(config: &Config, asked: &Asked<'_>) -> String {
    sets(config, Format::Human, asked).unwrap_or_else(|why| why)
}

fn found(root: &Path) -> Config {
    crate::cli::config::discovered(root)
}

/// The configuration of `ctrm sets <flags>`, `|`-separated.
fn argued(root: &Path, flags: &str) -> Config {
    let words = std::iter::once("sets").chain(flags.split('|'));
    let argv: Vec<String> = words.map(str::to_owned).collect();
    crate::cli::config::from_argv(root, &argv).unwrap_or_default()
}

fn at(text: &str, needle: &str) -> usize {
    text.find(needle).unwrap_or(usize::MAX)
}

/// The curated presets lead, each with what it is for; the locales are
/// one line, not 1,800.
#[test]
fn the_presets_lead_with_descriptions_and_the_locales_are_counted() {
    let Some(root) = fixture("ctrm-sets-fixture", &[]) else {
        return;
    };
    let said = shown(&found(&root), &NONE);
    assert!(said.starts_with("ascii -- printable ASCII"), "{said}");
    assert!(said.contains("caveman -- the logic and arrow"), "{said}");
    assert!(said.contains("\n  U+00A7 U+00AC"), "{said}");
    assert!(said.contains(" CLDR locale sets: `ctrm sets --locales`"));
    assert!(!said.contains("\nja -- "), "{said}");
}

/// A repo's own sets list after the presets: what may I name HERE.
#[test]
fn the_declared_sets_follow_the_presets() {
    let files = [(".ctrm-sets", "house U+2261\n")];
    let Some(root) = fixture("ctrm-sets-declared-fixture", &files) else {
        return;
    };
    let said = shown(&found(&root), &NONE);
    let house = "house -- declared by this repository\n  U+2261";
    assert!(said.contains(house), "{said}");
    assert!(at(&said, "\nany -- ") < at(&said, "\nhouse -- "), "{said}");
}

/// `--locales` asks for the locales, and only them; the listing holds
/// every one while a check reads only the named (`src/charset/locale:V61`).
#[test]
fn locales_list_only_when_asked_and_a_check_reads_only_the_named() {
    let files = [(".ctrm", "*.md ascii+pl\n")];
    let Some(root) = fixture("ctrm-sets-locale-fixture", &files) else {
        return;
    };
    let config = found(&root);
    let asked = Asked {
        locales: true,
        ..NONE
    };
    let said = shown(&config, &asked);
    assert!(said.contains("ja -- one locale's letters, from CLDR\n  U+3005"));
    assert!(said.contains("\npt-BR -- ") && !said.contains("caveman"));
    let catalog = config.catalog().unwrap_or_default();
    assert!(catalog.get("pl").is_some() && catalog.get("ja").is_none());
}

/// Names filter, in the order named; a name nothing declares is refused.
#[test]
fn names_filter_the_listing_and_an_unknown_one_is_refused() {
    let Some(root) = fixture("ctrm-sets-named-fixture", &[]) else {
        return;
    };
    let config = found(&root);
    let said = shown(&config, &naming(&["pl", "typography"]));
    assert!(said.starts_with("pl -- ") && said.contains("\ntypography -- "));
    assert_eq!(said.lines().count(), 4, "{said}");
    let typo = shown(&config, &naming(&["a.md"]));
    assert_eq!(typo, "unknown set `a.md`");
}

/// An `Asked` naming `names`, leaked so it outlives the call: a test only.
fn naming(names: &[&str]) -> Asked<'static> {
    let owned: Vec<String> = names.iter().map(|n| (*n).to_owned()).collect();
    Asked {
        names: Vec::leak(owned),
        ..NONE
    }
}

/// `--containing` answers which sets hold a code point, curated first,
/// and takes the character itself as well as its `U+XXXX` name.
#[test]
fn containing_lists_the_sets_holding_a_code_point() {
    let Some(root) = fixture("ctrm-sets-containing-fixture", &[]) else {
        return;
    };
    let ask = |what: &'static str| Asked {
        containing: Some(what),
        ..NONE
    };
    let said = shown(&found(&root), &ask("U+22A5"));
    assert!(said.starts_with("caveman -- "), "{said}");
    assert!(said.contains("\nany -- ") && !said.contains("\nmath -- "));
    assert!(!said.contains("CLDR locale sets"), "{said}");
    assert_eq!(shown(&found(&root), &ask("\u{22A5}")), said);
    let bad = shown(&found(&root), &ask("ab"));
    assert!(bad.contains("one character"), "{bad}");
}

/// The json mirrors the human form: kind, description, the count.
#[test]
fn the_json_carries_kind_description_and_the_locale_count() {
    let Some(root) = fixture("ctrm-sets-json-fixture", &[]) else {
        return;
    };
    let said = sets(&found(&root), Format::Json, &NONE).unwrap_or_default();
    assert!(said.contains(r#""name":"caveman","kind":"preset","#));
    assert!(said.contains(r#""unlisted_locales":1831"#), "{said}");
    assert!(said.contains(r#""containing":null"#), "{said}");
}

/// V41 is visible here: `marks` holds different characters at another
/// fidelity, and `--fidelity f` IS the rule `* @f` (B30).
#[test]
fn the_listing_resolves_at_the_family_the_rules_give() {
    let Some(root) = fixture("ctrm-sets-fidelity-fixture", &[]) else {
        return;
    };
    let text = shown(&found(&root), &NONE);
    let emoji = shown(&argued(&root, "--fidelity|emoji"), &NONE);
    assert!(
        text.contains("U+2713") && emoji.contains("U+2705"),
        "{emoji}"
    );
    assert_eq!(emoji, shown(&argued(&root, "--rule|* @emoji"), &NONE));
}
