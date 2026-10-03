//! The tests of `fix.rs`, in a file of their own so the module reads as
//! code (sherd V50). Still its child: `super` is `fix`. The hazard cases
//! (`src/fix:V104`) are a child of this one, in `fix_hazard_test.rs`.

use super::run;
use crate::cli::testkit::fixture;
use crate::render::Format;
use std::path::Path;

fn ran(root: &Path, write: bool) -> (String, u8) {
    let asked = ["notes.md".to_owned()];
    match run(
        &crate::cli::config::discovered(root),
        &asked,
        Format::Human,
        write,
    ) {
        Ok(report) => (report.text, report.code),
        Err(why) => (why, 9),
    }
}

fn read(root: &Path) -> String {
    std::fs::read_to_string(root.join("notes.md")).unwrap_or_default()
}

/// The builtin map, through the verb: an em dash and curly quotes
/// become ASCII, and the file on disk changes.
#[test]
fn fix_rewrites_what_the_builtin_map_covers() {
    let files = [("notes.md", "a \u{2014} \u{201C}b\u{201D}\n")];
    let Some(root) = fixture("ctrm-fix-fixture", &files) else {
        return;
    };
    let (text, code) = ran(&root, true);
    assert_eq!(read(&root), "a -- \"b\"\n", "{text}");
    // V7: a bare `fix` that left nothing behind exits 0. Only `check`
    // and `fix --check` gate, and the file IS clean now.
    assert_eq!(code, 0, "{text}");
}

/// `--check` reports the SAME thing and writes nothing, which is the
/// half of V7 that makes it usable in a gate.
#[test]
fn fix_check_reports_the_drift_without_writing() {
    let before = "a \u{2014} b\n";
    let files = [("notes.md", before)];
    let Some(root) = fixture("ctrm-fixcheck-fixture", &files) else {
        return;
    };
    let (text, code) = ran(&root, false);
    assert_eq!(read(&root), before, "{text}");
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("U+2014"), "{text}");
}

/// V6: a character the file is ALLOWED to hold is not touched, even
/// when the map has an entry for it. The grant wins.
#[test]
fn an_allowed_character_is_left_alone() {
    let files = [
        (".ctrm", "*.md ascii+typography\n"),
        ("notes.md", "a \u{2014} b\n"),
    ];
    let Some(root) = fixture("ctrm-fix-allowed-fixture", &files) else {
        return;
    };
    let (text, code) = ran(&root, true);
    assert_eq!(read(&root), "a \u{2014} b\n", "{text}");
    assert_eq!(code, 0, "{text}");
}

/// V4: a disallowed character with no mapping is KEPT and reported,
/// and the run exits 1 rather than claiming success.
#[test]
fn a_character_with_no_mapping_is_kept_and_still_fails() {
    // IDENTICAL TO: outside `ascii`, and no builtin entry covers it.
    let files = [("notes.md", "a \u{2261} b\n")];
    let Some(root) = fixture("ctrm-fix-unmapped-fixture", &files) else {
        return;
    };
    let (text, code) = ran(&root, true);
    assert_eq!(read(&root), "a \u{2261} b\n", "{text}");
    assert_eq!(code, 1, "{text}");
    // B23: and NAMED, in `check`'s row grammar, so exit 1 says why.
    assert_eq!(text, "notes.md:1:3 U+2261 ascii");
}

/// B21: a file `check` skips as binary is skipped by `fix` too, and
/// named. Before, only UTF-8 was asked, so a NUL-laden blob that
/// happened to decode had its bytes rewritten.
#[test]
fn a_binary_file_is_skipped_and_named_not_rewritten() {
    let blob = "\0\0\u{2014}\u{FEFF}data\0";
    let files = [("notes.md", blob)];
    let Some(root) = fixture("ctrm-fix-binary-fixture", &files) else {
        return;
    };
    let (text, code) = ran(&root, true);
    assert_eq!(read(&root), blob, "{text}");
    assert_eq!(text, "notes.md: skipped, binary");
    assert_eq!(code, 0, "{text}");
}

/// B42: what `fix` leaves is judged as `check` judges it (V80). Each
/// `.ctrm` here made the two verbs disagree: `fix` judged the set at
/// a fixed `deny` and never asked about hazards.
fn left_behind(name: &str, ctrm: &str, text: &str) -> (String, u8) {
    let files = [(".ctrm", ctrm), ("notes.md", text)];
    let Some(root) = fixture(name, &files) else {
        return (String::new(), 9);
    };
    ran(&root, false)
}

#[test]
fn an_allowed_or_warned_lint_does_not_fail_fix_check() {
    let text = "a \u{2261} b\n";
    let allow = "*.md ascii !outside-set=allow\n";
    let (out, code) = left_behind("ctrm-fix-allow-fixture", allow, text);
    assert_eq!((out.as_str(), code), ("", 0));
    let warn = "*.md ascii !outside-set=warn\n";
    let (out, code) = left_behind("ctrm-fix-warn-fixture", warn, text);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("U+2261"), "{out}");
}

/// `src/fix:V104` (B59): a hazard is rewritten even where the set
/// grants it, so `--check` fails on the DRIFT -- the row a person
/// acts on -- where it used to fail on a hazard only a hand could
/// remove. A control character `fix` may not delete is still left
/// and judged as `check` judges it.
#[test]
fn a_hazard_the_set_grants_is_drift_for_fix_check() {
    let text = "a\u{202E}b\n";
    let gone = "notes.md:1:2 U+202E -> \"\"";
    let any = "* any\n";
    let (out, code) = left_behind("ctrm-fix-hazard-any", any, text);
    assert_eq!((out.as_str(), code), (gone, 1));
    let ascii = "* ascii\n";
    let (out, code) = left_behind("ctrm-fix-hazard-ascii", ascii, text);
    assert_eq!((out.as_str(), code), (gone, 1));
    // Placed in the original: the em dash before it became `--`, and
    // the row still names column 3, where the override sits on disk.
    let shifted = "a\u{2014}\u{202E}b\n";
    let (out, _) = left_behind("ctrm-fix-hazard-shift", ascii, shifted);
    assert!(out.ends_with("\nnotes.md:1:3 U+202E -> \"\""), "{out}");
    let bell = "a\u{7}b\n";
    let (out, code) = left_behind("ctrm-fix-hazard-bell", any, bell);
    assert_eq!((out.as_str(), code), ("notes.md:1:2 U+0007 hazard", 1));
}

/// B41: a file that is not UTF-8 fails both forms, as it fails
/// `check` (B22). It used to exit 0: `fix` never looked at skips.
#[test]
fn a_file_that_is_not_utf8_fails_fix_and_fix_check() {
    let Some(root) = fixture("ctrm-fix-not-utf8-fixture", &[]) else {
        return;
    };
    let wrote = std::fs::write(root.join("notes.md"), b"ab\xffcd\n");
    assert!(wrote.is_ok());
    let (text, code) = ran(&root, false);
    assert_eq!(code, 1, "{text}");
    let (text, code) = ran(&root, true);
    assert_eq!(code, 1, "{text}");
    let left = std::fs::read(root.join("notes.md")).unwrap_or_default();
    assert_eq!(left, b"ab\xffcd\n");
}

/// V72: a run that fails on a later file writes NOTHING, not the files
/// it judged before the failure (B25). The failure here is a file the
/// run cannot read; as root it can, and then there is no failure.
#[cfg(unix)]
#[test]
fn a_failed_run_writes_no_file_at_all() {
    let before = "a \u{2014} b\n";
    let files = [("notes.md", before), ("zz.md", "x\n")];
    let Some(root) = fixture("ctrm-fix-two-phase", &files) else {
        return;
    };
    let asked = ["notes.md".to_owned(), "zz.md".to_owned()];
    let config = crate::cli::config::discovered(&root);
    mode(&root.join("zz.md"), 0o000);
    let failed = run(&config, &asked, Format::Human, true).is_err();
    mode(&root.join("zz.md"), 0o644);
    if failed {
        assert_eq!(read(&root), before);
    }
}

#[cfg(unix)]
fn mode(path: &Path, bits: u32) {
    use std::os::unix::fs::PermissionsExt;
    let wanted = std::fs::Permissions::from_mode(bits);
    let _ = std::fs::set_permissions(path, wanted);
}

/// `.ctrm-map` is discovered beside `.ctrm` and wins over the builtin
/// for the same source (`src/rules:V19`, `src/rules:V45`).
#[test]
fn a_declared_map_entry_beats_the_builtin() {
    let files = [(".ctrm-map", "U+2014 -\n"), ("notes.md", "a \u{2014} b\n")];
    let Some(root) = fixture("ctrm-fix-map-fixture", &files) else {
        return;
    };
    let (text, _) = ran(&root, true);
    assert_eq!(read(&root), "a - b\n", "{text}");
}

/// `src/fix:V51` through the verb: the notation is left alone until a
/// `.ctrm-map` line opts in, and then it becomes words that do not
/// fuse with the letter beside them.
#[test]
fn the_words_map_rewrites_only_once_asked_for() {
    let before = "x \u{22A5}owns y\n";
    let files = [("notes.md", before)];
    let Some(root) = fixture("ctrm-fix-words-fixture", &files) else {
        return;
    };
    let _ = std::fs::remove_file(root.join(".ctrm-map"));
    let (text, _) = ran(&root, true);
    assert_eq!(read(&root), before, "{text}");
    let opted = std::fs::write(root.join(".ctrm-map"), "use words\n");
    assert!(opted.is_ok());
    let (text, _) = ran(&root, true);
    assert_eq!(read(&root), "x not owns y\n", "{text}");
}

/// `src/fix:R18`: the report is held per file now, and reads exactly as
/// it did when every row carried its own path -- files named out of
/// order come back in path order, the rewrites first, then what is left
/// in `check`'s grammar, a hazard naming `hazard` beside a set's row.
#[test]
fn two_files_report_in_path_order_rewrites_then_leftovers() {
    let files = [
        ("b.md", "x\u{2014}\u{2261}\n"),
        ("a.md", "\u{7}\u{2261}\u{2014}\n"),
    ];
    let Some(root) = fixture("ctrm-fix-two-files", &files) else {
        return;
    };
    let asked = ["b.md".to_owned(), "a.md".to_owned()];
    let config = crate::cli::config::discovered(&root);
    let report = run(&config, &asked, Format::Human, false);
    let (text, code) = report.map(|r| (r.text, r.code)).unwrap_or_default();
    let want = "a.md:1:3 U+2014 -> \"--\"\nb.md:1:2 U+2014 -> \"--\"\n\
        a.md:1:1 U+0007 hazard\na.md:1:2 U+2261 ascii\nb.md:1:3 U+2261 ascii";
    assert_eq!((text.as_str(), code), (want, 1));
}

#[path = "fix_hazard_test.rs"]
mod hazards;
