//! The tests of the `init` node (V128), on in-memory files and fixtures.

use super::cover::{Candidates, Grant};
use super::draft::draft;
use super::survey::{Survey, pattern_of};
use super::verb::{Ask, run};
use crate::cli::testkit::fixture;
use crate::render::Format;
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
        prose(&text)
            .contains("until it runs, check reports 1 finding in 1 file "),
        "{text}"
    );
    let bidi = "U+202E bidi-control in docs/a.md:1:11";
    let plain = "Pure ASCII, no line needed: *.rs (1)";
    assert!(text.contains(bidi) && text.contains(plain), "{text}");
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

/// `--keep-fixable` grants what `fix` would rewrite, so `check` reports
/// none of it and the draft has no "until fix runs" note to make.
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
    assert!(
        prose(&text).contains("reports 3 findings in 2 files"),
        "{text}"
    );
    assert!(text.lines().all(|line| line.len() <= 72), "{text}");
    assert!(text.contains("--keep-fixable"), "{text}");
}

/// Each count takes its own noun's number: one finding, one file, and
/// two findings in one file.
#[test]
fn a_count_of_one_is_singular() {
    let said = |text: &str| {
        let mut survey = Survey::new(false);
        survey.add("docs/a.md", text);
        prose(&draft(&survey, &Candidates::builtin()))
    };
    let one = said("a \u{2014} b\n");
    assert!(one.contains("reports 1 finding in 1 file ("), "{one}");
    let two = said("a \u{2014} b \u{2014} c\n");
    assert!(two.contains("reports 2 findings in 1 file ("), "{two}");
}

/// The note's advice still works once the draft is `.ctrm`: the command
/// it names is not refused for the file it is written in.
#[test]
fn the_advice_works_beside_the_draft_it_is_in() {
    let mut survey = Survey::new(false);
    survey.add("a.md", "a \u{2014} b\n");
    let ctrm = draft(&survey, &Candidates::builtin());
    let advice = "init --print --keep-fixable";
    assert!(prose(&ctrm).contains(&format!("(`{advice}`")), "{ctrm}");
    let Some(root) = fixture("ctrm-init-advice", &[(".ctrm", &ctrm)]) else {
        return;
    };
    let config = crate::cli::config::discovered(&root);
    let found = run(&config, asked(advice));
    let refused = found.as_ref().err().is_some_and(|e| e.contains("exists"));
    assert!(!refused, "{found:?}");
}

/// The flags an `init` command line names.
fn asked(command: &str) -> Ask {
    let words: Vec<&str> = command.split(' ').collect();
    Ask {
        print: words.contains(&"--print"),
        keep: words.contains(&"--keep-fixable"),
    }
}

/// A line that grants a character the map rewrites leaves it alone: it is
/// no finding and `fix` does not touch it (`src/fix:V6`), so the draft
/// neither counts it nor says `fix` rewrites it.
#[test]
fn what_the_line_grants_is_not_counted_until_fix_runs() {
    let mut survey = Survey::new(false);
    survey.add("docs/a.md", "a \u{E000} b \u{2014} c\n");
    let text = prose(&draft(&survey, &Candidates::builtin()));
    assert!(text.contains("\n*.md any\n"), "{text}");
    assert!(!text.contains("until it runs"), "{text}");
    assert!(text.contains("this line grants them: U+2014"), "{text}");
}

/// The draft read as prose: continuation lines joined to the line above.
fn prose(text: &str) -> String {
    text.replace("\n#   ", " ")
}

/// A set chosen for its letters can hold a character the map rewrites:
/// `kek` holds U+2019, `en-aux` does not.
#[test]
fn a_chosen_set_allows_what_it_holds_beyond_the_need() {
    let candidates = Candidates::builtin();
    let line = |name: &str| Grant::Sets(vec![(String::from(name), Vec::new())]);
    assert!(candidates.allows(&line("kek"), '\u{2019}'));
    assert!(!candidates.allows(&line("en-aux"), '\u{2019}'));
}

/// Two files of dashes, surveyed with or without `--keep-fixable`.
const DASHES: [(&str, &str); 2] = [
    ("a.md", "a \u{2014} b \u{2014} c\n"),
    ("b.md", "d \u{2014} e\n"),
];

/// `check`'s exit code and tally on [`DASHES`] under the draft itself,
/// or its refusal; `None` only when the fixture cannot be written.
type Checked = Result<(u8, String), String>;

fn checked_under_draft(name: &str, keep: bool) -> Option<Checked> {
    checked_under_draft_of(name, keep, &DASHES)
}

/// The same, for any tracked `files`.
fn checked_under_draft_of(
    name: &str,
    keep: bool,
    tracked: &[(&str, &str)],
) -> Option<Checked> {
    let mut survey = Survey::new(keep);
    tracked
        .iter()
        .for_each(|(path, text)| survey.add(path, text));
    let ctrm = draft(&survey, &Candidates::builtin());
    let mut files = tracked.to_vec();
    files.push((".ctrm", ctrm.as_str()));
    let root = fixture(name, &files)?;
    let asked: Vec<String> =
        tracked.iter().map(|(p, _)| (*p).to_owned()).collect();
    let config = crate::cli::config::discovered(&root);
    let found = crate::cli::check::run(&config, &asked, Format::Human);
    Some(found.map(|report| (report.code, report.note)))
}

/// What the draft promises is what `check` does under it: the default
/// draft's count is `check`'s own, and `--keep-fixable`'s leaves nothing
/// `fix` would rewrite for `check` to report.
#[test]
fn check_under_the_draft_does_what_the_draft_says() {
    let Some(unfixed) = checked_under_draft("ctrm-init-unfixed", false) else {
        return;
    };
    assert!(unfixed.is_ok(), "{unfixed:?}");
    let (code, note) = unfixed.unwrap_or_default();
    assert_eq!(code, 1, "{note}");
    assert!(note.starts_with("3 findings in 2 files"), "{note}");
    let kept = checked_under_draft("ctrm-init-keep-fixable", true);
    assert_eq!(kept.map(|found| found.map(|(code, _)| code)), Some(Ok(0)));
}

/// `--keep-fixable` clears what `fix` would rewrite, not every finding: a
/// hazard is never granted (V128), so `check` still fails on it, and on
/// it alone.
#[test]
fn keep_fixable_still_leaves_a_hazard_for_check() {
    let tracked = [("a.md", "a \u{2014} b\u{202E}c\n")];
    let found = checked_under_draft_of("ctrm-init-keep-hazard", true, &tracked);
    let Some(found) = found else {
        return;
    };
    assert!(found.is_ok(), "{found:?}");
    let (code, note) = found.unwrap_or_default();
    assert_eq!(code, 1, "{note}");
    assert!(note.starts_with("1 finding in 1 file;"), "{note}");
}

/// `src/lint/hazard:V142`: a double-encoded run is damage to fix, not a need.
/// Its pieces (a lead letter, a Latin-1 symbol) are left out of the grant,
/// so the type is not widened to `any`, and the run is listed under "fix
/// these" by what it decodes to.
#[test]
fn a_double_encoded_run_is_listed_not_granted() {
    let mut survey = Survey::new(false);
    survey.add("docs/a.md", "must \u{e2}\u{8a}\u{a5} share\n");
    let text = draft(&survey, &Candidates::builtin());
    assert!(
        text.contains("U+22A5 double-encoded in docs/a.md:1:6"),
        "{text}"
    );
    assert!(
        !text.contains("U+00E2") && !text.contains(" any\n"),
        "{text}"
    );
    assert!(!text.contains("\n*.md "), "{text}");
}

/// A literal U+202E and a run decoding to it are one character two lints
/// name: the draft lists both, not whichever was seen first.
#[test]
fn a_literal_hazard_and_a_run_decoding_to_it_keep_both_lints() {
    let mut survey = Survey::new(false);
    survey.add("docs/a.md", "x\u{202E}y \u{e2}\u{80}\u{ae}\n");
    let text = draft(&survey, &Candidates::builtin());
    let both =
        "U+202E double-encoded, bidi-control in docs/a.md:1:5, docs/a.md:1:2";
    assert!(text.contains(both), "{text}");
}

/// Every file holding a hazard is listed with line:col, not only the first,
/// and a long list is cut with a count of the rest.
#[test]
fn a_hazard_is_listed_in_every_file_and_cut_when_long() {
    let mut survey = Survey::new(false);
    survey.add("one.sh", "a \u{8a} b\n");
    survey.add("two.sh", "c \u{8a} d\n");
    let text = draft(&survey, &Candidates::builtin());
    let both = "U+008A control-character in one.sh:1:3, two.sh:1:3";
    assert!(text.contains(both), "{text}");
    for n in 0..10 {
        survey.add(&format!("f{n}.sh"), "x \u{8a}\n");
    }
    let text = draft(&survey, &Candidates::builtin());
    assert!(text.contains(" and 4 more -- ctrm check"), "{text}");
    assert!(!text.contains("f9.sh"), "{text}");
}
