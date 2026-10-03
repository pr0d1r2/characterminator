//! The tests of `json.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `json`.

use super::{check, error, explain, fix, sets, stats};
use crate::charset::{CharRange, CharSet};
use crate::fix::Rewrite;
use crate::lint::{Finding, Group, Level, Lint};
use crate::render::Violation;
use crate::render::{Change, Explanation, FileStats, InForce, Skipped};
use crate::rules::{LevelChoice, Origin, Rule, Sourced};
use crate::scan::{Hit, Position, Unreadable};
use crate::tokens::{Count, Method};
use std::path::PathBuf;

const CHARSET: Lint = Lint {
    name: "charset",
    group: Group::Charset,
};

/// U+2014 EM DASH, written as an escape because the source is ASCII.
fn hit() -> Hit {
    let position = Position {
        line: 2,
        column: 5,
        byte: 7,
    };
    Hit {
        position,
        character: '\u{2014}',
    }
}

fn em_dash() -> Violation<'static> {
    let finding = Finding {
        hit: hit(),
        lint: CHARSET,
        level: Level::Deny,
    };
    Violation {
        path: "src/a.rs",
        finding,
        set: "ascii",
    }
}

fn skip(path: &str) -> Skipped<'_> {
    Skipped {
        path,
        reason: Unreadable::Binary,
    }
}

fn broken(path: &str, byte: usize) -> Skipped<'_> {
    Skipped {
        path,
        reason: Unreadable::NotUtf8 { byte },
    }
}

fn estimate(tokens: u64) -> Count {
    Count {
        tokens,
        method: Method::Estimate,
    }
}

fn bpe(tokens: u64) -> Count {
    Count {
        tokens,
        method: Method::Bpe,
    }
}

fn pedantic_warn() -> LevelChoice {
    LevelChoice {
        target: String::from("pedantic"),
        level: Level::Warn,
    }
}

fn file_rule() -> Rule {
    Rule {
        pattern: String::from("docs/**"),
        sets: vec![String::from("ascii")],
        family: Some(String::from("dash")),
        levels: vec![pedantic_warn()],
        default_level: None,
        origin: Origin::File {
            path: PathBuf::from(".ctrm"),
            line: 4,
        },
    }
}

fn explained(path: Option<&str>) -> String {
    explained_by(path, &file_rule())
}

fn explained_by(path: Option<&str>, rule: &Rule) -> String {
    let set = CharSet {
        name: String::from("ascii"),
        ranges: vec![],
    };
    explain(&Explanation {
        path,
        set: &set,
        rule: Some(rule),
        in_force: alone(rule),
    })
}

/// What one rule puts in force when it is the only one that matched.
fn alone(rule: &Rule) -> InForce<'_> {
    let at = &rule.origin;
    let sourced = |value| Sourced { value, origin: at };
    InForce {
        family: rule.family.as_deref().unwrap_or("text"),
        family_origin: rule.family.as_ref().map(|_| at),
        level: rule.default_level.map(sourced),
        levels: rule
            .levels
            .iter()
            .map(|c| Sourced {
                value: c.clone(),
                origin: at,
            })
            .collect(),
    }
}

#[test]
fn the_check_document_is_the_contract() {
    let expected = concat!(
        r#"{"schema":1,"verb":"check","violations":[{"path":"src/a.rs","#,
        r#""line":2,"column":5,"byte":7,"codepoint":"U+2014","#,
        r#""character":"\u2014","set":"ascii","lint":"charset","#,
        r#""level":"deny","replacement":null,"fixable":false}],"skipped":[]}"#
    );
    assert_eq!(check(&[em_dash()], &[]), expected);
}

/// V122: a run that failed is a document too, schema first.
#[test]
fn an_error_is_a_document_with_its_verb() {
    let expected = r#"{"schema":1,"verb":"fix","error":"bad \"x\""}"#;
    assert_eq!(error("fix", "bad \"x\""), expected);
}

#[test]
fn an_empty_check_still_carries_every_key() {
    let expected =
        r#"{"schema":1,"verb":"check","violations":[],"skipped":[]}"#;
    assert_eq!(check(&[], &[]), expected);
}

#[test]
fn unread_files_are_tagged_objects_in_path_order() {
    let items = [skip("b.bin"), broken("a.txt", 17)];
    let expected = concat!(
        r#"{"schema":1,"verb":"check","violations":[],"skipped":["#,
        r#"{"path":"a.txt","reason":"not-utf8","byte":17},"#,
        r#"{"path":"b.bin","reason":"binary"}]}"#
    );
    assert_eq!(check(&[], &items), expected);
}

#[test]
fn a_rewrite_document_carries_the_replacement_text() {
    let rewrite = Rewrite {
        hit: hit(),
        to: String::from("--"),
    };
    let change = Change::plain("a.rs", &rewrite);
    let expected = concat!(
        r#"{"schema":1,"verb":"fix","rewrites":[{"path":"a.rs","line":2,"#,
        r#""column":5,"byte":7,"codepoint":"U+2014","#,
        r#""character":"\u2014","to":"--","set":"ascii","lint":"outside-set","#,
        r#""level":"deny","replacement":"--","fixable":true}],"unmapped":[],"#,
        r#""skipped":[]}"#
    );
    assert_eq!(fix(&[change], [], &[]), expected);
}

/// B23: what no map entry covers is in the document, as `check`'s
/// violation, so a caller learns WHY `fix` exited 1.
#[test]
fn a_fix_document_carries_the_unmapped_characters() {
    let expected = concat!(
        r#"{"schema":1,"verb":"fix","rewrites":[],"unmapped":[{"path":"src/a.rs","#,
        r#""line":2,"column":5,"byte":7,"codepoint":"U+2014","#,
        r#""character":"\u2014","set":"ascii","lint":"charset","#,
        r#""level":"deny","replacement":null,"fixable":false}],"skipped":[]}"#
    );
    assert_eq!(
        fix(&[], crate::render::order::lines(&[em_dash()]), &[]),
        expected
    );
}

#[test]
fn a_stats_row_carries_each_count_beside_its_method() {
    let row = FileStats {
        path: "a.rs",
        outside: 3,
        bytes: 120,
        now: estimate(40),
        after: bpe(38),
    };
    let figures = concat!(
        r#""bytes":120,"tokens_now":{"tokens":40,"method":"estimate"},"#,
        r#""tokens_after":{"tokens":38,"method":"bpe"}"#,
    );
    let expected = format!(
        r#"{{"schema":1,"verb":"stats","files":[{{"path":"a.rs","outside":3,{figures}}}],"total":{{"files":1,"outside":3,{figures}}},"skipped":[]}}"#
    );
    assert_eq!(stats(&[row], &[]), expected);
}

/// B26: a file that is not text is named in `skipped`, as in `check`.
#[test]
fn a_stats_document_names_the_files_it_skipped() {
    let expected = concat!(
        r#"{"schema":1,"verb":"stats","files":[],"total":null,"skipped":["#,
        r#"{"path":"a.txt","reason":"not-utf8","byte":17},"#,
        r#"{"path":"b.bin","reason":"binary"}]}"#
    );
    let items = [skip("b.bin"), broken("a.txt", 17)];
    assert_eq!(stats(&[], &items), expected);
}

#[test]
fn a_set_renders_its_ranges_as_code_points() {
    let range = CharRange {
        start: '\u{0}',
        end: '\u{7f}',
    };
    let ascii = CharSet {
        name: String::from("ascii"),
        ranges: vec![range],
    };
    let expected = concat!(
        r#"{"schema":1,"verb":"sets","sets":[{"name":"ascii","ranges":["#,
        r#"{"start":"U+0000","end":"U+007F"}]}]}"#
    );
    assert_eq!(sets(&[ascii]), expected);
}

#[test]
fn an_absent_path_is_null_rather_than_a_missing_key() {
    assert!(explained(None).contains(r#""path":null"#));
}

#[test]
fn an_explanation_names_the_set_and_the_winning_rule() {
    let expected = concat!(
        r#"{"schema":1,"verb":"explain","path":"a.rs","set":{"name":"ascii","#,
        r#""ranges":[]},"rule":{"pattern":"docs/**","#,
        r#""sets":["ascii"],"family":"dash","levels":["#,
        r#"{"target":"pedantic","level":"warn"}],"#,
        r#""default_level":null,"origin":{"#,
        r#""kind":"file","path":".ctrm","line":4}},"#,
        r#""effective":{"family":{"name":"dash","origin":{"#,
        r#""kind":"file","path":".ctrm","line":4}},"#,
        r#""default_level":null,"levels":[{"target":"pedantic","#,
        r#""level":"warn","origin":{"#,
        r#""kind":"file","path":".ctrm","line":4}}]}}"#
    );
    assert_eq!(explained(Some("a.rs")), expected);
}

#[test]
fn a_bare_level_is_carried_as_the_rules_default_level() {
    let rule = Rule {
        default_level: Some(Level::Warn),
        ..file_rule()
    };
    let expected = concat!(
        r#"{"schema":1,"verb":"explain","path":null,"set":{"name":"ascii","#,
        r#""ranges":[]},"rule":{"pattern":"docs/**","#,
        r#""sets":["ascii"],"family":"dash","levels":["#,
        r#"{"target":"pedantic","level":"warn"}],"#,
        r#""default_level":"warn","origin":{"#,
        r#""kind":"file","path":".ctrm","line":4}},"#,
        r#""effective":{"family":{"name":"dash","origin":{"#,
        r#""kind":"file","path":".ctrm","line":4}},"#,
        r#""default_level":{"level":"warn","origin":{"#,
        r#""kind":"file","path":".ctrm","line":4}},"#,
        r#""levels":[{"target":"pedantic","level":"warn","origin":{"#,
        r#""kind":"file","path":".ctrm","line":4}}]}}"#
    );
    assert_eq!(explained_by(None, &rule), expected);
}

#[test]
fn a_quote_in_a_path_is_escaped_inside_the_document() {
    let items = [skip("a\"b")];
    let expected = concat!(
        r#"{"schema":1,"verb":"check","violations":[],"skipped":["#,
        r#"{"path":"a\"b","reason":"binary"}]}"#
    );
    assert_eq!(check(&[], &items), expected);
}
