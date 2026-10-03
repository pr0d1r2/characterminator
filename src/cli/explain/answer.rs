//! The `explain` and `sets` verbs (`src/cli:T12`): what is in force, and
//! why.
//!
//! Both are REPORT-ONLY and exit 0 whatever they find (`src/cli:V7`). They
//! answer questions about configuration rather than about files, so there
//! is nothing for them to fail at: a path with no rule is an answer, not a
//! violation.
//!
//! They exist because every other verb's verdict rests on a resolution
//! nobody can see. `check` says a character is outside `ascii+caveman`;
//! `explain` says which line of which file decided that, and `sets` says
//! what those names hold.

use super::export::{self, Shape};
use super::prompt;
use crate::charset::CharSet;
use crate::cli::checker::Checker;
use crate::cli::config::Config;
use crate::render::{Explanation, Format, InForce};
use crate::render::{explain as render_explain, sets as render_sets};
use crate::rules::{self, Resolution, Sourced};

/// `explain [<path>]`: the effective set for a path, and the rule behind
/// it.
///
/// With no path the question is asked of the whole repo, which resolves
/// the rules against nothing and reports the default in force. That is
/// `src/rules:V1`'s answer written down: a repo with no `.ctrm` grants
/// `ascii` everywhere, and a reader should be able to see that without
/// having to own a file to ask about.
///
/// # Errors
///
/// As [`Checker::configured`], plus a rule naming a set nothing declares.
pub(crate) fn run(
    config: &Config,
    paths: &[String],
    format: Format,
) -> Result<String, String> {
    let checker = Checker::configured(config)?;
    let asked = paths.first().map(String::as_str);
    let (set, winner) = checker.effective(asked.unwrap_or(""))?;
    let rules = config.rules()?;
    let found = rules::resolve(asked.unwrap_or(""), &rules, &rules::matches);
    Ok(render_explain(
        format,
        &Explanation {
            path: asked,
            set: &set,
            rule: winner.as_ref(),
            in_force: in_force(&found),
        },
    ))
}

/// The family and levels in force, each with the line that set it
/// (`src/rules:V20`): the winner of the grant need not be that line, and
/// reporting only the winner hid a family or a level that `check` applied
/// (B28).
fn in_force<'a>(found: &'a Resolution<'a>) -> InForce<'a> {
    let level = found.leveller.and_then(|rule| {
        let value = rule.default_level?;
        Some(Sourced {
            value,
            origin: &rule.origin,
        })
    });
    InForce {
        family: &found.family,
        family_origin: found.fidelity.map(|rule| &rule.origin),
        level,
        levels: found.sourced.clone(),
    }
}

/// `explain [<path>] --as args|lines|prompt` (`V32`): the same
/// configuration, rendered for something else to take in.
///
/// Every form is TEXT for a shell, a file or a model, so `--format` has
/// nothing to choose and asking for one is refused rather than ignored.
///
/// # Errors
///
/// An unknown form, a `--format` alongside it, or a configuration that
/// does not parse.
pub(crate) fn exported(
    config: &Config,
    paths: &[String],
    format: Format,
    form: &str,
) -> Result<String, String> {
    let shape = Shape::named(form)?;
    if format != Format::Human {
        return Err(String::from("--as writes text and takes no --format"));
    }
    let asked = paths.first().map(String::as_str);
    match shape {
        Shape::Args => export::args(config, asked),
        Shape::Lines => export::lines(config, asked),
        Shape::Prompt => prompt::render(config, asked),
    }
}

/// `sets`: every declared set and what it holds.
///
/// Resolved at the family the run's rules give the whole repo, the
/// question `explain` with no path asks (`src/rules:V29`): a preset with
/// labelled members (`src/charset:V41`) holds different characters at
/// another family, so `--fidelity emoji` and `--rule '* @emoji'`, which
/// are one rule, list the same thing (B30).
///
/// Every CLDR locale set is listed, not only the ones the rules name.
///
/// # Errors
///
/// As [`Checker::configured`], plus a declared set that cannot be resolved --
/// a cycle, or a member naming a set nothing declares.
pub(crate) fn sets(config: &Config, format: Format) -> Result<String, String> {
    let checker = Checker::listing(config)?;
    let rules = config.rules()?;
    let family = rules::resolve("", &rules, &rules::matches).family;
    let listed: Vec<CharSet> = checker.declared(&family)?;
    Ok(render_sets(format, &listed))
}

#[cfg(test)]
mod tests {
    use super::{Config, run, sets};
    use crate::render::Format;
    use std::path::{Path, PathBuf};

    /// A tree with the dotfiles named, under `target/` so it is untracked
    /// by construction. `None` if it cannot be written.
    fn fixture(name: &str, files: &[(&str, &str)]) -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(name);
        std::fs::create_dir_all(&root).ok()?;
        for (file, text) in files {
            std::fs::write(root.join(file), text).ok()?;
        }
        Some(root)
    }

    fn explained(root: &Path, path: &[String]) -> String {
        run(&Config::discovered(root), path, Format::Human)
            .unwrap_or_else(|why| why)
    }

    #[test]
    fn explain_names_the_rule_that_won_and_the_line_behind_it() {
        let rules = "*.md ascii\ndocs/*.md ascii+caveman\n";
        let files = [(".ctrm", rules)];
        let Some(root) = fixture("ctrm-explain-fixture", &files) else {
            return;
        };
        let said = explained(&root, &["docs/a.md".to_owned()]);
        assert!(said.contains("set ascii+caveman"), "{said}");
        assert!(said.contains("pattern docs/*.md"), "{said}");
        // V20: the origin is the file and LINE, so the config that
        // decided this is one grep away.
        assert!(said.contains(".ctrm:2"), "{said}");
    }

    /// V1 through the report: an unmatched path gets `ascii` and there is
    /// NO rule to name. Reporting a synthesised one would put an origin
    /// in the answer that no file could be opened at.
    #[test]
    fn explain_says_when_no_rule_matched() {
        let files = [(".ctrm", "docs/*.md ascii+caveman\n")];
        let Some(root) = fixture("ctrm-explain-bare-fixture", &files) else {
            return;
        };
        let said = explained(&root, &["src/main.rs".to_owned()]);
        assert!(said.contains("set ascii"), "{said}");
        assert!(said.contains("rule none"), "{said}");
    }

    /// B28: the family and the level `check` applies are named with the
    /// argument that set them, though neither line won the grant.
    #[test]
    fn explain_names_a_family_and_a_level_the_winner_did_not_set() {
        let Some(root) = fixture("ctrm-explain-in-force-fixture", &[]) else {
            return;
        };
        let flags = "--rule|*.md marks|--fidelity|emoji|--rule|*.md !warn";
        let said = run(&argued(&root, flags), &["m.md".to_owned()], HUMAN)
            .unwrap_or_else(|why| why);
        assert!(said.contains("effective family emoji argv[5]"), "{said}");
        assert!(said.contains("effective level warn argv[7]"), "{said}");
    }

    const HUMAN: Format = Format::Human;

    /// The configuration of `ctrm explain <flags>`, the flags written as
    /// one `|`-separated string so a test reads as its command line.
    fn argued(root: &Path, flags: &str) -> Config {
        let words = std::iter::once("explain").chain(flags.split('|'));
        let argv: Vec<String> = words.map(str::to_owned).collect();
        crate::cli::config::from_argv(root, &argv).unwrap_or_default()
    }

    #[test]
    fn sets_lists_what_a_name_holds() {
        let Some(root) = fixture("ctrm-sets-fixture", &[]) else {
            return;
        };
        let listed =
            sets(&Config::discovered(&root), HUMAN).unwrap_or_default();
        assert!(listed.contains("caveman"), "{listed}");
        assert!(listed.contains("U+2192"), "{listed}");
    }

    /// A repo's own declarations are listed beside the builtins, because
    /// the question `sets` answers is "what may I name here".
    #[test]
    fn sets_lists_what_the_repo_declared_too() {
        let files = [(".ctrm-sets", "house U+2261\n")];
        let Some(root) = fixture("ctrm-sets-declared-fixture", &files) else {
            return;
        };
        let listed =
            sets(&Config::discovered(&root), HUMAN).unwrap_or_default();
        assert!(listed.contains("house U+2261"), "{listed}");
    }

    /// `src/charset:V61`: the listing carries every locale, while the
    /// catalog a check reads holds only the locales its rules name.
    #[test]
    fn sets_lists_every_locale_but_a_check_reads_only_the_named() {
        let files = [(".ctrm", "*.md ascii+pl\n")];
        let Some(root) = fixture("ctrm-sets-locale-fixture", &files) else {
            return;
        };
        let config = Config::discovered(&root);
        let listed = sets(&config, HUMAN).unwrap_or_default();
        assert!(listed.contains("ja U+3005"), "{listed}");
        assert!(listed.contains("pt-BR U+"), "{listed}");
        let catalog = config.catalog().unwrap_or_default();
        assert!(catalog.get("pl").is_some());
        assert!(catalog.get("ja").is_none());
    }

    /// V41 is visible here: `marks` holds different characters at another
    /// fidelity, so the listing has to be asked for one.
    #[test]
    fn sets_lists_a_preset_at_the_fidelity_it_was_asked_for() {
        let Some(root) = fixture("ctrm-sets-fidelity-fixture", &[]) else {
            return;
        };
        let text = sets(&Config::discovered(&root), HUMAN).unwrap_or_default();
        let emoji =
            sets(&argued(&root, "--fidelity|emoji"), HUMAN).unwrap_or_default();
        assert!(text.contains("U+2713"), "{text}");
        assert!(emoji.contains("U+2705"), "{emoji}");
        assert_ne!(text, emoji);
    }

    /// B30: `--fidelity f` IS the rule `* @f`, so the listing cannot tell
    /// them apart; it used to read the flag and ignore the rule.
    #[test]
    fn sets_lists_at_the_family_a_rule_names_as_at_the_flag() {
        let Some(root) = fixture("ctrm-sets-rule-family-fixture", &[]) else {
            return;
        };
        let flag = sets(&argued(&root, "--fidelity|emoji"), HUMAN);
        let rule = sets(&argued(&root, "--rule|* @emoji"), HUMAN);
        assert!(flag.is_ok() && flag == rule, "{flag:?} {rule:?}");
    }

    /// B30: a family the map's tree does not declare is refused, at the
    /// argument that named it, rather than resolved as a typo.
    #[test]
    fn an_undeclared_family_is_refused() {
        let Some(root) = fixture("ctrm-sets-bad-family-fixture", &[]) else {
            return;
        };
        let why = argued(&root, "--fidelity|emjoi").validate();
        let why = why.err().unwrap_or_default();
        assert!(why.contains("argv[3]") && why.contains("emjoi"), "{why}");
        assert!(argued(&root, "--rule|* @ascii").validate().is_ok());
    }
}
