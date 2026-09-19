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

/// Whether `path` matches `pattern`, under V2's gitignore semantics.
#[must_use]
pub fn matches(pattern: &str, path: &str) -> bool {
    let normal = normalize(pattern);
    let pat: Vec<&str> = normal.split('/').collect();
    let seg: Vec<&str> = path.split('/').collect();
    segments(&pat, &seg)
}

/// Gitignore's SURFACE rules, applied once so the walk below sees only
/// the three wildcards.
///
/// A pattern with no `/` matches at ANY depth, which is why `*.md` is the
/// spelling people expect to work. A leading `/` anchors to the repo root.
/// A trailing `/` names a directory, so everything under it matches.
fn normalize(pattern: &str) -> String {
    let anchored = pattern.starts_with('/');
    let trimmed = pattern.trim_start_matches('/');
    let directory = trimmed.ends_with('/');
    let core = trimmed.trim_end_matches('/');
    let mut out = String::new();
    if !anchored && !core.contains('/') {
        out.push_str("**/");
    }
    out.push_str(core);
    if directory {
        out.push_str("/**");
    }
    out
}

/// Segment-wise match. `**` is the only pattern that consumes more, or
/// fewer, than one segment.
fn segments(pat: &[&str], path: &[&str]) -> bool {
    match pat.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => any_suffix(rest, path),
        Some((p, rest)) => path
            .split_first()
            .is_some_and(|(s, srest)| within(p, s) && segments(rest, srest)),
    }
}

/// `**` matches zero or more segments, so every remaining suffix is a
/// candidate. The zero case is why `src/**/*.rs` also matches `src/a.rs`.
fn any_suffix(rest: &[&str], path: &[&str]) -> bool {
    (0..=path.len()).any(|i| segments(rest, path.get(i..).unwrap_or_default()))
}

/// Within ONE segment, where `/` can no longer appear.
fn within(pattern: &str, seg: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = seg.chars().collect();
    wild(&p, &s)
}

fn wild(p: &[char], s: &[char]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((&'*', rest)) => {
            (0..=s.len()).any(|i| wild(rest, s.get(i..).unwrap_or_default()))
        }
        Some((&'?', rest)) => {
            s.split_first().is_some_and(|(_, t)| wild(rest, t))
        }
        Some((c, rest)) => s
            .split_first()
            .is_some_and(|(f, t)| f == c && wild(rest, t)),
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
