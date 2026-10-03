//! The tests of `map.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `map`.

use super::Map;
use crate::fix::Error;
use crate::rules::Origin;

fn parse(source: &str) -> Result<Map, Error> {
    Map::parse(source, &|line| Origin::Builtin { line })
}

fn parsed(source: &str) -> Map {
    parse(source).unwrap_or_default()
}

#[test]
fn reads_a_line_in_either_form() {
    let map = parsed("U+2014 --\n\u{2013} -\n");
    assert!(parse("U+2014 --\n\u{2013} -\n").is_ok());
    assert_eq!(map.entries().len(), 2);
}

/// B68 / `src/judge:V116`: a source every set grants is never a
/// violation, so a line mapping it did nothing and was accepted.
#[test]
fn a_source_made_only_of_ascii_is_refused() {
    let never = Err(Error::NeverRewritten { line: 2 });
    assert_eq!(parse("U+2014 -\nfoo bar\n"), never);
    assert_eq!(parse("U+2014 -\nword U+0041 b\n"), never);
    assert!(parse("U+000D\nfoo\u{2014} x\n").is_ok());
}

#[test]
fn an_absent_replacement_is_an_explicit_delete() {
    let map = parsed("U+200B\n");
    assert_eq!(map.entries().first().map(|e| e.to.as_str()), Some(""));
}

#[test]
fn a_replacement_of_one_space_is_written_in_code_point_form() {
    let map = parsed("U+00A0 U+0020\n");
    assert_eq!(map.entries().first().map(|e| e.to.as_str()), Some(" "));
}

#[test]
fn skips_blank_lines_and_comments() {
    assert_eq!(parsed("\n# a note\n   \nU+2014 --\n").entries().len(), 1);
}

#[test]
fn carries_the_origin_of_the_line_it_came_from() {
    let map = parsed("# a note\nU+2014 --\n");
    let origin = map.entries().first().map(|e| e.origin.clone());
    assert_eq!(origin, Some(Origin::Builtin { line: 2 }));
}

#[test]
fn a_later_line_wins_for_the_same_source() {
    let map = parsed("U+2014 --\nU+2014 -\n");
    assert_eq!(map.entries().len(), 1);
    assert_eq!(map.entries().first().map(|e| e.to.as_str()), Some("-"));
}

/// The winner sits where it was LAST declared, within a layer and across
/// layers -- the order the per-line removal used to leave, and the one
/// `explain` lists entries in.
#[test]
fn a_redeclared_source_moves_to_its_last_declaration() {
    let first = parsed("U+2014 a\nU+2013 b\nU+2014 c\nU+2012 d\n");
    let layered = first.layer("U+2013 e\n", &|line| Origin::Builtin { line });
    let held = layered.unwrap_or_default();
    let pairs: Vec<(&str, &str)> = held
        .entries()
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();
    let want = [("\u{2014}", "c"), ("\u{2012}", "d"), ("\u{2013}", "e")];
    assert_eq!(pairs, want);
}

#[test]
fn holds_the_longest_source_first() {
    let map = parsed("U+1F44D +1\nU+1F44D+U+1F3FD U+1F44D\n");
    let first = map.entries().first().map(|e| e.from.chars().count());
    assert_eq!(first, Some(2));
}

#[test]
fn rejects_a_line_it_cannot_read() {
    assert_eq!(parse("U+2014 -- --\n"), Err(Error::Syntax { line: 1 }));
    assert_eq!(parse("\nU+ZZZZ -\n"), Err(Error::Syntax { line: 2 }));
}

#[test]
fn a_family_line_declares_a_family() {
    let map = parsed("family nerd emoji\n");
    let want = Ok(vec!["nerd", "emoji", "text", "ascii"]);
    assert_eq!(map.tree().path("nerd"), want);
    assert_eq!(map.entries().len(), 0);
}

#[test]
fn a_family_line_with_an_unknown_parent_fails_to_parse() {
    let name = String::from("runic");
    let found = parse("family nerd runic\n");
    assert_eq!(found, Err(Error::UnknownFamily { name }));
}

#[test]
fn a_family_cycle_fails_to_parse() {
    let found = parse("family a b\nfamily b a\n");
    assert!(matches!(found, Err(Error::FamilyCycle { .. })));
}

#[test]
fn a_class_line_declares_a_class() {
    let map = parsed("= tick ascii:[x] emoji:U+2705\n");
    assert_eq!(map.classes().len(), 1);
    assert_eq!(map.entries().len(), 0);
}

#[test]
fn a_later_class_line_wins_for_the_same_name() {
    let map = parsed("= tick ascii:[x]\n= tick ascii:(x)\n");
    assert_eq!(map.classes().len(), 1);
    let first = map.classes().first().map(|held| held.members.len());
    assert_eq!(first, Some(1));
}

#[test]
fn a_member_in_an_unknown_family_fails_to_parse() {
    let name = String::from("runic");
    let found = parse("= tick runic:x\n");
    assert_eq!(found, Err(Error::UnknownFamily { name }));
}

#[test]
fn the_fidelity_family_defaults_to_the_root_and_must_be_known() {
    let map = parsed("= tick ascii:[x] emoji:U+2705\n");
    assert_eq!(map.fidelity(), "ascii");
    assert!(map.clone().with_fidelity("emoji").is_ok());
    assert!(map.with_fidelity("runic").is_err());
}

#[test]
fn rejects_a_class_line_it_cannot_read() {
    let line = Err(Error::Syntax { line: 1 });
    assert_eq!(parse("= tick\n"), line);
    assert_eq!(parse("= tick emoji\n"), line);
}

#[test]
fn rejects_a_family_line_it_cannot_read() {
    let line = Err(Error::Syntax { line: 1 });
    assert_eq!(parse("family nerd\n"), line);
    assert_eq!(parse("family a b c\n"), line);
    assert_eq!(parse("family we:ird emoji\n"), line);
}
/// The builtin map ships as DATA in this grammar (V26,
/// `src/charset:V22`), so it has to parse -- and a defect here is a
/// defect in this crate, not in anyone's configuration.
#[test]
fn the_builtin_map_parses() {
    assert!(parse(super::BUILTIN).is_ok());
    assert!(!super::Map::builtin().entries().is_empty());
}

#[test]
fn the_builtin_map_is_pure_ascii() {
    assert!(super::BUILTIN.is_ascii());
}

/// V26 names what it targets. Written out rather than read back from
/// the map: a test that asked the file what it declares would pass
/// just as happily after an entry was deleted from it.
fn rewrites(map: &Map, from: char, to: &str) {
    let found = map
        .entries()
        .iter()
        .find(|entry| entry.from == from.to_string());
    assert_eq!(found.map(|e| e.to.as_str()), Some(to), "{from:?}");
}

/// `src/fix/emoji:V60`: the presentation selectors and skin tones are deleted,
/// so an emoji with a modifier compresses to its base without a sequence scan.
#[test]
fn the_builtin_map_deletes_the_emoji_modifiers() {
    let map = parsed(super::BUILTIN);
    for from in ['\u{FE0E}', '\u{FE0F}', '\u{1F3FB}', '\u{1F3FF}'] {
        rewrites(&map, from, "");
    }
}

#[test]
fn the_builtin_map_targets_what_v26_names() {
    let map = parsed(super::BUILTIN);
    let rewrites = |from, to| rewrites(&map, from, to);
    rewrites('\u{2014}', "--");
    rewrites('\u{2013}', "-");
    rewrites('\u{2212}', "-");
    rewrites('\u{2018}', "'");
    rewrites('\u{2019}', "'");
    rewrites('\u{201C}', "\"");
    rewrites('\u{201D}', "\"");
    rewrites('\u{00AB}', "\"");
    rewrites('\u{00BB}', "\"");
    rewrites('\u{2026}', "...");
    rewrites('\u{00A0}', " ");
}

/// The two that leave rather than change: they carry no glyph, so any
/// visible replacement would put a character on the page the author
/// never typed (V4 makes the delete explicit).
#[test]
fn the_builtin_map_deletes_the_invisibles() {
    let map = parsed(super::BUILTIN);
    for gone in ['\u{200B}', '\u{FEFF}'] {
        let found = map
            .entries()
            .iter()
            .find(|entry| entry.from == gone.to_string());
        assert_eq!(found.map(|e| e.to.as_str()), Some(""), "{gone:?}");
    }
}

/// `src/lint/hazard:V34`: the builtin MAP has no entry for a bidi control.
/// `fix` still deletes one, but as a hazard (V104), not as a mapping:
/// a `.ctrm-map` line for it wins, and either way the deletion is a
/// reported rewrite row -- the record that a Trojan Source override was
/// there.
#[test]
fn the_builtin_map_leaves_every_bidi_control_alone() {
    let map = parsed(super::BUILTIN);
    let bidi = ['\u{061C}', '\u{200E}', '\u{200F}']
        .into_iter()
        .chain('\u{202A}'..='\u{202E}')
        .chain('\u{2066}'..='\u{2069}');
    for kept in bidi {
        let from = kept.to_string();
        let mapped = map.entries().iter().any(|entry| entry.from == from);
        assert!(!mapped, "U+{:04X} is mapped", u32::from(kept));
    }
}

/// Every replacement the builtin offers is itself ASCII. A map whose
/// answer was another non-ASCII character would move the problem
/// rather than solve it, and `fix` would report the result as a
/// violation of the same rule (V4).
///
/// The ONE exception is `src/fix/emoji:V62`'s: a ZWJ sequence compresses to a
/// single emoji, which has no ASCII form. It is still ONE code point the
/// `emoji` preset grants, so a file under `emoji` is settled by it, and a file
/// under `ascii` is left one finding instead of several.
#[test]
fn every_builtin_replacement_is_ascii_or_one_emoji() {
    let emoji = crate::charset::builtin::catalog()
        .ok()
        .and_then(|c| c.resolve("emoji", "text").ok());
    for entry in parsed(super::BUILTIN).entries() {
        let one = entry.to.chars().count() == 1
            && entry
                .to
                .chars()
                .all(|c| emoji.as_ref().is_some_and(|e| e.contains(c)));
        let ok = entry.to.is_ascii() || one;
        assert!(ok, "{} -> {}", entry.from, entry.to);
    }
}

fn found(map: &Map, from: char) -> Option<super::MapEntry> {
    let from = from.to_string();
    map.entries()
        .iter()
        .find(|entry| entry.from == from)
        .cloned()
}

#[test]
fn a_word_line_declares_a_word_and_a_plain_line_does_not() {
    let map = parsed("word U+22A5 not\nU+2014 --\n");
    assert_eq!(found(&map, '\u{22A5}').map(|e| e.word), Some(true));
    assert_eq!(found(&map, '\u{2014}').map(|e| e.word), Some(false));
}

/// A word that deletes is a delete, which the plain line already says;
/// a `word` line without a word is a typo, not a second spelling.
#[test]
fn a_word_line_needs_a_word() {
    assert_eq!(parse("word U+22A5\n"), Err(Error::Syntax { line: 1 }));
    assert_eq!(parse("word a b c\n"), Err(Error::Syntax { line: 1 }));
}

/// `src/fix/words:V51`: `use words` reads the named map at that line, and each
/// entry it brings answers "why" with the line that opted in (V20).
#[test]
fn a_use_line_pulls_in_a_named_map_under_its_own_origin() {
    let map = parsed("# opt in\nuse words\n");
    let up_tack = found(&map, '\u{22A5}');
    assert_eq!(up_tack.clone().map(|e| e.to), Some(String::from("not")));
    let origin = up_tack.map(|e| e.origin);
    assert_eq!(origin, Some(Origin::Builtin { line: 2 }));
}

#[test]
fn a_use_line_naming_no_builtin_map_is_refused() {
    let name = String::from("klingon");
    assert_eq!(parse("use klingon\n"), Err(Error::UnknownMap { name }));
}

/// Precedence is POSITIONAL (`src/rules:V19`): a line after `use`
/// overrides what it pulled in, and `use` overrides a line before it.
#[test]
fn a_use_line_sits_in_the_chain_where_it_was_written() {
    let after = parsed("use words\nword U+22A5 never\n");
    let before = parsed("word U+22A5 never\nuse words\n");
    let to = |map: &Map| found(map, '\u{22A5}').map(|e| e.to);
    assert_eq!(to(&after), Some(String::from("never")));
    assert_eq!(to(&before), Some(String::from("not")));
}

/// V18 for the two new line kinds: every line of a map, layered one
/// at a time the way a `--map` flag arrives, is the same map as the
/// file -- origins aside, which differ by design (V20).
#[test]
fn a_map_read_line_by_line_as_flags_is_the_same_map() {
    let text = "use words\nword U+2234 thus\nU+2014 --\n= t ascii:x\n";
    let flags = as_flags(text);
    assert_eq!(flags.clone().map(forget), parse(text).map(forget));
    let classes = flags.map(|map| map.classes().len());
    assert_eq!(classes, Ok(1));
}

/// Each line layered alone, labelled as the argv slot it would hold.
fn as_flags(text: &str) -> Result<Map, Error> {
    text.lines()
        .enumerate()
        .try_fold(Map::default(), |map, (index, line)| {
            map.layer(line, &|_| Origin::Argument { index })
        })
}

/// The entries with their origins dropped.
fn forget(map: Map) -> Vec<(String, String, bool)> {
    let entries = map.entries().iter();
    entries
        .map(|e| (e.from.clone(), e.to.clone(), e.word))
        .collect()
}

/// Every named map parses, is pure ASCII, offers only ASCII, and
/// pulls in no other map: a `use` inside one would be read again on
/// every `use` of it, and a map naming itself would never finish.
#[test]
fn every_named_map_is_well_formed_data() {
    for (name, text) in super::NAMED {
        assert!(text.is_ascii(), "{name}");
        let use_lines = text.lines().filter(|l| l.starts_with("use "));
        assert_eq!(use_lines.count(), 0, "{name}");
        let map = parse(text);
        assert!(map.is_ok(), "{name}");
        for entry in map.unwrap_or_default().entries() {
            assert!(entry.to.is_ascii(), "{name}: {}", entry.from);
        }
    }
}

/// `src/fix/words:V51` names what the `words` map targets. Written out, for the
/// reason `the_builtin_map_targets_what_v26_names` states.
#[test]
fn the_words_map_targets_what_v51_names() {
    let map = parsed(crate::fix::words::WORDS);
    let word = |from, to: &str| {
        let held = found(&map, from).map(|e| (e.to, e.word));
        assert_eq!(held, Some((to.to_owned(), true)), "{from:?}");
    };
    word('\u{22A5}', "not");
    word('\u{2234}', "so");
    word('\u{2235}', "because");
    word('\u{2200}', "all");
    word('\u{2208}', "in");
    word('\u{2203}', "exists");
    rewrites(&map, '\u{2260}', "!=");
    rewrites(&map, '\u{2192}', "->");
    rewrites(&map, '\u{21D2}', "=>");
    assert_eq!(map.entries().len(), 9);
}

/// V26 holds: the DEFAULT map rewrites none of the notation `words`
/// covers. Opting in is the only way those symbols are touched.
#[test]
fn the_builtin_map_leaves_the_notation_alone() {
    let builtin = parsed(super::BUILTIN);
    for entry in parsed(crate::fix::words::WORDS).entries() {
        let from = entry.from.chars().next().unwrap_or_default();
        assert_eq!(found(&builtin, from), None, "{from:?}");
    }
}
