//! The path matcher a rule's pattern is read with (T42).
//!
//! Written here rather than taken from a dependency, and rather than
//! reached for in `itok`: that crate's matcher is `pub(crate)`, and it
//! implements the three wildcards WITHOUT gitignore's surface rules, so
//! `*.md` would match at the repo root and nowhere else. V2 promises
//! gitignore semantics, so a wrapper was needed either way.
//!
//! Three wildcards, the ones every developer already knows:
//!
//!   `?`   one character, never `/`
//!   `*`   any run of characters WITHIN one path segment, never `/`
//!   `**`  any run of SEGMENTS, including none
//!
//! `*` stopping at `/` is the load-bearing half: a `*` that crossed
//! separators would make `src/*` silently mean `src/**`, so a rule
//! written for one directory would quietly govern the whole tree.
//!
//! Pure and total: no filesystem, no pattern is invalid. An unmatched
//! `[` is a literal `[`, because a parser that can REJECT a pattern
//! needs a second grammar saying which patterns are legal.
//!
//! HOW it matches (V87): one loop, used twice -- chars within a segment
//! under `*`, segments within a path under `**` -- that remembers only the
//! LAST star and, on a mismatch, lets that star swallow one more unit.
//! Nothing recurses and nothing is allocated, so a pattern like
//! `*a*a*a*a*a*a*b` costs O(pattern x path) instead of exponential time.

/// Whether `path` matches `pattern`, under V2's gitignore semantics.
#[must_use]
pub(crate) fn matches(pattern: &str, path: &str) -> bool {
    starred(Glob::of(pattern), Segments(Some(path)))
}

/// A pattern read as segments, with gitignore's SURFACE rules applied as
/// two virtual `**` segments rather than by building a new string, so the
/// walk below sees only the three wildcards and allocates nothing.
///
/// A pattern with no `/` matches at ANY depth, which is why `*.md` is the
/// spelling people expect to work: it reads as `**/*.md`. A leading `/`
/// anchors to the repo root. A trailing `/` names a directory, so
/// everything under it matches: `locales/` reads as `**/locales/**`.
#[derive(Clone, Copy)]
struct Glob<'a> {
    lead: bool,
    core: Segments<'a>,
    tail: bool,
}

impl<'a> Glob<'a> {
    fn of(pattern: &'a str) -> Self {
        let anchored = pattern.starts_with('/');
        let trimmed = pattern.trim_start_matches('/');
        let core = trimmed.trim_end_matches('/');
        Self {
            lead: !anchored && !core.contains('/'),
            core: Segments(Some(core)),
            tail: trimmed.ends_with('/'),
        }
    }
}

/// What the loop walks: a cursor that hands out one unit and the rest.
trait Units: Copy {
    type Unit;
    fn split(self) -> Option<(Self::Unit, Self)>;
}

/// A pattern: units, which of them is the star, and when a non-star unit
/// accepts one unit of the text.
trait Wildcard: Units {
    type Text: Units;
    fn is_star(unit: &Self::Unit) -> bool;
    fn accepts(unit: &Self::Unit, text: &<Self::Text as Units>::Unit) -> bool;
}

/// The chars of ONE segment, where `/` can no longer appear.
#[derive(Clone, Copy)]
struct Chars<'a>(&'a str);

/// The segments of a path; `None` once the last one is taken. An empty
/// path is one empty segment, as `"".split('/')` reads it.
#[derive(Clone, Copy)]
struct Segments<'a>(Option<&'a str>);

impl Units for Chars<'_> {
    type Unit = char;
    fn split(self) -> Option<(char, Self)> {
        let mut chars = self.0.chars();
        let first = chars.next()?;
        Some((first, Self(chars.as_str())))
    }
}

impl<'a> Units for Segments<'a> {
    type Unit = &'a str;
    fn split(self) -> Option<(&'a str, Self)> {
        let rest = self.0?;
        Some(match rest.split_once('/') {
            Some((head, tail)) => (head, Self(Some(tail))),
            None => (rest, Self(None)),
        })
    }
}

impl<'a> Units for Glob<'a> {
    type Unit = &'a str;
    fn split(self) -> Option<(&'a str, Self)> {
        if self.lead {
            let rest = Self {
                lead: false,
                ..self
            };
            return Some(("**", rest));
        }
        if let Some((unit, core)) = self.core.split() {
            return Some((unit, Self { core, ..self }));
        }
        let rest = Self {
            tail: false,
            ..self
        };
        self.tail.then_some(("**", rest))
    }
}

/// `?` is exactly one char and `*` any run of them, within a segment.
impl Wildcard for Chars<'_> {
    type Text = Self;
    fn is_star(unit: &char) -> bool {
        *unit == '*'
    }
    fn accepts(unit: &char, text: &char) -> bool {
        *unit == '?' || unit == text
    }
}

/// `**` is any run of segments, including none, which is why
/// `src/**/*.rs` also matches `src/a.rs`. Any other segment is matched
/// within one segment of the path, so `*` never crosses a `/`.
impl<'a> Wildcard for Glob<'a> {
    type Text = Segments<'a>;
    fn is_star(unit: &&str) -> bool {
        *unit == "**"
    }
    fn accepts(unit: &&str, text: &&str) -> bool {
        starred(Chars(unit), Chars(text))
    }
}

/// One move of the loop.
enum Step<P, T> {
    /// A star: remember where the pattern resumes after it.
    Star(P),
    /// A unit accepted: both cursors advance.
    Took(P, T),
    /// Both exhausted together: a match.
    Done,
    /// A mismatch: the last star, if any, must swallow more.
    Stuck,
}

fn step<P: Wildcard>(pattern: P, text: P::Text) -> Step<P, P::Text> {
    match (pattern.split(), text.split()) {
        (Some((unit, after)), _) if P::is_star(&unit) => Step::Star(after),
        (None, None) => Step::Done,
        (Some((unit, after)), Some((got, rest))) if P::accepts(&unit, &got) => {
            Step::Took(after, rest)
        }
        _ => Step::Stuck,
    }
}

/// The last star swallows one more unit of the text, or, with the text
/// exhausted or no star seen, the match fails.
fn retry<P: Units, T: Units>(star: Option<(P, T)>) -> Option<(P, T)> {
    let (after, mark) = star?;
    let (_, rest) = mark.split()?;
    Some((after, rest))
}

/// Whether `text` matches `pattern`, remembering only the last star.
///
/// Remembering one is enough: a later star can absorb anything an earlier
/// one would have, so backtracking past it never finds a match it missed.
fn starred<P: Wildcard>(pattern: P, text: P::Text) -> bool {
    let mut at = (pattern, text);
    let mut star = None;
    loop {
        match step(at.0, at.1) {
            Step::Star(after) => (star, at.0) = (Some((after, at.1)), after),
            Step::Took(after, rest) => at = (after, rest),
            Step::Done => return true,
            Step::Stuck => match retry(star) {
                Some(resume) => (star, at) = (Some(resume), resume),
                None => return false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn a_pattern_with_no_slash_matches_at_any_depth() {
        // The spelling people expect: `*.md` is a rule about markdown,
        // not a rule about markdown in the root directory.
        assert!(matches("*.md", "README.md"));
        assert!(matches("*.md", "docs/guide.md"));
        assert!(matches("*.md", "a/b/c/deep.md"));
        assert!(!matches("*.md", "docs/guide.txt"));
    }

    #[test]
    fn a_leading_slash_anchors_to_the_root() {
        assert!(matches("/README.md", "README.md"));
        assert!(!matches("/README.md", "docs/README.md"));
    }

    #[test]
    fn a_pattern_with_an_inner_slash_is_already_anchored() {
        assert!(matches("docs/*.md", "docs/guide.md"));
        assert!(!matches("docs/*.md", "src/docs/guide.md"));
    }

    #[test]
    fn a_trailing_slash_names_a_directory_and_all_under_it() {
        assert!(matches("locales/", "locales/pl.yml"));
        assert!(matches("locales/", "app/locales/pl.yml"));
        assert!(!matches("locales/", "locales.yml"));
    }

    #[test]
    fn a_star_never_crosses_a_separator() {
        // THE load-bearing rule: without it `src/*` silently means
        // `src/**`, and a rule for one directory governs the tree.
        assert!(matches("src/*.rs", "src/main.rs"));
        assert!(!matches("src/*.rs", "src/rules/glob.rs"));
        assert!(matches("src/*", "src/rules"));
        assert!(!matches("src/*", "src/rules/glob.rs"));
    }

    #[test]
    fn a_double_star_crosses_segments_including_none() {
        assert!(matches("src/**/*.rs", "src/rules/glob.rs"));
        assert!(matches("src/**/*.rs", "src/main.rs"));
        assert!(!matches("src/**/*.rs", "tests/a.rs"));
    }

    #[test]
    fn a_question_mark_is_exactly_one_character() {
        assert!(matches("a?.rs", "ab.rs"));
        assert!(!matches("a?.rs", "abc.rs"));
        assert!(!matches("a?.rs", "a.rs"));
        assert!(!matches("a?c", "a/c"));
    }

    #[test]
    fn a_literal_pattern_is_an_exact_path() {
        // V2: a per-type glob and a per-file path share ONE grammar, so a
        // plain path is a pattern that matches itself and nothing else.
        assert!(matches("SPEC.md", "SPEC.md"));
        assert!(matches("src/rules/SPEC.md", "src/rules/SPEC.md"));
        assert!(!matches("src/rules/SPEC.md", "src/scan/SPEC.md"));
    }

    #[test]
    fn no_pattern_is_invalid() {
        // Rejecting a pattern would need a second grammar saying which
        // ones are legal, so unmatched punctuation is a literal.
        assert!(matches("a[b.rs", "a[b.rs"));
        assert!(matches("", ""));
        assert!(!matches("", "a"));
    }
}

#[cfg(test)]
#[path = "glob_test.rs"]
mod differential;
