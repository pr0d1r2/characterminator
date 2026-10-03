//! The `Read` of a file that is not plain text: bytes that are not
//! UTF-8 (V66, B17) and a file past the 16 MiB cap (V102). A child of
//! `adapter_test.rs`, reaching its fixtures as `super`.

use super::{answer_in, read_payload, tree};

/// The answer to a `Read` of one file holding raw `bytes`.
fn read_of_bytes(name: &str, ctrm: &str, bytes: &[u8]) -> String {
    let Some(root) = tree(name, &[(".ctrm", ctrm)]) else {
        return String::from("(disk refused)");
    };
    let full = root.join("f.rs");
    if std::fs::write(&full, bytes).is_err() {
        return String::from("(disk refused)");
    }
    answer_in(&read_payload(&root, &full.to_string_lossy()), &root)
}

/// B17: a file that is not text is still shown to the model, lossily
/// decoded, so a hazard in it denies the read (V66) -- after a stray
/// `\xff`, after a NUL, and under a `.ctrm` that cannot be applied.
#[test]
fn a_hazard_in_a_file_that_is_not_text_is_denied() {
    let utf8 = b"ok\nlet x = \"\xe2\x80\xae\";\n\xff\n";
    let got = read_of_bytes("ctrm-guard-notutf8", "", utf8);
    assert!(got.contains("f.rs:2:10 U+202E bidi-control"), "{got}");
    let nul = b"a\0\xe2\x80\xae";
    let broken = "*.rs nosuchset";
    for (name, ctrm) in [("ctrm-guard-nul", ""), ("ctrm-guard-bad", broken)] {
        let got = read_of_bytes(name, ctrm, nul);
        assert!(got.contains("f.rs:1:3 U+202E bidi-control"), "{got}");
    }
}

/// No hazard but the NUL and the bytes a binary is made of: the read
/// goes ahead, with a note that no rule could apply.
#[test]
fn a_binary_file_without_a_smuggling_hazard_passes_with_a_note() {
    let got =
        read_of_bytes("ctrm-guard-binary", "", b"\x89PNG\r\n\x1a\n\0\0\xff");
    assert!(got.contains("is not text"), "{got}");
    assert!(!got.contains("permissionDecision"), "{got}");
}

/// A SPARSE file: `head` at byte 0, `tail` at `at`, nothing (NUL, no
/// disk) between. Its read answer, under a `.ctrm` of `* ascii`.
fn read_of_sparse(name: &str, head: &str, at: u64, tail: &str) -> String {
    use std::io::{Seek, SeekFrom, Write};
    let Some(root) = tree(name, &[(".ctrm", "* ascii\n")]) else {
        return String::from("(disk refused)");
    };
    let full = root.join("big.log");
    let written = std::fs::File::create(&full).and_then(|mut f| {
        f.write_all(head.as_bytes())?;
        f.seek(SeekFrom::Start(at))?;
        f.write_all(tail.as_bytes())
    });
    if written.is_err() {
        return String::from("(disk refused)");
    }
    answer_in(&read_payload(&root, &full.to_string_lossy()), &root)
}

/// V102: past the 16 MiB cap only a prefix is judged. A hazard OUTSIDE
/// it is not found -- read whole, the NUL gap and the U+202E would deny
/// -- so the read passes, never in silence: the note says the rest was
/// not judged.
#[test]
fn a_file_over_the_cap_is_judged_by_its_prefix_and_says_so() {
    let past = super::super::CAP.saturating_add(10);
    let got =
        read_of_sparse("ctrm-guard-cap-tail", "plain\n", past, "\u{202E}\n");
    assert!(!got.contains("permissionDecision"), "{got}");
    assert!(got.contains("16 MiB cap"), "{got}");
    assert!(got.contains("the rest was NOT"), "{got}");
}

/// A hazard INSIDE the prefix still denies: the cap bounds memory, it
/// opens no door.
#[test]
fn a_hazard_in_the_prefix_of_a_capped_file_still_denies() {
    let past = super::super::CAP.saturating_add(10);
    let got =
        read_of_sparse("ctrm-guard-cap-head", "a\u{202E}b\n", past, "x\n");
    assert!(got.contains(r#""permissionDecision":"deny""#), "{got}");
    assert!(got.contains("big.log:1:2 U+202E"), "{got}");
}

/// The cut lands after the last newline inside the cap, else after the
/// last whole UTF-8 character, so it never splits one; a file within
/// the cap is read whole.
#[test]
fn the_prefix_is_cut_at_a_line_or_a_whole_character() {
    use super::super::cut;
    assert_eq!(cut(b"ab\ncd\nef", 7), 6);
    // U+2014 is 3 bytes at 1..4; a cap of 3 would split it.
    assert_eq!(cut("a\u{2014}b".as_bytes(), 3), 1);
    assert_eq!(cut(b"abcdef", 4), 4);
    let root = tree("ctrm-guard-cap-small", &[("s.md", "ab\ncd\n")]);
    let file = root.unwrap_or_default().join("s.md");
    let whole = super::super::loaded(&file, 6).map(|(b, w)| (b.len(), w));
    assert_eq!(whole, Some((6, true)));
    let capped = super::super::loaded(&file, 5).map(|(b, w)| (b.len(), w));
    assert_eq!(capped, Some((3, false)));
}
