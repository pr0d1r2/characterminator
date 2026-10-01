//! The SARIF form: `check`'s findings as a SARIF 2.1.0 log (V50), the
//! format GitHub code scanning reads.
//!
//! Written with the same primitives as the json contract (`value`,
//! `escape`), so it is compact, pure ASCII and needs no dependency. It is
//! DETERMINISTIC: results go out in the one report order (`order`), and
//! the log carries no timestamp, no absolute root and nothing else that
//! differs between two runs over one tree.
//!
//! The `rules` array is the lint REGISTRY (`crate::lint::LINTS`) read as
//! data, each row's group name and default level taken from the lint
//! vocabulary's own accessors. A lint registered there -- a hazard lint,
//! say -- reaches code scanning with no edit here, which a hand-written
//! list could not promise.
//!
//! COLUMNS. SARIF reads `startColumn` as UTF-16 code units unless the run
//! says otherwise, and `src/scan` counts CHARACTERS (Unicode scalar
//! values). The two disagree on every line holding an astral character
//! before the hit, so the run declares `columnKind: unicodeCodePoints`,
//! which is what the scanner actually counts.
//!
//! LINES. The scanner ends a line at LF only. A consumer that also breaks
//! at a lone CR places a hit in such a file on a different line; a CRLF
//! file reads the same either way.

use crate::lint::{LINTS, Level, Lint};
use crate::render::escape::string;
use crate::render::name::codepoint;
use crate::render::order;
use crate::render::value::{array, field, number, object};
use crate::render::{Skipped, Violation};
use crate::scan::{Position, Unreadable};

/// The published schema for the version below, so a validator can find
/// it without being told.
const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// The one SARIF version this writes.
const VERSION: &str = "2.1.0";

/// The tool as a reader invokes it: the binary, not the crate.
const NAME: &str = "ctrm";

/// `check`: one run, every violation a result, every unread file a
/// notification.
pub fn check(items: &[Violation<'_>], skipped: &[Skipped<'_>]) -> String {
    object(&[
        field("$schema", &string(SCHEMA)),
        field("version", &string(VERSION)),
        field("runs", &array(&[run(items, skipped)])),
    ])
}

fn run(items: &[Violation<'_>], skipped: &[Skipped<'_>]) -> String {
    let results: Vec<String> =
        order::violations(items).into_iter().map(result).collect();
    object(&[
        field("tool", &object(&[field("driver", &driver())])),
        field("columnKind", &string("unicodeCodePoints")),
        field("results", &array(&results)),
        field("invocations", &array(&[invocation(skipped)])),
    ])
}

/// The tool, from Cargo's own metadata rather than restated here, so a
/// version bump cannot leave the log claiming the old one.
fn driver() -> String {
    let mut fields = vec![
        field("name", &string(NAME)),
        field("version", &string(env!("CARGO_PKG_VERSION"))),
    ];
    if let Some(uri) = information_uri(env!("CARGO_PKG_REPOSITORY")) {
        fields.push(field("informationUri", &string(uri)));
    }
    let rules: Vec<String> = LINTS.iter().copied().map(rule).collect();
    fields.push(field("rules", &array(&rules)));
    object(&fields)
}

/// Cargo hands an absent `repository` over as the empty string. That is
/// OMITTED rather than written: an empty URI is not a link, and inventing
/// one would be worse.
fn information_uri(repository: &str) -> Option<&str> {
    (!repository.is_empty()).then_some(repository)
}

/// One registered lint. The group travels as a TAG, which is how code
/// scanning filters, so `hazard` findings can be told apart at a glance.
fn rule(lint: Lint) -> String {
    let level = field("level", &string(level_word(lint.default_level())));
    let tags = field("tags", &array(&[string(lint.group.name())]));
    object(&[
        field("id", &string(lint.name)),
        field("defaultConfiguration", &object(&[level])),
        field("properties", &object(&[tags])),
    ])
}

/// SARIF's level words. `forbid` and `deny` both fail a run, so both are
/// `error`; `allow` is `none`, which only a rule's default can carry,
/// because an allowed finding is never reported.
const fn level_word(level: Level) -> &'static str {
    match level {
        Level::Allow => "none",
        Level::Warn => "warning",
        Level::Deny | Level::Forbid => "error",
    }
}

fn result(item: &Violation<'_>) -> String {
    let at = region(item.finding.hit.position);
    object(&[
        field("ruleId", &string(item.finding.lint.name)),
        field("level", &string(level_word(item.finding.level))),
        field("message", &text(&message(item))),
        field("locations", &array(&[location(item.path, Some(at))])),
    ])
}

/// The code point and the set the file was judged against: the two facts
/// the human line carries besides the position. Phrased so it stays true
/// for a lint that fires INSIDE the set, as a hazard does under `any`.
fn message(item: &Violation<'_>) -> String {
    let found = codepoint(item.finding.hit.character);
    format!("{found} (set in force: {})", item.set)
}

fn text(words: &str) -> String {
    object(&[field("text", &string(words))])
}

/// One character, exactly. `endColumn` is EXCLUSIVE and defaults to the
/// end of the line when absent, so it is written: a region that silently
/// covered the rest of the line would underline text that did nothing.
fn region(at: Position) -> String {
    object(&[
        field("startLine", &number(at.line)),
        field("startColumn", &number(at.column)),
        field("endColumn", &number(at.column.saturating_add(1))),
    ])
}

fn location(path: &str, region: Option<String>) -> String {
    let artifact = object(&[field("uri", &string(&uri(path)))]);
    let mut fields = vec![field("artifactLocation", &artifact)];
    if let Some(at) = region {
        fields.push(field("region", &at));
    }
    object(&[field("physicalLocation", &object(&fields))])
}

/// A path as a URI reference (RFC 3986). Every byte outside the unreserved
/// set and `/` is percent-encoded, so a space, a `%`, a `:` that would read
/// as a scheme, or a non-ASCII name all survive.
///
/// A relative path stays relative, to the directory `ctrm` ran in, which
/// code scanning resolves against the checkout. An absolute one -- a file
/// named outside that directory -- becomes a `file://` URI rather than a
/// relative reference that would resolve somewhere it is not.
fn uri(path: &str) -> String {
    let encoded: String = path.bytes().map(uri_byte).collect();
    if path.starts_with('/') {
        return format!("file://{encoded}");
    }
    encoded
}

fn uri_byte(byte: u8) -> String {
    if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
        return String::from(char::from(byte));
    }
    format!("%{byte:02X}")
}

/// The run itself. It SUCCEEDED whatever it found: a violation is a
/// result, and a run that could not start emits no log at all.
fn invocation(skipped: &[Skipped<'_>]) -> String {
    let notes: Vec<String> = order::skipped(skipped)
        .into_iter()
        .map(notification)
        .collect();
    object(&[
        field("executionSuccessful", "true"),
        field("toolExecutionNotifications", &array(&notes)),
    ])
}

/// An unread file is NAMED (`src/scan:V8`), never dropped: a log that
/// left it out would read exactly like a clean file. Invalid UTF-8 is a
/// `warning`, because text went unchecked; a binary skip is a `note`,
/// because that is the scanner working as intended.
fn notification(item: &Skipped<'_>) -> String {
    let (level, words, at) = match item.reason {
        Unreadable::NotUtf8 { byte } => (
            "warning",
            format!("not read: invalid UTF-8 at byte {byte}"),
            Some(object(&[field("byteOffset", &number(byte))])),
        ),
        Unreadable::Binary => ("note", String::from("not read: binary"), None),
    };
    object(&[
        field("level", &string(level)),
        field("message", &text(&words)),
        field("locations", &array(&[location(item.path, at)])),
    ])
}

#[cfg(test)]
mod tests {
    use super::{check, information_uri, level_word, uri};
    use crate::lint::{Finding, Group, LINTS, Level, Lint};
    use crate::render::{Skipped, Violation};
    use crate::scan::{Hit, Position, Unreadable};

    /// The registered lint, so the result's `ruleId` names a row of the
    /// `rules` array the way a real run's does.
    fn outside_set() -> Lint {
        Lint::named("outside-set").unwrap_or_else(|| {
            unreachable!("the registry always carries this lint")
        })
    }

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

    fn em_dash(path: &str, level: Level) -> Violation<'_> {
        let (hit, lint) = (hit(), outside_set());
        let finding = Finding { hit, lint, level };
        Violation {
            path,
            finding,
            set: "ascii",
        }
    }

    fn skip(path: &str, reason: Unreadable) -> Skipped<'_> {
        Skipped { path, reason }
    }

    /// The fixed head of every log: the schema, the version, and the
    /// driver with one rule per REGISTERED lint, in registry order.
    ///
    /// Built from the registry rather than spelled out, so registering a
    /// lint (a hazard lint, say) does not break a test about the shape.
    /// The level words are this test's own table, keyed on the group, so
    /// the mapping is still checked rather than recomputed.
    fn head() -> String {
        let rules: Vec<String> = LINTS.iter().map(|l| rule_row(*l)).collect();
        format!(
            concat!(
                r#"{{"$schema":"https://json.schemastore.org/"#,
                r#"sarif-2.1.0.json","version":"2.1.0","runs":[{{"tool":"#,
                r#"{{"driver":{{"name":"ctrm","version":"{}","#,
                r#""informationUri":"{}","rules":[{}]}}}},"#,
                r#""columnKind":"unicodeCodePoints","#,
            ),
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_REPOSITORY"),
            rules.join(",")
        )
    }

    fn rule_row(lint: Lint) -> String {
        let (level, tag) = match lint.group {
            Group::Hazard => ("error", "hazard"),
            Group::Charset => ("error", "charset"),
            Group::Pedantic => ("none", "pedantic"),
        };
        format!(
            concat!(
                r#"{{"id":"{}","defaultConfiguration":{{"level":"{}"}},"#,
                r#""properties":{{"tags":["{}"]}}}}"#,
            ),
            lint.name, level, tag
        )
    }

    /// The one row a reader of this file can check by eye, pinned whole:
    /// the tool's ordinary violation. Pinned by CONTENT, not position: the
    /// registry lists the loudest group first, so the hazard lints precede
    /// it, and a test that assumed it led the list broke when they landed.
    #[test]
    fn the_ordinary_violation_is_a_rule() {
        let row = concat!(
            r#"{"id":"outside-set","#,
            r#""defaultConfiguration":{"level":"error"},"#,
            r#""properties":{"tags":["charset"]}}"#
        );
        assert!(head().contains(row), "{}", head());
        assert!(check(&[], &[]).starts_with(&head()));
    }

    #[test]
    fn the_log_is_the_contract() {
        let expected = concat!(
            r#""results":[{"ruleId":"outside-set","level":"error","#,
            r#""message":{"text":"U+2014 (set in force: ascii)"},"#,
            r#""locations":[{"physicalLocation":{"artifactLocation":{"#,
            r#""uri":"src/a.rs"},"region":{"startLine":2,"startColumn":5,"#,
            r#""endColumn":6}}}]}],"invocations":[{"#,
            r#""executionSuccessful":true,"#,
            r#""toolExecutionNotifications":[]}]}]}"#
        );
        let log = check(&[em_dash("src/a.rs", Level::Deny)], &[]);
        assert_eq!(log, format!("{}{expected}", head()));
    }

    #[test]
    fn unread_files_are_notifications_in_path_order() {
        let items = [
            skip("b.bin", Unreadable::Binary),
            skip("a.txt", Unreadable::NotUtf8 { byte: 17 }),
        ];
        let expected = concat!(
            r#""results":[],"invocations":[{"executionSuccessful":true,"#,
            r#""toolExecutionNotifications":[{"level":"warning","#,
            r#""message":{"text":"not read: invalid UTF-8 at byte 17"},"#,
            r#""locations":[{"physicalLocation":{"artifactLocation":{"#,
            r#""uri":"a.txt"},"region":{"byteOffset":17}}}]},"#,
            r#"{"level":"note","message":{"text":"not read: binary"},"#,
            r#""locations":[{"physicalLocation":{"artifactLocation":{"#,
            r#""uri":"b.bin"}}}]}]}]}]}"#
        );
        assert_eq!(check(&[], &items), format!("{}{expected}", head()));
    }

    #[test]
    fn a_warned_finding_is_a_warning_and_results_keep_report_order() {
        let items =
            [em_dash("b.md", Level::Warn), em_dash("a.md", Level::Forbid)];
        let log = check(&items, &[]);
        let first = log.find(r#""uri":"a.md""#).unwrap_or(usize::MAX);
        let second = log.find(r#""uri":"b.md""#).unwrap_or(0);
        assert!(first < second, "{log}");
        assert!(log.contains(r#""level":"warning","message""#), "{log}");
    }

    #[test]
    fn levels_map_onto_the_sarif_words() {
        assert_eq!(level_word(Level::Forbid), "error");
        assert_eq!(level_word(Level::Deny), "error");
        assert_eq!(level_word(Level::Warn), "warning");
        assert_eq!(level_word(Level::Allow), "none");
    }

    #[test]
    fn a_path_is_percent_encoded_into_a_uri_reference() {
        assert_eq!(uri("src/a_b-c.d~e.rs"), "src/a_b-c.d~e.rs");
        assert_eq!(uri("my notes/100%.md"), "my%20notes/100%25.md");
        // A colon in the first segment would otherwise read as a scheme.
        assert_eq!(uri("c:d.md"), "c%3Ad.md");
        // U+00E9 LATIN SMALL LETTER E WITH ACUTE, as its two UTF-8 bytes.
        assert_eq!(uri("caf\u{e9}.md"), "caf%C3%A9.md");
    }

    #[test]
    fn an_absolute_path_becomes_a_file_uri() {
        assert_eq!(uri("/elsewhere/a.rs"), "file:///elsewhere/a.rs");
    }

    #[test]
    fn an_absent_repository_is_omitted_rather_than_empty() {
        assert_eq!(information_uri(""), None);
        assert_eq!(
            information_uri("https://x.test/r"),
            Some("https://x.test/r")
        );
    }
}
