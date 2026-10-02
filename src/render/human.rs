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
use crate::lint::Group;
use crate::render::Violation;
use crate::render::name::{codepoint, level_name};
use crate::render::order;
use crate::render::{Change, Explanation, FileStats, InForce, Skipped};
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
        shown(item.path),
        at.line,
        at.column,
        codepoint(item.finding.hit.character),
        verdict(item)
    )
}

/// A path as a terminal may safely print it (V11): every character outside
/// printable ASCII is spelled `<U+XXXX>`, as `src/cli:V53` spells a guard
/// reason. A tracked file can be named `e<ESC>[2Jx<U+202E>y.md`, and a
/// report that echoed it raw would clear the screen or reverse the line
/// it sits on (B31). The json form needs none of this: it escapes.
fn shown(path: &str) -> String {
    path.chars()
        .map(|c| match c {
            ' '..='~' => String::from(c),
            _ => format!("<{}>", codepoint(c)),
        })
        .collect()
}

/// The last word of the line: the set the character was judged against,
/// or, for a PEDANTIC finding, the lint that fired (`src/lint:V55`).
///
/// A charset or hazard finding is told apart by its code point plus the
/// set column. A pedantic one is not: U+0020 is trailing whitespace on
/// one line and the last character of an unterminated file on another,
/// and the set in force -- which GRANTS it either way -- would read as
/// though the space fell outside it. The lint name is the one word that
/// says what is wrong. The json keeps `set` and `lint` apart, as before.
fn verdict<'v>(item: &'v Violation<'_>) -> &'v str {
    if item.finding.lint.group == Group::Pedantic {
        item.finding.lint.name
    } else {
        item.set
    }
}

fn unread_lines(items: &[Skipped<'_>]) -> Vec<String> {
    order::skipped(items).into_iter().map(unread).collect()
}

/// An unread file is NAMED (`src/scan:V8`): one that reported nothing would
/// read exactly like a clean one.
fn unread(item: &Skipped<'_>) -> String {
    match item.reason {
        Unreadable::NotUtf8 { byte } => {
            format!("{}: invalid UTF-8 at byte {byte}", shown(item.path))
        }
        Unreadable::Binary => format!("{}: skipped, binary", shown(item.path)),
    }
}

/// `fix`: what was rewritten, and what it became; then what no map entry
/// covers, as `check` would print it, since that is what a reader goes to
/// look at next (`src/fix:V4`).
pub fn fix(
    items: &[Change<'_>],
    unmapped: &[Violation<'_>],
    skipped: &[Skipped<'_>],
) -> String {
    let mut lines: Vec<String> =
        order::changes(items).into_iter().map(change).collect();
    lines.extend(order::violations(unmapped).into_iter().map(violation));
    lines.extend(unread_lines(skipped));
    lines.join("\n")
}

fn change(item: &Change<'_>) -> String {
    let at = item.rewrite.hit.position;
    format!(
        "{}:{}:{} {} -> {:?}",
        shown(item.path),
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
        shown(item.path),
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
        format!(
            "path {}",
            item.path.map_or_else(|| "(whole repo)".into(), shown)
        ),
        format!("set {}", item.set.name),
    ];
    match item.rule {
        Some(rule) => lines.extend(rule_lines(rule)),
        // V1: the strict default, with no line behind it to name.
        None => lines.push(String::from("rule none -- no rule grants a set")),
    }
    lines.extend(in_force(&item.in_force));
    lines.join("\n")
}

/// What is in force, each with the line that set it (`src/rules:V20`),
/// which need not be the winner's (B28). The family always has an answer,
/// so it always prints; a level nobody set does not.
fn in_force(item: &InForce<'_>) -> Vec<String> {
    let named = item.family_origin.map_or_else(|| "default".into(), origin);
    let mut lines = vec![format!("effective family {} {named}", item.family)];
    if let Some(bare) = &item.level {
        let (level, at) = (level_name(bare.value), origin(bare.origin));
        lines.push(format!("effective level {level} {at}"));
    }
    for choice in &item.levels {
        let (target, at) = (&choice.value.target, origin(choice.origin));
        let level = level_name(choice.value.level);
        lines.push(format!("effective level {target}={level} {at}"));
    }
    lines
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
        format!("level {}", rule.default_level.map_or("none", level_name)),
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

/// The ONE spelling `src/rules:V20` fixes: `<file>:<line>`,
/// `argv[<n>]`, `builtin:<line>`.
///
/// Spelled here rather than called from `src/rules::describe`, because
/// this node calls no sibling (`src:V39`) -- and kept identical to it,
/// because V20 says one spelling serves a parse error and what `explain`
/// prints. This renderer used to say `.ctrm line 2` while an error about
/// the same line said `.ctrm:2`, which is exactly the drift V20 forbids.
fn origin(item: &Origin) -> String {
    match item {
        Origin::File { path, line } => {
            format!("{}:{line}", shown(&path.to_string_lossy()))
        }
        Origin::Argument { index } => format!("argv[{index}]"),
        Origin::Builtin { line } => format!("builtin:{line}"),
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
    use crate::render::Violation;
    use crate::render::{Change, Explanation, FileStats, InForce, Skipped};
    use crate::rules::{LevelChoice, Origin, Rule, Sourced};
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
        explained_with(InForce {
            family: "text",
            family_origin: None,
            level: None,
            levels: vec![],
        })
    }

    fn explained_with(in_force: InForce<'_>) -> String {
        let set = CharSet {
            name: String::from("ascii"),
            ranges: vec![],
        };
        let rule = builtin_rule();
        explain(&Explanation {
            path: None,
            set: &set,
            rule: Some(&rule),
            in_force,
        })
    }

    #[test]
    fn a_violation_is_path_line_column_codepoint_then_set() {
        assert_eq!(check(&[em_dash()], &[]), "src/a.rs:2:5 U+2014 ascii");
    }

    /// B31: a file name carrying an escape sequence and a bidi override
    /// reaches the terminal spelled out, on every line that names a path.
    #[test]
    fn a_path_never_carries_a_control_or_bidi_character_out() {
        let path = "e\u{1b}[2Jx\u{202e}y.md";
        let safe = "e<U+001B>[2Jx<U+202E>y.md";
        let mut item = em_dash();
        item.path = path;
        let said = check(&[item], &[skip(path)]);
        let want = format!("{safe}:2:5 U+2014 ascii\n{safe}: skipped, binary");
        assert_eq!(said, want);
        assert_eq!(super::shown("za\u{17c}.md"), "za<U+017C>.md");
    }

    /// A pedantic finding names its LINT where the set would go: the set
    /// in force granted the character, so naming it explains nothing.
    #[test]
    fn a_pedantic_finding_names_its_lint_in_the_last_column() {
        let mut item = em_dash();
        item.finding.lint = Lint::new("trailing-whitespace", Group::Pedantic);
        item.finding.hit.character = ' ';
        let expected = "src/a.rs:2:5 U+0020 trailing-whitespace";
        assert_eq!(check(&[item], &[]), expected);
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
        assert_eq!(fix(&[change], &[], &[]), "a.rs:2:5 U+2014 -> \"--\"");
    }

    /// B23: a character `fix` could not rewrite is printed in `check`'s
    /// row grammar, after the rewrites.
    #[test]
    fn an_unmapped_character_is_a_check_row() {
        let rewrite = Rewrite {
            hit: hit(),
            to: String::from("--"),
        };
        let change = Change {
            path: "a.rs",
            rewrite,
        };
        let said = fix(&[change], &[em_dash()], &[]);
        assert_eq!(
            said,
            "a.rs:2:5 U+2014 -> \"--\"\nsrc/a.rs:2:5 U+2014 ascii"
        );
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
            "family none\nlevels none\nlevel none\norigin builtin:3\n",
            "effective family text default"
        );
        assert_eq!(explained(), expected);
    }

    /// B28: the family and the levels in force are named with the line
    /// that set them, which here is not the winner's.
    #[test]
    fn explain_names_what_is_in_force_and_where_each_came_from() {
        let at = Origin::Argument { index: 5 };
        let said = explained_with(InForce {
            family: "emoji",
            family_origin: Some(&at),
            level: Some(Sourced {
                value: Level::Allow,
                origin: &at,
            }),
            levels: vec![pedantic_warn(&at)],
        });
        let tail = concat!(
            "origin builtin:3\neffective family emoji argv[5]\neffective ",
            "level allow argv[5]\neffective level pedantic=warn argv[5]"
        );
        assert!(said.ends_with(tail), "{said}");
    }

    fn pedantic_warn(origin: &Origin) -> Sourced<'_, LevelChoice> {
        let value = LevelChoice {
            target: String::from("pedantic"),
            level: Level::Warn,
        };
        Sourced { value, origin }
    }
}
