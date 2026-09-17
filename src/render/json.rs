//! The JSON form: the STABLE CONTRACT (V11).
//!
//! What that means in practice: a key does not change its name, its type or
//! its meaning under a caller, and the tests below assert whole documents
//! EXACTLY rather than probing field by field. A contract that only promises
//! "contains what you need" is not one, because nothing then catches a key
//! quietly renamed.
//!
//! One compact document per run, and pure ASCII: see `value` and `escape`.
//!
//! A document carries what the invariants NAME and no more. A violation
//! carries its position in all three units and its code point
//! (`src/scan:V12`), the set that did not contain it (the root spec's
//! interface section), and the lint name and level (`src/lint:V36`). The
//! lint's GROUP is deliberately absent: it follows from the name, and a key
//! added to a stable contract is a key that can never be taken back.

use crate::charset::{CharRange, CharSet};
use crate::render::escape::string;
use crate::render::name::{codepoint, level_name, method_name};
use crate::render::order;
use crate::render::value::{array, field, number, object, optional};
use crate::render::{Change, Explanation, FileStats, Skipped, Violation};
use crate::rules::{LevelChoice, Origin, Rule};
use crate::scan::{Hit, Unreadable};
use crate::tokens::Count;
use std::path::Path;

/// `check`: every violation, and every file that could not be read.
pub fn check(items: &[Violation<'_>], skipped: &[Skipped<'_>]) -> String {
    let found: Vec<String> = order::violations(items)
        .into_iter()
        .map(violation)
        .collect();
    object(&[
        field("verb", &string("check")),
        field("violations", &array(&found)),
        field("skipped", &unread(skipped)),
    ])
}

fn violation(item: &Violation<'_>) -> String {
    let mut fields = vec![field("path", &string(item.path))];
    fields.extend(hit_fields(item.finding.hit));
    fields.push(field("set", &string(item.set)));
    fields.push(field("lint", &string(item.finding.lint.name)));
    fields.push(field("level", &string(level_name(item.finding.level))));
    object(&fields)
}

/// The position in all three units, plus the character itself: a consumer
/// that wants to show it should not have to parse `U+XXXX` back into a code
/// point, and the escaper keeps it ASCII on the way out.
fn hit_fields(hit: Hit) -> Vec<String> {
    vec![
        field("line", &number(hit.position.line)),
        field("column", &number(hit.position.column)),
        field("byte", &number(hit.position.byte)),
        field("codepoint", &string(&codepoint(hit.character))),
        field("character", &string(&String::from(hit.character))),
    ]
}

fn unread(items: &[Skipped<'_>]) -> String {
    let rendered: Vec<String> =
        order::skipped(items).into_iter().map(unread_item).collect();
    array(&rendered)
}

fn unread_item(item: &Skipped<'_>) -> String {
    let mut fields = vec![field("path", &string(item.path))];
    fields.extend(reason_fields(item.reason));
    object(&fields)
}

fn reason_fields(reason: Unreadable) -> Vec<String> {
    match reason {
        Unreadable::NotUtf8 { byte } => vec![
            field("reason", &string("not-utf8")),
            field("byte", &number(byte)),
        ],
        Unreadable::Binary => vec![field("reason", &string("binary"))],
    }
}

/// `fix`: every rewrite, and every file that could not be read.
pub fn fix(items: &[Change<'_>], skipped: &[Skipped<'_>]) -> String {
    let rewrites: Vec<String> =
        order::changes(items).into_iter().map(change).collect();
    object(&[
        field("verb", &string("fix")),
        field("rewrites", &array(&rewrites)),
        field("skipped", &unread(skipped)),
    ])
}

fn change(item: &Change<'_>) -> String {
    let mut fields = vec![field("path", &string(item.path))];
    fields.extend(hit_fields(item.rewrite.hit));
    fields.push(field("to", &string(&item.rewrite.to)));
    object(&fields)
}

/// `stats`: one row per file.
pub fn stats(files: &[FileStats<'_>]) -> String {
    let rows: Vec<String> = order::stats(files).into_iter().map(row).collect();
    object(&[
        field("verb", &string("stats")),
        field("files", &array(&rows)),
    ])
}

fn row(item: &FileStats<'_>) -> String {
    object(&[
        field("path", &string(item.path)),
        field("outside", &number(item.outside)),
        field("bytes", &number(item.bytes)),
        field("tokens_now", &count(item.now)),
        field("tokens_after", &count(item.after)),
    ])
}

/// The method travels WITH the number (`src/tokens:V10`), as an object
/// rather than as a suffix on a string: a consumer should not have to strip
/// a tilde to get at the figure.
fn count(value: Count) -> String {
    object(&[
        field("tokens", &number(value.tokens)),
        field("method", &string(method_name(value.method))),
    ])
}

/// `explain`: the effective set, and the rule that won.
pub fn explain(item: &Explanation<'_>) -> String {
    object(&[
        field("verb", &string("explain")),
        field("path", &optional(item.path)),
        field("set", &char_set(item.set)),
        field("rule", &rule(item.rule)),
    ])
}

/// `sets`: the builtin sets and what they hold.
pub fn sets(items: &[CharSet]) -> String {
    let rendered: Vec<String> = items.iter().map(char_set).collect();
    object(&[
        field("verb", &string("sets")),
        field("sets", &array(&rendered)),
    ])
}

fn char_set(item: &CharSet) -> String {
    let ranges: Vec<String> = item.ranges.iter().map(range).collect();
    object(&[
        field("name", &string(&item.name)),
        field("ranges", &array(&ranges)),
    ])
}

/// Endpoints as `U+XXXX` rather than as bare numbers: the same spelling the
/// human form uses, and one a person can read in a raw document.
fn range(item: &CharRange) -> String {
    object(&[
        field("start", &string(&codepoint(item.start))),
        field("end", &string(&codepoint(item.end))),
    ])
}

fn rule(item: &Rule) -> String {
    let sets: Vec<String> = item.sets.iter().map(|set| string(set)).collect();
    let levels: Vec<String> = item.levels.iter().map(level_choice).collect();
    object(&[
        field("pattern", &string(&item.pattern)),
        field("sets", &array(&sets)),
        field("family", &optional(item.family.as_deref())),
        field("levels", &array(&levels)),
        field("origin", &origin(&item.origin)),
    ])
}

fn level_choice(item: &LevelChoice) -> String {
    object(&[
        field("target", &string(&item.target)),
        field("level", &string(level_name(item.level))),
    ])
}

/// A tagged union: `kind` says which case, and the remaining keys follow
/// from it.
fn origin(item: &Origin) -> String {
    match item {
        Origin::File { path, line } => file_origin(path, *line),
        Origin::Argument { index } => object(&[
            field("kind", &string("argument")),
            field("index", &number(*index)),
        ]),
        Origin::Builtin { line } => object(&[
            field("kind", &string("builtin")),
            field("line", &number(*line)),
        ]),
    }
}

/// The path is rendered LOSSILY. What a non-UTF-8 path should print is a
/// rendering question and this is the honest answer: the replacement
/// character, which the escaper then writes as `\ufffd` -- visibly wrong in
/// the output rather than silently dropped from it.
fn file_origin(path: &Path, line: usize) -> String {
    object(&[
        field("kind", &string("file")),
        field("path", &string(&path.to_string_lossy())),
        field("line", &number(line)),
    ])
}

#[cfg(test)]
mod tests {
    use super::{check, explain, fix, sets, stats};
    use crate::charset::{CharRange, CharSet};
    use crate::fix::Rewrite;
    use crate::lint::{Finding, Group, Level, Lint};
    use crate::render::{Change, Explanation, FileStats, Skipped, Violation};
    use crate::rules::{LevelChoice, Origin, Rule};
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
            origin: Origin::File {
                path: PathBuf::from(".ctrm"),
                line: 4,
            },
        }
    }

    fn explained(path: Option<&str>) -> String {
        let set = CharSet {
            name: String::from("ascii"),
            ranges: vec![],
        };
        let rule = file_rule();
        explain(&Explanation {
            path,
            set: &set,
            rule: &rule,
        })
    }

    #[test]
    fn the_check_document_is_the_contract() {
        let expected = concat!(
            r#"{"verb":"check","violations":[{"path":"src/a.rs","#,
            r#""line":2,"column":5,"byte":7,"codepoint":"U+2014","#,
            r#""character":"\u2014","set":"ascii","lint":"charset","#,
            r#""level":"deny"}],"skipped":[]}"#
        );
        assert_eq!(check(&[em_dash()], &[]), expected);
    }

    #[test]
    fn an_empty_check_still_carries_every_key() {
        let expected = r#"{"verb":"check","violations":[],"skipped":[]}"#;
        assert_eq!(check(&[], &[]), expected);
    }

    #[test]
    fn unread_files_are_tagged_objects_in_path_order() {
        let items = [skip("b.bin"), broken("a.txt", 17)];
        let expected = concat!(
            r#"{"verb":"check","violations":[],"skipped":["#,
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
        let change = Change {
            path: "a.rs",
            rewrite,
        };
        let expected = concat!(
            r#"{"verb":"fix","rewrites":[{"path":"a.rs","line":2,"#,
            r#""column":5,"byte":7,"codepoint":"U+2014","#,
            r#""character":"\u2014","to":"--"}],"skipped":[]}"#
        );
        assert_eq!(fix(&[change], &[]), expected);
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
        let expected = concat!(
            r#"{"verb":"stats","files":[{"path":"a.rs","outside":3,"#,
            r#""bytes":120,"tokens_now":{"tokens":40,"#,
            r#""method":"estimate"},"tokens_after":{"tokens":38,"#,
            r#""method":"bpe"}}]}"#
        );
        assert_eq!(stats(&[row]), expected);
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
            r#"{"verb":"sets","sets":[{"name":"ascii","ranges":["#,
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
            r#"{"verb":"explain","path":"a.rs","set":{"name":"ascii","#,
            r#""ranges":[]},"rule":{"pattern":"docs/**","#,
            r#""sets":["ascii"],"family":"dash","levels":["#,
            r#"{"target":"pedantic","level":"warn"}],"origin":{"#,
            r#""kind":"file","path":".ctrm","line":4}}}"#
        );
        assert_eq!(explained(Some("a.rs")), expected);
    }

    #[test]
    fn a_quote_in_a_path_is_escaped_inside_the_document() {
        let items = [skip("a\"b")];
        let expected = concat!(
            r#"{"verb":"check","violations":[],"skipped":["#,
            r#"{"path":"a\"b","reason":"binary"}]}"#
        );
        assert_eq!(check(&[], &items), expected);
    }
}
