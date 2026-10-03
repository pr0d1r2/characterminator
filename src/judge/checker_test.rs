//! The tests of `checker.rs`, in a file of their own so the module
//! reads as code (sherd V50). Still its child: `super` is `checker`.
//!
//! This file holds the shared helpers and the core: what a file is
//! judged against, and where a finding sits. Two subjects big enough to
//! be read alone are child modules of this one, so they reach these
//! helpers as `super` and nothing is widened for them: the hazards and
//! their exemptions, and the pedantic lints' fixtures (V37, V58).

use super::{Checker, Config, Judge, Looked, OUTSIDE, inspect, levels_for};
use crate::charset::{CharSet, builtin};
use crate::fix as engine;
use crate::lint::{Finding, Hazards, Level, Levels, Lint};
use crate::rules::{self, Rule, Sources};
use std::rc::Rc;

#[path = "checker_hazard_test.rs"]
mod hazard;
#[path = "checker_pedantic_test.rs"]
mod pedantic;

fn lint() -> Lint {
    match Lint::named(OUTSIDE) {
        Some(found) => found,
        None => Lint::named("outside-set").unwrap_or_else(|| {
            unreachable!("the registry always carries this lint")
        }),
    }
}

fn hazards() -> Hazards {
    Hazards::builtin()
}

/// What `check` finds in `bytes` when the file is granted `set`.
fn found_in(bytes: &[u8], set: &CharSet) -> Looked {
    let hazards = hazards();
    let judge = Judge::new(set, &hazards, lint());
    inspect(bytes, &judge, &Levels::new())
}

fn found(bytes: &[u8]) -> Looked {
    found_in(bytes, &builtin::ascii())
}

/// The findings, as `(lint name, level, byte)`, for a text that must
/// be readable.
fn findings(text: &str, set: &CharSet) -> Vec<(&'static str, Level, usize)> {
    match found_in(text.as_bytes(), set) {
        Looked::Findings(all) => all.iter().map(summary).collect(),
        Looked::Unread(_) => unreachable!("this text is readable"),
    }
}

fn summary(finding: &Finding) -> (&'static str, Level, usize) {
    let at = finding.hit.position.byte;
    (finding.lint.name, finding.level, at)
}

/// `any`, as the builtin catalog resolves it.
fn any() -> CharSet {
    builtin::catalog()
        .ok()
        .and_then(|sets| sets.resolve("any", "text").ok())
        .unwrap_or_else(|| unreachable!("`any` ships as a preset"))
}

#[test]
fn ascii_text_reports_nothing() {
    match found(b"plain ascii, nothing to say\n") {
        Looked::Findings(findings) => assert!(findings.is_empty()),
        Looked::Unread(_) => unreachable!("this text is readable"),
    }
}

#[test]
fn a_character_outside_the_set_is_found_where_it_sits() {
    // An em dash on the second line, after four characters.
    let text = "ok\nab c\u{2014}d\n";
    match found(text.as_bytes()) {
        Looked::Findings(findings) => {
            assert_eq!(findings.len(), 1);
            let first = findings.first().cloned();
            let one = first.unwrap_or_else(|| unreachable!("one finding"));
            assert_eq!(one.hit.character, '\u{2014}');
            assert_eq!(one.hit.position.line, 2);
            assert_eq!(one.hit.position.column, 5);
            // The level lives on the FINDING, not on the hit: the
            // scanner reports what is where, the lint node says how
            // loudly, and `charset` denies by default.
            assert_eq!(one.level, Level::Deny);
        }
        Looked::Unread(_) => unreachable!("this text is readable"),
    }
}

#[test]
fn a_binary_file_is_skipped_rather_than_reported() {
    match found(b"text\0more") {
        Looked::Unread(_) => (),
        Looked::Findings(_) => unreachable!("a NUL means binary"),
    }
}

#[test]
fn an_unmatched_path_is_governed_by_ascii() {
    // V1: no rule means `ascii`, so the strict default needs no file.
    let resolution = rules::resolve("a.md", &[] as &[Rule], &rules::matches);
    assert_eq!(resolution.sets, vec![String::from("ascii")]);
}

#[test]
fn a_level_directive_naming_nothing_is_an_error() {
    // A misspelled lint that silently did nothing would read exactly
    // like a rule that worked.
    let rules = match rules::parse_rule(
        "*.md !nosuchlint=warn",
        rules::Origin::Builtin { line: 1 },
    ) {
        Ok(rule) => vec![rule],
        Err(_) => unreachable!("the line parses; the TARGET is unknown"),
    };
    let resolution = rules::resolve("a.md", &rules, &rules::matches);
    assert!(levels_for(&resolution).is_err());
}

/// A run configured by these dotfiles alone, over the builtin sets and
/// map: what discovery (`src/cli`) builds from a tree holding them, with
/// no tree. Judging reads sources, never the disk.
fn configured(files: &[(&str, &str)]) -> Config {
    let source = |name: &str, under: Sources| match files
        .iter()
        .find(|(file, _)| *file == name)
    {
        Some((_, text)) => under.dotfile(name, *text),
        None => under,
    };
    Config {
        rules: source(".ctrm", Sources::new()),
        sets: source(".ctrm-sets", Sources::new().builtin(builtin::SETS)),
        map: source(".ctrm-map", Sources::new().builtin(engine::BUILTIN)),
        ..Config::default()
    }
}

/// The set one path is judged against, under `config`.
fn granted(config: &Config, shown: &str) -> Result<CharSet, String> {
    Checker::configured(config)?.law(shown).map(|(set, _)| set)
}

/// A rule may name a preset that ships as DATA, not only the
/// intrinsic set: `src/charset:V22` compiles the preset file in, and
/// this is the path that makes it reachable from a `.ctrm` line.
#[test]
fn a_rule_may_grant_a_preset_that_ships_as_data() {
    let preset = configured(&[(".ctrm", "*.md ascii+caveman\n")]);
    let found = granted(&preset, "notes.md");
    let why = found.as_ref().err().cloned().unwrap_or_default();
    let set = found.unwrap_or_else(|_| unreachable!("{why}"));
    assert_eq!(set.name, "ascii+caveman");
    // RIGHTWARDS ARROW, granted by the preset, and EM DASH, which is
    // the map's business rather than a grant (`src/fix:V26`).
    assert!(set.contains('\u{2192}'));
    assert!(!set.contains('\u{2014}'));
}

/// `.ctrm-sets` is DISCOVERED beside `.ctrm` (`src/rules:V45`), so a
/// repo whose files hold characters no preset covers declares a set
/// rather than reaching for `any`.
#[test]
fn a_declared_set_is_discovered_beside_the_rules() {
    // IDENTICAL TO, which no builtin preset grants.
    let files = [
        (".ctrm", "*.md ascii+house\n"),
        (".ctrm-sets", "house U+2261\n"),
    ];
    let config = configured(&files);
    let found = granted(&config, "notes.md");
    let why = found.as_ref().err().cloned().unwrap_or_default();
    let set = found.unwrap_or_else(|_| unreachable!("{why}"));
    assert!(set.contains('\u{2261}'));
}

/// A declared set REPLACES the builtin of the same name (V19), which
/// is what "later wins" means for a set, and is how a repo narrows a
/// preset it finds too generous.
#[test]
fn a_declared_set_replaces_the_builtin_it_renames() {
    let files = [
        (".ctrm", "*.md ascii+legal\n"),
        // The builtin `legal` also grants REGISTERED and TRADE MARK.
        (".ctrm-sets", "legal U+00A9\n"),
    ];
    let config = configured(&files);
    let set = granted(&config, "notes.md").unwrap_or_else(|why| {
        unreachable!("{why}");
    });
    assert!(set.contains('\u{00A9}'));
    assert!(!set.contains('\u{00AE}'));
}

/// A `.ctrm-sets` line that cannot be read names the FILE and LINE,
/// not just the complaint: the origin travels with the error
/// (`src/rules:V20`).
#[test]
fn a_bad_declared_line_is_reported_at_its_origin() {
    let files = [(".ctrm-sets", "\n\nbad U+ZZZZ\n")];
    let config = configured(&files);
    let why = Checker::configured(&config).err().unwrap_or_default();
    assert!(why.contains(".ctrm-sets:3"), "{why}");
    assert!(why.contains("U+ZZZZ"), "{why}");
}
/// The whole of V41 through the real path: two rules name the SAME
/// preset, and the fidelity each one chose decides what it grants.
#[test]
fn one_preset_grants_differently_under_two_fidelities() {
    let rules = "*.md ascii+marks\ndocs/*.md ascii+marks @emoji\n";
    let config = configured(&[(".ctrm", rules)]);
    let plain = granted(&config, "notes.md").unwrap_or_else(|why| {
        unreachable!("{why}");
    });
    let rich = granted(&config, "docs/notes.md").unwrap_or_else(|why| {
        unreachable!("{why}");
    });
    // CHECK MARK at `text`, WHITE HEAVY CHECK MARK at `emoji`.
    assert!(plain.contains('\u{2713}') && !plain.contains('\u{2705}'));
    assert!(rich.contains('\u{2705}') && !rich.contains('\u{2713}'));
    // WARNING SIGN carries no label, so both spellings hold it.
    assert!(plain.contains('\u{26A0}') && rich.contains('\u{26A0}'));
}

/// `src/render:R17`: the lints one hit could fire, strongest first, are the
/// same whether the pedantic tail is asked eagerly or, as now, only when
/// nothing before it spoke. FULLWIDTH LATIN CAPITAL A is outside `ascii`, NFKC
/// folds it, and UTS #39 draws it as `A`.
#[test]
fn every_lint_a_hit_could_fire_is_listed_strongest_first() {
    let (set, hazards) = (builtin::ascii(), hazards());
    let judge = Judge::new(&set, &hazards, lint());
    let position = crate::scan::Position {
        line: 1,
        column: 1,
        byte: 0,
    };
    let character = '\u{FF21}';
    let hit = crate::scan::Hit {
        position,
        character,
    };
    let names: Vec<&str> = judge.lints_for(hit).map(|l| l.name).collect();
    assert_eq!(names, ["outside-set", "nfkc-compat", "confusable"]);
}

/// `src/render:R17`: one union per sets-and-family, kept for every path that
/// asks for it, and the kept answer is the one a fresh checker resolves.
#[test]
fn a_union_is_resolved_once_and_shared_by_every_path_it_governs() {
    let ctrm = "*.md ascii+caveman\n*.txt ascii\n";
    let config = configured(&[(".ctrm", ctrm)]);
    let Some([a, b, txt]) = kept(&config, ["a.md", "sub/b.md", "c.txt"]) else {
        unreachable!("every path here resolves");
    };
    assert!(Rc::ptr_eq(&a, &b));
    assert!(!Rc::ptr_eq(&a, &txt));
    assert_eq!(Ok(CharSet::clone(&a)), granted(&config, "a.md"));
    assert_eq!(
        (a.name.as_str(), txt.name.as_str()),
        ("ascii+caveman", "ascii")
    );
}

/// The sets ONE checker answers for each path, in order.
fn kept(config: &Config, paths: [&str; 3]) -> Option<[Rc<CharSet>; 3]> {
    let checker = Checker::configured(config).ok()?;
    let set = |shown: &str| checker.shared_law(shown).ok().map(|(set, _)| set);
    let [one, two, three] = paths.map(set);
    Some([one?, two?, three?])
}
