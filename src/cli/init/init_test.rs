//! The tests of the `init` node (V128), on in-memory files and fixtures.

use super::cover::{Candidates, Grant};
use super::draft::draft;
use super::survey::{Survey, pattern_of};
use super::verb::{Ask, run};
use crate::cli::testkit::fixture;
use std::collections::BTreeSet;

#[test]
fn a_file_type_is_its_extension_glob_or_its_own_name() {
    assert_eq!(pattern_of("src/a.rs"), "*.rs");
    assert_eq!(pattern_of("docs/x.tar.gz"), "*.gz");
    assert_eq!(pattern_of("Makefile"), "Makefile");
    assert_eq!(pattern_of("sub/.gitignore"), ".gitignore");
}

fn covered(chars: &[char]) -> Grant {
    let needed: BTreeSet<char> = chars.iter().copied().collect();
    Candidates::builtin().cover(&needed)
}

/// Greedy over the curated presets: each set chosen says what it covers.
#[test]
fn the_cover_takes_the_presets_that_gain_most() {
    let grant = covered(&['\u{2192}', '\u{2234}', '\u{00A9}']);
    let want = vec![
        (String::from("caveman"), vec!['\u{2192}', '\u{2234}']),
        (String::from("legal"), vec!['\u{00A9}']),
    ];
    assert_eq!(grant, Grant::Sets(want));
}

/// Letters no preset holds go to ONE locale; nothing at all, to `any`.
#[test]
fn letters_take_a_locale_and_the_uncovered_take_any() {
    let sets = match covered(&['\u{0105}', '\u{0142}']) {
        Grant::Sets(sets) => sets,
        Grant::Any(_) => Vec::new(),
    };
    assert_eq!(sets.len(), 1, "{sets:?}");
    assert_eq!(covered(&['\u{E000}']), Grant::Any(vec!['\u{E000}']));
}

/// B80: Polish letters and one U+00E9 take the language they are written
/// in, then English's loan letters, not one auxiliary set of a third
/// language that holds them all by accident.
#[test]
fn letters_take_their_language_before_an_accidental_cover() {
    let polish = ['\u{00E9}', '\u{00F3}', '\u{0107}', '\u{0142}', '\u{017C}'];
    let names: Vec<String> = match covered(&polish) {
        Grant::Sets(sets) => sets.into_iter().map(|(name, _)| name).collect(),
        Grant::Any(_) => Vec::new(),
    };
    assert_eq!(names, ["pl", "en-aux"]);
}

const WRITE: Ask = Ask {
    print: false,
    keep: false,
};
const PRINT: Ask = Ask {
    print: true,
    keep: false,
};

fn surveyed() -> Survey {
    let mut survey = Survey::new(false);
    let prose = "caf\u{e9} \u{2192} \u{2014} x\u{202E}y\n";
    survey.add("docs/a.md", prose);
    survey.add("src/b.rs", "fn main() {}\n");
    survey
}

/// The draft grants what is needed, leaves the map's work to `fix`,
/// names a hazard as work to do and NEVER grants it, and is the same
/// bytes every time.
#[test]
fn the_draft_grants_the_need_and_never_a_hazard() {
    let candidates = Candidates::builtin();
    let text = draft(&surveyed(), &candidates);
    assert!(text.contains("\n*.md ascii+marks+en-aux\n"), "{text}");
    assert!(
        text.contains("until it runs, check reports 1 findings in 1 files"),
        "{text}"
    );
    assert!(text.contains("U+202E bidi-control in docs/a.md"), "{text}");
    assert!(
        text.contains("Pure ASCII, no line needed: *.rs (1)"),
        "{text}"
    );
    assert!(!text.contains("hazard-bidi") && !text.contains(" any\n"));
    assert_eq!(text, draft(&surveyed(), &candidates));
}

/// An existing `.ctrm` is refused, exit 2, pointing at `--print`, and
/// `--print` writes nothing.
#[test]
fn an_existing_ctrm_is_never_overwritten() {
    let files = [(".ctrm", "* ascii\n")];
    let Some(root) = fixture("ctrm-init-existing-fixture", &files) else {
        return;
    };
    let config = crate::cli::config::discovered(&root);
    let why = run(&config, WRITE).err().unwrap_or_default();
    assert!(why.contains("--print"), "{why}");
    // `--print` is never refused for the existing file. In this fixture,
    // which has no git-tracked file, it may still stop at the empty-run
    // rule (`src/judge:V118`), and that is not the refusal under test.
    let printed = run(&config, PRINT);
    let refused = printed
        .as_ref()
        .err()
        .is_some_and(|e| e.contains("--print"));
    assert!(!refused, "{printed:?}");
    let kept = std::fs::read_to_string(root.join(".ctrm")).unwrap_or_default();
    assert_eq!(kept, "* ascii\n");
}

/// `--keep-fixable` grants what `fix` would rewrite, so the draft passes
/// `check` as written and has no "until fix runs" note to make.
#[test]
fn keep_fixable_grants_what_fix_would_rewrite() {
    let mut survey = Survey::new(true);
    survey.add("docs/a.md", "a \u{2014} b \u{2014} c\n");
    let text = draft(&survey, &Candidates::builtin());
    assert!(text.contains("\n*.md ascii+typography\n"), "{text}");
    assert!(!text.contains("until it runs"), "{text}");
}

/// Without it the draft says how many findings `check` reports until `fix`
/// runs, and in how many files.
#[test]
fn the_default_draft_counts_what_check_reports_until_fix_runs() {
    let mut survey = Survey::new(false);
    survey.add("docs/a.md", "a \u{2014} b \u{2014} c\n");
    survey.add("docs/b.md", "d \u{2014} e\n");
    let text = draft(&survey, &Candidates::builtin());
    assert!(text.contains("reports 3 findings in 2 files"), "{text}");
    assert!(text.contains("--keep-fixable"), "{text}");
}
