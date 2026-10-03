//! V87: the iterative matcher answers exactly as the recursive one it
//! replaced, kept here as the reference and nowhere else.

use super::matches;

/// The matcher as it was before V87: correct, but exponential on
/// `*a*a*a*a*a*a*b` and allocating on every call.
mod reference {

    /// Whether `path` matches `pattern`, under V2's gitignore semantics.
    #[must_use]
    pub(super) fn matches(pattern: &str, path: &str) -> bool {
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
            Some((p, rest)) => path.split_first().is_some_and(|(s, srest)| {
                within(p, s) && segments(rest, srest)
            }),
        }
    }

    /// `**` matches zero or more segments, so every remaining suffix is a
    /// candidate. The zero case is why `src/**/*.rs` also matches `src/a.rs`.
    fn any_suffix(rest: &[&str], path: &[&str]) -> bool {
        (0..=path.len())
            .any(|i| segments(rest, path.get(i..).unwrap_or_default()))
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
            Some((&'*', rest)) => (0..=s.len())
                .any(|i| wild(rest, s.get(i..).unwrap_or_default())),
            Some((&'?', rest)) => {
                s.split_first().is_some_and(|(_, t)| wild(rest, t))
            }
            Some((c, rest)) => s
                .split_first()
                .is_some_and(|(f, t)| f == c && wild(rest, t)),
        }
    }
}

/// A deterministic xorshift, so a failing case can be rebuilt by seed.
fn next(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

/// A string of up to `most` pieces drawn from `alphabet`.
fn drawn(state: &mut u32, alphabet: &[&str], most: u32) -> String {
    let bound = most.saturating_add(1);
    let count = next(state).checked_rem(bound).unwrap_or_default();
    let len = u32::try_from(alphabet.len()).unwrap_or(1);
    let mut pick = || {
        let drawn = next(state).checked_rem(len).unwrap_or_default();
        let at = usize::try_from(drawn).unwrap_or_default();
        alphabet.get(at).copied().unwrap_or_default()
    };
    (0..count).map(|_| pick()).collect()
}

/// Every wildcard, both slash positions, and literals that collide.
const PATTERN: &[&str] = &["a", "b", "*", "?", "/", "**", "**/", "["];
const PATH: &[&str] = &["a", "b", "/", "["];

#[test]
fn the_new_matcher_agrees_with_the_old_on_generated_cases() {
    let mut state = 0x9E37_79B9;
    for _ in 0..200_000 {
        let pattern = drawn(&mut state, PATTERN, 7);
        let path = drawn(&mut state, PATH, 8);
        let old = reference::matches(&pattern, &path);
        assert_eq!(matches(&pattern, &path), old, "{pattern:?} vs {path:?}");
    }
}

#[test]
fn the_new_matcher_agrees_on_the_surface_edge_cases() {
    let patterns =
        ["", "/", "//", "a/", "/a", "/a/", "a//b", "**", "/**", "**/"];
    let paths = ["", "/", "a", "a/", "/a", "a//b", "a/b", "b/a/c"];
    for pattern in patterns {
        for path in paths {
            let old = reference::matches(pattern, path);
            assert_eq!(matches(pattern, path), old, "{pattern:?} vs {path:?}");
        }
    }
}

/// The two inputs that were exponential: a star-heavy segment, and a
/// `**`-heavy path. Each now answers at once, and correctly.
#[test]
fn star_heavy_patterns_finish_and_answer_correctly() {
    let name = "a".repeat(80);
    assert!(!matches("*a*a*a*a*a*a*b", &name));
    assert!(matches("*a*a*a*a*a*a*", &name));
    let deep = ["a"; 60].join("/");
    assert!(!matches("**/a/**/a/**/a/**/a/**/b", &deep));
    assert!(matches("**/a/**/a/**/a/**/a/**", &deep));
}
