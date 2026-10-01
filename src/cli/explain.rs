//! The `explain` and `sets` verbs (T12): what is in force, and why.
//!
//! Both are REPORT-ONLY and exit 0 whatever they find (V7). They answer
//! questions about configuration rather than about files, so there is
//! nothing for them to fail at: a path with no rule is an answer, not a
//! violation.
//!
//! They exist because every other verb's verdict rests on a resolution
//! nobody can see. `check` says a character is outside `ascii+caveman`;
//! `explain` says which line of which file decided that, and `sets` says
//! what those names hold.

use super::check::Checker;
use super::config::Config;
use crate::charset::CharSet;
use crate::render::{Explanation, Format};
use crate::render::{explain as render_explain, sets as render_sets};

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
pub fn run(
    config: &Config,
    paths: &[String],
    format: Format,
) -> Result<String, String> {
    let checker = Checker::configured(config)?;
    let asked = paths.first().map(String::as_str);
    let (set, winner) = checker.effective(asked.unwrap_or(""))?;
    Ok(render_explain(
        format,
        &Explanation {
            path: asked,
            set: &set,
            rule: winner.as_ref(),
        },
    ))
}

/// `sets`: every declared set and what it holds.
///
/// Resolved at the DEFAULT fidelity, and the listing says so rather than
/// hiding it: a preset with labelled members (`src/charset:V41`) holds
/// different characters at another one, and a listing that quietly showed
/// one spelling would be a wrong answer about `marks`.
///
/// # Errors
///
/// As [`Checker::configured`], plus a declared set that cannot be resolved --
/// a cycle, or a member naming a set nothing declares.
pub fn sets(
    config: &Config,
    family: &str,
    format: Format,
) -> Result<String, String> {
    let checker = Checker::configured(config)?;
    let listed: Vec<CharSet> = checker.declared(family)?;
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

    #[test]
    fn sets_lists_what_a_name_holds() {
        let Some(root) = fixture("ctrm-sets-fixture", &[]) else {
            return;
        };
        let listed = sets(&Config::discovered(&root), "text", Format::Human)
            .unwrap_or_default();
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
        let listed = sets(&Config::discovered(&root), "text", Format::Human)
            .unwrap_or_default();
        assert!(listed.contains("house U+2261"), "{listed}");
    }

    /// V41 is visible here: `marks` holds different characters at another
    /// fidelity, so the listing has to be asked for one.
    #[test]
    fn sets_lists_a_preset_at_the_fidelity_it_was_asked_for() {
        let Some(root) = fixture("ctrm-sets-fidelity-fixture", &[]) else {
            return;
        };
        let text = sets(&Config::discovered(&root), "text", Format::Human)
            .unwrap_or_default();
        let emoji = sets(&Config::discovered(&root), "emoji", Format::Human)
            .unwrap_or_default();
        assert!(text.contains("U+2713"), "{text}");
        assert!(emoji.contains("U+2705"), "{emoji}");
        assert_ne!(text, emoji);
    }
}
