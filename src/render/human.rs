//! The HUMAN form: one line per thing.
//!
//! Cosmetic by V11. Nothing here is a contract, and the tests assert only
//! the one shape the root spec's interface section fixes --
//! `path:line:col U+XXXX <set>`. Everything else is free to change the day a
//! reader is better served by something else, which is exactly the freedom
//! the json form does not have.
//!
//! The offending character is printed as `U+XXXX` rather than literally: a
//! tool that hunts invisible code points must not put one in its own report,
//! where it would be as invisible as it was in the file.
//!
//! Renderers return text with NO trailing newline, and an empty report as
//! the empty string, so the caller owns its own line endings.

use crate::charset::{CharRange, CharSet};
use crate::render::name::{codepoint, level_name};
use crate::render::order;
use crate::render::{Change, Explanation, FileStats, Skipped, Violation};
use crate::rules::{LevelChoice, Origin, Rule};
use crate::scan::Unreadable;
use crate::tokens::{Count, Method};

/// `check`: every violation, then every file that could not be read.
pub fn check(items: &[Violation<'_>], skipped: &[Skipped<'_>]) -> String {
    let mut lines: Vec<String> = order::violations(items)
        .into_iter()
        .map(violation)
        .collect();
    lines.extend(unread_lines(skipped));
    lines.join("\n")
}

/// `path:line:col U+XXXX <set>`, the interface section's shape verbatim.
fn violation(item: &Violation<'_>) -> String {
    let at = item.finding.hit.position;
    format!(
        "{}:{}:{} {} {}",
        item.path,
        at.line,
        at.column,
        codepoint(item.finding.hit.character),
        item.set
    )
}

fn unread_lines(items: &[Skipped<'_>]) -> Vec<String> {
    order::skipped(items).into_iter().map(unread).collect()
}

/// An unread file is NAMED (`src/scan:V8`): one that reported nothing would
/// read exactly like a clean one.
fn unread(item: &Skipped<'_>) -> String {
    match item.reason {
        Unreadable::NotUtf8 { byte } => {
            format!("{}: invalid UTF-8 at byte {byte}", item.path)
        }
        Unreadable::Binary => format!("{}: skipped, binary", item.path),
    }
}

/// `fix`: what was rewritten, and what it became.
pub fn fix(items: &[Change<'_>], skipped: &[Skipped<'_>]) -> String {
    let mut lines: Vec<String> =
        order::changes(items).into_iter().map(change).collect();
    lines.extend(unread_lines(skipped));
    lines.join("\n")
}

fn change(item: &Change<'_>) -> String {
    let at = item.rewrite.hit.position;
    format!(
        "{}:{}:{} {} -> {:?}",
        item.path,
        at.line,
        at.column,
        codepoint(item.rewrite.hit.character),
        item.rewrite.to
    )
}

/// `stats`: one row per file.
pub fn stats(files: &[FileStats<'_>]) -> String {
    let rows: Vec<String> = order::stats(files).into_iter().map(row).collect();
    rows.join("\n")
}

fn row(item: &FileStats<'_>) -> String {
    format!(
        "{} outside {} bytes {} tokens {} -> {}",
        item.path,
        item.outside,
        item.bytes,
        count(item.now),
        count(item.after)
    )
}

/// The figure says how it was measured (`src/tokens:V10`): a tilde is the
/// cheap proxy, a real count names the tokenizer. Neither can be mistaken
/// for the other at a glance, which is the whole point.
fn count(value: Count) -> String {
    match value.method {
        Method::Estimate => format!("~{}", value.tokens),
        Method::Bpe => format!("{} (o200k)", value.tokens),
    }
}

/// `explain`: the effective set, and the rule that won.
pub fn explain(item: &Explanation<'_>) -> String {
    let mut lines = vec![
        format!("path {}", item.path.unwrap_or("(whole repo)")),
        format!("set {}", item.set.name),
    ];
    lines.extend(rule_lines(item.rule));
    lines.join("\n")
}

/// The rule is DESCRIBED in labelled lines rather than written back as a
/// `.ctrm` line. That grammar belongs to `src/rules`, and rendering config
/// syntax is `explain --as lines`, which is `src/cli:T34` -- reproducing it
/// here would mean two nodes owning one grammar.
fn rule_lines(rule: &Rule) -> Vec<String> {
    vec![
        format!("pattern {}", rule.pattern),
        format!("sets {}", words(&rule.sets)),
        format!("family {}", rule.family.as_deref().unwrap_or("none")),
        format!("levels {}", levels(&rule.levels)),
        format!("origin {}", origin(&rule.origin)),
    ]
}

fn levels(items: &[LevelChoice]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|item| format!("{}={}", item.target, level_name(item.level)))
        .collect();
    words(&parts)
}

fn origin(item: &Origin) -> String {
    match item {
        Origin::File { path, line } => {
            format!("{} line {line}", path.display())
        }
        Origin::Argument { index } => format!("argument {index}"),
        Origin::Builtin { line } => format!("builtin line {line}"),
    }
}

/// `sets`: each set and the ranges it is built from.
pub fn sets(items: &[CharSet]) -> String {
    let lines: Vec<String> = items.iter().map(set).collect();
    lines.join("\n")
}

fn set(item: &CharSet) -> String {
    let ranges: Vec<String> = item.ranges.iter().map(range).collect();
    format!("{} {}", item.name, words(&ranges))
}

fn range(item: &CharRange) -> String {
    if item.start == item.end {
        return codepoint(item.start);
    }
    format!("{}-{}", codepoint(item.start), codepoint(item.end))
}

/// Space-joined, or `none` when there is nothing to join: a label with
/// nothing after it reads like the renderer gave up.
fn words(items: &[String]) -> String {
    if items.is_empty() {
        return String::from("none");
    }
    items.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{check, count, explain, fix, sets, stats};
    use crate::charset::{CharRange, CharSet};
    use crate::fix::Rewrite;
    use crate::lint::{Finding, Group, Level, Lint};
    use crate::render::{Change, Explanation, FileStats, Skipped, Violation};
    use crate::rules::{Origin, Rule};
    use crate::scan::{Hit, Position, Unreadable};
    use crate::tokens::{Count, Method};

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

    fn builtin_rule() -> Rule {
        Rule {
            pattern: String::from("docs/**"),
            sets: vec![String::from("ascii")],
            family: None,
            levels: vec![],
            default_level: None,
            origin: Origin::Builtin { line: 3 },
        }
    }

    fn explained() -> String {
        let set = CharSet {
            name: String::from("ascii"),
            ranges: vec![],
        };
        let rule = builtin_rule();
        explain(&Explanation {
            path: None,
            set: &set,
            rule: &rule,
        })
    }

    #[test]
    fn a_violation_is_path_line_column_codepoint_then_set() {
        assert_eq!(check(&[em_dash()], &[]), "src/a.rs:2:5 U+2014 ascii");
    }

    #[test]
    fn an_empty_report_is_the_empty_string() {
        assert_eq!(check(&[], &[]), "");
    }

    #[test]
    fn an_unread_file_is_named_rather_than_dropped() {
        let items = [skip("b.bin"), broken("a.txt", 17)];
        let expected =
            "a.txt: invalid UTF-8 at byte 17\nb.bin: skipped, binary";
        assert_eq!(check(&[], &items), expected);
    }

    #[test]
    fn an_estimate_wears_a_tilde() {
        assert_eq!(count(estimate(40)), "~40");
    }

    #[test]
    fn a_real_count_names_its_tokenizer_instead() {
        assert_eq!(count(bpe(40)), "40 (o200k)");
    }

    #[test]
    fn a_stats_row_carries_both_figures_with_their_methods() {
        let row = FileStats {
            path: "a.rs",
            outside: 3,
            bytes: 120,
            now: estimate(40),
            after: bpe(38),
        };
        let expected = "a.rs outside 3 bytes 120 tokens ~40 -> 38 (o200k)";
        assert_eq!(stats(&[row]), expected);
    }

    #[test]
    fn a_rewrite_shows_what_replaces_the_character() {
        let rewrite = Rewrite {
            hit: hit(),
            to: String::from("--"),
        };
        let change = Change {
            path: "a.rs",
            rewrite,
        };
        assert_eq!(fix(&[change], &[]), "a.rs:2:5 U+2014 -> \"--\"");
    }

    #[test]
    fn a_set_prints_its_ranges_as_code_points() {
        let range = CharRange {
            start: '\u{0}',
            end: '\u{7f}',
        };
        let ascii = CharSet {
            name: String::from("ascii"),
            ranges: vec![range],
        };
        assert_eq!(sets(&[ascii]), "ascii U+0000-U+007F");
    }

    #[test]
    fn explain_labels_every_line_rather_than_writing_config_syntax() {
        let expected = concat!(
            "path (whole repo)\nset ascii\npattern docs/**\nsets ascii\n",
            "family none\nlevels none\norigin builtin line 3"
        );
        assert_eq!(explained(), expected);
    }
}
