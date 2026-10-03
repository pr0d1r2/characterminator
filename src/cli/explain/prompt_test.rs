//! The tests of `prompt.rs` and its sections (V32, V129), in a file of
//! their own so the module reads as code. Still its child: `super` is
//! `prompt`.

use super::render;
use crate::cli::config::from_argv;
use crate::judge::Config;
use std::path::Path;

fn loaded(words: &[&str]) -> Config {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let words: Vec<String> = ["explain", "--no-files"]
        .iter()
        .chain(words)
        .map(|w| (*w).to_owned())
        .collect();
    from_argv(root, &words).unwrap_or_default()
}

fn prompt(words: &[&str], path: Option<&str>) -> String {
    render(&loaded(words), path).unwrap_or_else(|why| why)
}

/// The same configuration renders the same bytes, every time.
#[test]
fn the_prompt_is_deterministic() {
    let words = ["--rule", "*.md caveman", "--map", "use words"];
    assert_eq!(prompt(&words, None), prompt(&words, None));
    assert_eq!(prompt(&words, Some("a.md")), prompt(&words, Some("a.md")));
}

/// For one path: the set in force, the pattern behind it -- NOT the argv
/// position, which a model never saw (V129) -- and the fidelity.
#[test]
fn a_path_prompt_names_its_sets_pattern_and_fidelity() {
    let said = prompt(&["--rule", "*.md caveman @emoji"], Some("a.md"));
    assert!(said.contains("- ascii+caveman (pattern `*.md`)"), "{said}");
    assert!(!said.contains("argv["), "{said}");
    assert!(said.contains("\ncaveman U+00A7 \""), "{said}");
    assert!(said.contains("Fidelity: emoji."), "{said}");
}

/// V129: a small set shows its glyphs beside its code points, so a model
/// writing Polish sees the letters are allowed; a large one stays ranges.
#[test]
fn a_small_set_lists_its_glyphs_and_a_large_one_its_ranges() {
    let said = prompt(&["--rule", "* pl"], Some("a.md"));
    assert!(said.contains("\npl U+00D3 \"\u{00D3}\" U+00F3 \"\u{00F3}\""));
    assert!(said.contains("U+017C \"\u{017C}\""), "{said}");
    assert!(
        said.contains("\nascii U+0009-U+000A U+0020-U+007E"),
        "{said}"
    );
}

/// V129: only what a writer types, sorted by code point, each once; the
/// emoji modifiers and hazards the map also deletes are not listed, and
/// a line the user's own map declares is.
#[test]
fn the_replacements_are_typed_sorted_and_what_fix_would_do() {
    let plain = prompt(&[], None);
    let at = |needle: &str| plain.find(needle).unwrap_or(usize::MAX);
    assert!(at("U+00A0 -> \" \"") < at("U+2014 -> \"--\""), "{plain}");
    assert!(plain.contains("U+201E -> \"\\\"\""), "{plain}");
    assert!(!plain.contains("U+1F3FB") && !plain.contains("U+200B ->"));
    assert_eq!(plain.matches("U+2014 ->").count(), 1, "{plain}");
    let own = prompt(&["--map", "U+22A5 not"], None);
    assert!(own.contains("U+22A5 -> \"not\""), "{own}");
}

/// A character the path's set grants is not "replaced".
#[test]
fn a_granted_character_is_not_listed_as_a_replacement() {
    let said = prompt(&["--rule", "*.md typography"], Some("a.md"));
    assert!(!said.contains("U+2014 ->"), "{said}");
}

/// V129: the hazards appear ONCE, as one line of classes, never as every
/// range; under `any` there is no sequence sentence, and the joiner of an
/// RGI sequence is said to be fine.
#[test]
fn the_hazards_are_one_line_and_any_does_not_contradict_it() {
    let said = prompt(&["--rule", "* any"], Some("a.md"));
    assert_eq!(said.matches("Never write these").count(), 1, "{said}");
    assert!(said.contains("(hazard-bidi)") && !said.contains("U+202A-"));
    assert!(!said.contains("Write one emoji"), "{said}");
    assert!(
        said.contains("inside an RGI emoji sequence is fine"),
        "{said}"
    );
    let emoji = prompt(&["--rule", "* emoji"], Some("a.md"));
    assert!(emoji.contains("Write one emoji, not a sequence"), "{emoji}");
    assert!(!emoji.contains("sequence is fine"), "{emoji}");
}

/// V129: `--pedantic` adds one line naming the lints it turned on.
#[test]
fn pedantic_names_the_lints_it_enables() {
    let plain = prompt(&[], Some("a.md"));
    assert!(!plain.contains("Pedantic"), "{plain}");
    let said = prompt(&["--pedantic"], Some("a.md"));
    assert!(said.contains("Pedantic lints are on"), "{said}");
    assert!(said.contains("not-nfc") && said.contains("final-newline"));
}

/// The prompt is ASCII but for the glyphs of a small set's members line.
#[test]
fn the_prompt_is_ascii_outside_the_members_lines() {
    let said = prompt(&["--rule", "*.md caveman", "--map", "use words"], None);
    let glyphs = |line: &&str| line.contains(" \"") && line.contains("U+");
    let rest: Vec<&str> = said.lines().filter(|l| !glyphs(l)).collect();
    assert!(rest.iter().all(|line| line.is_ascii()), "{said}");
    assert!(said.contains("- `*.md`: ascii+caveman"), "{said}");
}
