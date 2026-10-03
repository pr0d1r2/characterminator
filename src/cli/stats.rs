//! The `stats` verb (T11): what the characters cost, and what `fix`
//! would save.
//!
//! Report-only, exit 0 whatever it finds (V7). It answers the question
//! the whole crate exists for -- how many tokens a file costs now, and
//! how many it would cost once the map has run -- and answering is not
//! a verdict, so nothing here gates.
//!
//! The counting is `itok`'s, through `src/tokens`, because a saving this
//! reports has to be the same number `itok` reports elsewhere. The
//! rewriting is `src/fix`'s. This node asks both and prints the pair.

use crate::fix::Map;
use crate::judge::{Checker, Config, File, files};
use crate::render::{self, FileStats, Format, Skipped};
use crate::scan::Unreadable;
use crate::tokens::{self, Count, Method};

/// One file's row, owned so the borrowed render rows can point at it.
struct Row {
    path: String,
    outside: u64,
    bytes: u64,
    now: Count,
    after: Count,
}

/// Run `stats` over a repository.
///
/// `bpe` picks the real tokenizer over the cheap proxy. The default is
/// the ESTIMATE, because a person asking what a tree costs wants an
/// answer now and the figure says which it is (`src/tokens:V10`) -- so
/// the cheap one can never be mistaken for the exact one.
///
/// # Errors
///
/// A configuration that cannot be read, or a file that cannot be
/// counted. A file that cannot be counted is an ERROR rather than a
/// zero: zero is the honest count of an empty file, and reusing it for
/// "unknown" understates a total while looking like a measurement.
pub(super) fn run(
    config: &Config,
    paths: &[String],
    format: Format,
    bpe: bool,
) -> Result<String, String> {
    let pass = Pass {
        checker: Checker::configured(config)?,
        map: config.map()?,
        method: if bpe { Method::Bpe } else { Method::Estimate },
    };
    let (rows, skips) = pass.walk(files(&config.root, paths)?)?;
    let listed: Vec<FileStats<'_>> = rows.iter().map(stat).collect();
    let skipped: Vec<Skipped<'_>> = skips.iter().map(skip).collect();
    Ok(render::stats(format, &listed, &skipped))
}

/// A file that is not text, named as `check` names it (B26).
type Skip = (String, Unreadable);

/// What a walk produced: the rows counted, and the files skipped.
type Walked = (Vec<Row>, Vec<Skip>);

fn skip((path, reason): &Skip) -> Skipped<'_> {
    Skipped {
        path,
        reason: *reason,
    }
}

/// What one run holds for every file, so a per-file call takes the file
/// and nothing else.
struct Pass {
    checker: Checker,
    map: Map,
    method: Method,
}

impl Pass {
    /// Every file in the set, in the order it was given: its numbers, or
    /// why it is not text.
    ///
    /// A file that is not text is left OUT of the counts -- a token figure
    /// for a PNG is a number nothing wrote -- and NAMED as skipped, the
    /// way `check` names it (`src/scan:V8`): a total quietly short of a
    /// file reads as a total over everything (B26).
    fn walk(
        &self,
        files: impl Iterator<Item = Result<File, String>>,
    ) -> Result<Walked, String> {
        let (mut rows, mut skips) = (Vec::new(), Vec::new());
        for file in files {
            let File { shown, text, .. } = file?;
            match text {
                Ok(text) => rows.push(self.counted(&shown, &text)?),
                Err(reason) => skips.push((shown, reason)),
            }
        }
        Ok((rows, skips))
    }

    /// One text's numbers: outside its set, and the cost before and after.
    fn counted(&self, shown: &str, text: &str) -> Result<Row, String> {
        let (set, _) = self.checker.shared_law(shown)?;
        let allowed = |point: char| set.contains(point);
        let fixed = self
            .checker
            .judge(&set)
            .fix(text, &self.map)
            .map_err(|bad| format!("{shown}: {bad}"))?;
        let outside = text.chars().filter(|point| !allowed(*point)).count();
        Ok(Row {
            path: shown.to_owned(),
            outside: outside as u64,
            bytes: text.len() as u64,
            now: tokens::of_text(text, self.method),
            after: tokens::of_text(&fixed.output, self.method),
        })
    }
}

fn stat(row: &Row) -> FileStats<'_> {
    FileStats {
        path: &row.path,
        outside: row.outside,
        bytes: row.bytes,
        now: row.now,
        after: row.after,
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::cli::testkit::fixture;
    use crate::render::Format;
    use std::path::Path;

    fn ran(root: &Path, bpe: bool) -> String {
        let asked = ["notes.md".to_owned()];
        run(
            &crate::cli::config::discovered(root),
            &asked,
            Format::Human,
            bpe,
        )
        .unwrap_or_else(|why| why)
    }

    /// The row carries both figures, which is the point of the verb: what
    /// the file costs now, and what it would cost after `fix`.
    #[test]
    fn a_row_carries_the_cost_now_and_after() {
        // An ellipsis is one character and one token; the `...` it maps to
        // is three characters, so this file gets BIGGER rather than
        // smaller. Chosen on purpose: `stats` reports what is true, not
        // what would sell the tool.
        let files = [("notes.md", "a \u{2026} b\n")];
        let Some(root) = fixture("ctrm-stats-fixture", &files) else {
            return;
        };
        let said = ran(&root, false);
        assert!(said.starts_with("notes.md outside 1 bytes"), "{said}");
        assert!(said.contains("tokens"), "{said}");
        assert!(said.contains("->"), "{said}");
    }

    /// V10: the figure says how it was measured, so the cheap proxy can
    /// never be read as an exact count.
    #[test]
    fn the_figure_says_which_method_produced_it() {
        let files = [("notes.md", "plain ascii\n")];
        let Some(root) = fixture("ctrm-stats-method-fixture", &files) else {
            return;
        };
        assert!(ran(&root, false).contains('~'), "estimate wears a tilde");
        assert!(ran(&root, true).contains("o200k"), "bpe names its encoding");
    }

    /// B21: a file `check` skips as binary is not counted either -- and,
    /// B26, it is NAMED, as `check` names it.
    #[test]
    fn a_binary_file_is_not_counted() {
        let files = [("notes.md", "\0\u{2014}data\0")];
        let Some(root) = fixture("ctrm-stats-binary-fixture", &files) else {
            return;
        };
        assert_eq!(ran(&root, false), "notes.md: skipped, binary");
    }

    /// B26: a file that is not UTF-8 was dropped from `stats` without a
    /// word. It is named, with the byte where decoding failed.
    #[test]
    fn a_file_that_is_not_utf8_is_named() {
        let Some(root) = fixture("ctrm-stats-not-utf8-fixture", &[]) else {
            return;
        };
        if std::fs::write(root.join("notes.md"), b"ab\xffcd\n").is_err() {
            return;
        }
        let said = ran(&root, false);
        assert_eq!(said, "notes.md: invalid UTF-8 at byte 2");
    }

    /// A file already inside its set costs the same before and after:
    /// there is nothing to rewrite, so the two figures match.
    #[test]
    fn a_clean_file_costs_the_same_after() {
        let files = [("notes.md", "plain ascii\n")];
        let Some(root) = fixture("ctrm-stats-clean-fixture", &files) else {
            return;
        };
        let said = ran(&root, false);
        assert!(said.contains("outside 0"), "{said}");
        let halves: Vec<&str> = said.split(" -> ").collect();
        assert_eq!(halves.len(), 2, "{said}");
        assert!(
            halves.last().is_some_and(|after| {
                halves.first().is_some_and(|now| now.ends_with(after))
            }),
            "{said}"
        );
    }
}
