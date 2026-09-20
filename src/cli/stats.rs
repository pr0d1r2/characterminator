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

use super::check::Checker;
use super::fix as fixer;
use crate::fix::{self as engine, Map};
use crate::render::{self, FileStats, Format};
use crate::tokens::{self, Count, Method};
use std::path::{Path, PathBuf};

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
pub fn run(
    root: &Path,
    paths: &[String],
    format: Format,
    bpe: bool,
) -> Result<String, String> {
    let checker = Checker::load(root)?;
    let map = fixer::map_of(root)?;
    let method = if bpe { Method::Bpe } else { Method::Estimate };
    let files = tokens::select(root, paths)
        .map_err(|bad| format!("{}: {}", bad.path.display(), bad.reason))?;
    let pass = Pass {
        checker,
        map,
        method,
    };
    let rows = pass.walk(root, &files)?;
    let listed: Vec<FileStats<'_>> = rows.iter().map(stat).collect();
    Ok(render::stats(format, &listed))
}

/// What one run holds for every file, so a per-file call takes the file
/// and nothing else.
struct Pass {
    checker: Checker,
    map: Map,
    method: Method,
}

impl Pass {
    /// Every file in the set, in the order it was given.
    fn walk(&self, root: &Path, files: &[PathBuf]) -> Result<Vec<Row>, String> {
        let mut rows = Vec::new();
        for full in files {
            let shown = super::check::shown_path(root, full);
            if let Some(row) = self.measure(full, shown)? {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// One file's numbers, or `None` when it is not text.
    ///
    /// A file that is not text is left OUT rather than counted: a token
    /// figure for a PNG is a number nothing wrote, and `check` is the
    /// verb that names an unreadable file.
    fn measure(
        &self,
        full: &Path,
        shown: String,
    ) -> Result<Option<Row>, String> {
        let (set, _) = self.checker.effective(&shown)?;
        let Some(text) = text_of(full)? else {
            return Ok(None);
        };
        let allowed = |point: char| set.contains(point);
        let fixed = engine::fix(&text, &self.map, &allowed)
            .map_err(|bad| format!("{shown}: {bad}"))?;
        let outside = text.chars().filter(|point| !allowed(*point)).count();
        Ok(Some(Row {
            path: shown,
            outside: outside as u64,
            bytes: text.len() as u64,
            now: tokens::of_text(&text, self.method),
            after: tokens::of_text(&fixed.output, self.method),
        }))
    }
}

/// A file's text, or `None` when its bytes are not UTF-8.
fn text_of(full: &Path) -> Result<Option<String>, String> {
    let bytes =
        std::fs::read(full).map_err(|e| format!("{}: {e}", full.display()))?;
    Ok(String::from_utf8(bytes).ok())
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
    use crate::render::Format;
    use std::path::{Path, PathBuf};

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

    fn ran(root: &Path, bpe: bool) -> String {
        let asked = ["notes.md".to_owned()];
        run(root, &asked, Format::Human, bpe).unwrap_or_else(|why| why)
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
