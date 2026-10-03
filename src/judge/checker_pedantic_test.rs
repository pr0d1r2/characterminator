//! The pedantic lints: silent until asked, one claim per character, and
//! the fixtures where each one is right to be allowed (V37, V58). A
//! child of the `checker` tests, for their helpers.

use super::super::{Judge, Looked, inspect};
use super::{any, findings, hazards, summary};
use crate::charset::{CharSet, builtin};
use crate::lint::{Group, Level, Levels, Target};

/// One finding as `(lint name, level, byte)`.
type Fired = (&'static str, Level, usize);

/// What `check` finds in `text` under `set`, with pedantic at `warn`
/// and the charset findings at `charset`, sorted by byte.
fn pedantic(text: &str, set: &CharSet, charset: Level) -> Vec<Fired> {
    let mut levels = Levels::new();
    levels.set(Target::Group(Group::Pedantic), Level::Warn);
    levels.set_charset(charset);
    let hazards = hazards();
    let judge = Judge::new(set, &hazards);
    let mut all = match inspect(text.as_bytes(), &judge, &levels) {
        Looked::Findings(all) => all.iter().map(summary).collect(),
        Looked::Unread(_) => Vec::new(),
    };
    all.sort_by_key(|(_, _, byte)| *byte);
    all
}

/// V37: the group is `allow`, so a text with every pedantic shape in
/// it says nothing until a run asks.
#[test]
fn pedantic_lints_are_silent_until_asked() {
    assert_eq!(findings("a \r\nb\u{a0}c", &any()), vec![]);
}

/// Asked, each of the four fires at the character it points at.
#[test]
fn asked_for_every_pedantic_shape_fires_once() {
    let fired = pedantic("a \r\nb\u{a0}c", &any(), Level::Deny);
    let warn = Level::Warn;
    let expected = vec![
        ("trailing-whitespace", warn, 1),
        ("crlf", warn, 2),
        ("unicode-space", warn, 5),
        ("final-newline", warn, 7),
    ];
    assert_eq!(fired, expected);
}

/// A no-break space the set does not grant is ONE finding: the set's,
/// which is the stronger claim. Allow that one, and pedantic speaks.
#[test]
fn a_space_outside_the_set_is_reported_once() {
    let ascii = builtin::ascii();
    let denied = pedantic("a\u{a0}b\n", &ascii, Level::Deny);
    assert_eq!(denied, vec![("outside-set", Level::Deny, 1)]);
    let allowed = pedantic("a\u{a0}b\n", &ascii, Level::Allow);
    assert_eq!(allowed, vec![("unicode-space", Level::Warn, 1)]);
}
/// Under plain `ascii` the CR is not granted, so a CR LF is already
/// `outside-set`; asking for pedantic does not say it twice.
#[test]
fn a_cr_the_set_refuses_is_one_finding() {
    let fired = pedantic("hi\r\n", &builtin::ascii(), Level::Deny);
    assert_eq!(fired, vec![("outside-set", Level::Deny, 2)]);
    let quiet = pedantic("hi\r\n", &builtin::ascii(), Level::Allow);
    assert_eq!(quiet, vec![("crlf", Level::Warn, 2)]);
}

/// Asked for nothing, none of the four fires, and a character the
/// set refuses is one `outside-set` finding however many lints it
/// would also fire. The `e` the accent decomposes from is in the set, so
/// `not-nfc` still names it.
#[test]
fn the_unicode_lints_are_silent_until_asked_and_claim_once() {
    let text = "cafe\u{301} p\u{430}y \u{ff21}\n";
    assert_eq!(findings(text, &any()), vec![]);
    let ascii = builtin::ascii();
    let fired = pedantic(text, &ascii, Level::Deny);
    let deny = Level::Deny;
    let expected = vec![
        ("not-nfc", Level::Warn, 3),
        ("outside-set", deny, 4),
        ("outside-set", deny, 8),
        ("outside-set", deny, 12),
    ];
    assert_eq!(fired, expected);
}
