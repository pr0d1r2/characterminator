//! Applying a map: the `fix` pass and the `--check` reporting mode.
//!
//! One pass walks the text and rewrites a span ONLY when the span covers a
//! character the caller's predicate disallows, so every other byte is copied
//! through untouched (V6). A character with no mapping is copied through and
//! reported rather than dropped (V4). A replacement is itself run through
//! the map before it is emitted, so a chain (skin tone to thumbs up to `+1`)
//! lands on its fixed point in one pass and a second run changes nothing
//! (V5).
//!
//! Whether a character is ALLOWED is not decided here. It arrives as a
//! predicate, because that answer belongs to the charset and rules nodes.

use crate::fix::map::Map;
use crate::fix::{Error, Rewrite};
use crate::scan::{Hit, Position};

/// What `fix` found, with no text to write: the `fix --check` answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Every span that would be rewritten, in the order they were found.
    pub rewrites: Vec<Rewrite>,
    /// Every disallowed character that has no mapping, kept as it was (V4).
    pub unmapped: Vec<Hit>,
}

impl Report {
    /// Whether writing would change the file. `fix --check` exits 1 on it.
    #[must_use]
    pub fn drifted(&self) -> bool {
        !self.rewrites.is_empty()
    }
}

/// The rewritten text together with the report that explains it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fixed {
    pub output: String,
    pub report: Report,
}

/// Rewrite every disallowed span of `text` that the map covers.
///
/// The V6 and V5 guards run BEFORE this returns, so a caller that writes
/// what it gets back cannot write a file that failed either.
pub fn fix(
    text: &str,
    map: &Map,
    allowed: &dyn Fn(char) -> bool,
) -> Result<Fixed, Error> {
    let pass = run(text, map, allowed, map.budget())?;
    if !untouched_bytes_match(text, &pass) {
        return Err(Error::TouchedAllowedBytes);
    }
    let again = run(&pass.output, map, allowed, map.budget())?;
    if again.output != pass.output {
        return Err(Error::NotIdempotent);
    }
    Ok(pass.finish())
}

/// Report what `fix` would do and hand back nothing to write (`--check`).
pub fn check(
    text: &str,
    map: &Map,
    allowed: &dyn Fn(char) -> bool,
) -> Result<Report, Error> {
    Ok(fix(text, map, allowed)?.report)
}

/// One walk over one text.
#[derive(Debug, Default)]
struct Pass {
    output: String,
    spans: Vec<Span>,
    unmapped: Vec<Hit>,
}

impl Pass {
    fn finish(self) -> Fixed {
        let rewrites = self.spans.into_iter().map(Span::into_rewrite).collect();
        let unmapped = self.unmapped;
        Fixed {
            output: self.output,
            report: Report { rewrites, unmapped },
        }
    }
}

/// A rewritten span. The public `Rewrite` names the first character; the
/// byte length stays here, because it is what the V6 guard needs.
#[derive(Debug)]
struct Span {
    hit: Hit,
    len: usize,
    to: String,
}

impl Span {
    fn into_rewrite(self) -> Rewrite {
        Rewrite {
            hit: self.hit,
            to: self.to,
        }
    }
}

fn run<'a>(
    text: &'a str,
    map: &'a Map,
    allowed: &'a dyn Fn(char) -> bool,
    budget: usize,
) -> Result<Pass, Error> {
    let mut walk = Run {
        text,
        map,
        allowed,
        budget,
        at: Cursor::start(),
        pass: Pass::default(),
    };
    walk.walk()?;
    Ok(walk.pass)
}

/// What a match at one position yields: the source span and the
/// replacement it settled on, or nothing to rewrite here.
type Found<'a> = Result<Option<(&'a str, String)>, Error>;

struct Run<'a> {
    text: &'a str,
    map: &'a Map,
    allowed: &'a dyn Fn(char) -> bool,
    budget: usize,
    at: Cursor,
    pass: Pass,
}

impl Run<'_> {
    fn walk(&mut self) -> Result<(), Error> {
        while let Some(rest) = self.text.get(self.at.byte..) {
            let Some(ch) = rest.chars().next() else { break };
            match self.matched(rest)? {
                Some((source, to)) => self.rewrite(source, ch, to)?,
                None => self.keep(ch),
            }
        }
        Ok(())
    }

    /// The span to rewrite here, if any. A zero-length match is refused: it
    /// would leave the cursor where it is.
    fn matched<'b>(&self, rest: &'b str) -> Found<'b> {
        let Some(found) = self.map.resolve_at(rest, self.allowed)? else {
            return Ok(None);
        };
        let Some(source) = rest.get(..found.len).filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        Ok(Some((source, found.to)))
    }

    fn rewrite(
        &mut self,
        source: &str,
        ch: char,
        to: String,
    ) -> Result<(), Error> {
        let to = self.expand(&to)?;
        let hit = Hit {
            position: self.at.position(),
            character: ch,
        };
        self.pass.output.push_str(&to);
        for c in source.chars() {
            self.at.advance(c);
        }
        self.pass.spans.push(Span {
            hit,
            len: source.len(),
            to,
        });
        Ok(())
    }

    /// Rewrite the replacement itself, so what is emitted is already the
    /// fixed point of the map (V5). A map that never settles is cyclic, and
    /// a cyclic map is a config error rather than a silent half-rewrite.
    fn expand(&self, to: &str) -> Result<String, Error> {
        let Some(budget) = self.budget.checked_sub(1) else {
            return Err(Error::MapCycle);
        };
        Ok(run(to, self.map, self.allowed, budget)?.output)
    }

    fn keep(&mut self, ch: char) {
        if !(self.allowed)(ch) {
            self.pass.unmapped.push(Hit {
                position: self.at.position(),
                character: ch,
            });
        }
        self.pass.output.push(ch);
        self.at.advance(ch);
    }
}

/// Where the walk is, in the three units `Position` reports.
#[derive(Debug)]
struct Cursor {
    line: usize,
    column: usize,
    byte: usize,
}

impl Cursor {
    fn start() -> Self {
        Self {
            line: 1,
            column: 1,
            byte: 0,
        }
    }

    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
            byte: self.byte,
        }
    }

    fn advance(&mut self, ch: char) {
        self.byte = self.byte.saturating_add(ch.len_utf8());
        if ch == '\n' {
            self.line = self.line.saturating_add(1);
            self.column = 1;
        } else {
            self.column = self.column.saturating_add(1);
        }
    }
}

/// V6, asserted BEFORE a caller can write: every byte outside a rewritten
/// span is identical in the input and the output.
fn untouched_bytes_match(text: &str, pass: &Pass) -> bool {
    let mut cut = Cut::default();
    for span in &pass.spans {
        if !cut.gap_matches(text, &pass.output, span.hit.position.byte) {
            return false;
        }
        cut.skip(span);
    }
    cut.tail_matches(text, &pass.output)
}

/// How far the comparison has read, on each side.
#[derive(Debug, Default)]
struct Cut {
    input: usize,
    output: usize,
}

impl Cut {
    fn gap_matches(&self, text: &str, out: &str, next: usize) -> bool {
        let before = text.get(self.input..next);
        let end = self.output.checked_add(next.saturating_sub(self.input));
        let after = end.and_then(|stop| out.get(self.output..stop));
        before.is_some() && before == after
    }

    fn skip(&mut self, span: &Span) {
        let gap = span.hit.position.byte.saturating_sub(self.input);
        self.output = self
            .output
            .saturating_add(gap)
            .saturating_add(span.to.len());
        self.input = span.hit.position.byte.saturating_add(span.len);
    }

    fn tail_matches(&self, text: &str, out: &str) -> bool {
        text.get(self.input..) == out.get(self.output..)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cut, Fixed, Pass, Span, check, fix, untouched_bytes_match};
    use crate::fix::map::Map;
    use crate::fix::{Error, ROOT};
    use crate::rules::Origin;
    use crate::scan::{Hit, Position};

    /// A map in the shape the builtin will have (V26, V31): typography to
    /// ASCII, one explicit delete, and one declared sequence whose prefix is
    /// also declared.
    const MAP: &str = "\
        # em dash, en dash, ellipsis, no-break space, zero width space.\n\
        U+2014 --\n\
        U+2013 -\n\
        U+2026 ...\n\
        U+00A0 U+0020\n\
        U+200B\n\
        # a sequence, its prefix, and a sequence that opens with ASCII.\n\
        U+1F44D+U+1F3FD U+1F44D\n\
        U+1F44D +1\n\
        U+0031+U+20E3 1\n";

    /// The inputs the properties below are asserted over. Each one is here
    /// for a reason a random generator would not reliably produce:
    /// the empty text; a text with nothing to do; a violation in the middle,
    /// alone, and doubled with no gap; a declared sequence whose prefix is
    /// also declared; that prefix on its own; a sequence whose FIRST code
    /// point is allowed, which a per-character check would miss; a violation
    /// with no mapping at all (V4); an explicit delete, which shortens the
    /// text; a replacement written in code point form; a violation on the
    /// second line, which only a position bug would misplace; and a mapped
    /// violation between two multi-byte characters that have no mapping,
    /// which is where a byte-sloppy copy shows up.
    const INPUTS: &[&str] = &[
        "",
        "plain ascii, nothing to do\n",
        "a\u{2014}b",
        "\u{2014}",
        "\u{2014}\u{2013}",
        "a\u{1F44D}\u{1F3FD}b",
        "a\u{1F44D}b",
        "1\u{20E3}",
        "a\u{2764}b",
        "a\u{200B}b",
        "a\u{00A0}b",
        "one\n\u{2014}\ntwo\n",
        "\u{00E9}\u{2014}\u{00E9}",
    ];

    /// What each input becomes when only ASCII is allowed.
    const WANT: &[&str] = &[
        "",
        "plain ascii, nothing to do\n",
        "a--b",
        "--",
        "---",
        "a+1b",
        "a+1b",
        "1",
        "a\u{2764}b",
        "ab",
        "a b",
        "one\n--\ntwo\n",
        "\u{00E9}--\u{00E9}",
    ];

    fn ascii(ch: char) -> bool {
        ch.is_ascii()
    }

    fn ascii_or_em_dash(ch: char) -> bool {
        ch.is_ascii() || ch == '\u{2014}'
    }

    fn anything(_: char) -> bool {
        true
    }

    type Predicate = fn(char) -> bool;

    const PREDICATES: &[Predicate] = &[ascii, ascii_or_em_dash, anything];

    fn map() -> Map {
        Map::parse(MAP, &|line| Origin::Builtin { line }).unwrap_or_default()
    }

    /// A span standing at `byte`, for the guard tests below. Which character
    /// it names does not matter to either guard; the byte length does.
    fn span(byte: usize, len: usize, to: &str) -> Span {
        Span {
            hit: Hit {
                position: Position {
                    line: 1,
                    column: byte.saturating_add(1),
                    byte,
                },
                character: '\u{2014}',
            },
            len,
            to: String::from(to),
        }
    }

    fn pass_of(output: &str, spans: Vec<Span>) -> Pass {
        Pass {
            output: String::from(output),
            spans,
            unmapped: Vec::new(),
        }
    }

    fn output(text: &str, allowed: &dyn Fn(char) -> bool) -> String {
        fix(text, &map(), allowed)
            .map(|fixed| fixed.output)
            .unwrap_or_else(|_| String::from("<error>"))
    }

    #[test]
    fn the_fixture_map_parses() {
        assert!(Map::parse(MAP, &|line| Origin::Builtin { line }).is_ok());
        assert_eq!(INPUTS.len(), WANT.len());
    }

    #[test]
    fn every_input_is_rewritten_as_declared() {
        for (input, want) in INPUTS.iter().zip(WANT.iter()) {
            assert_eq!(&output(input, &ascii), want, "input {input:?}");
        }
    }

    #[test]
    fn fix_is_idempotent() {
        for input in INPUTS {
            for allowed in PREDICATES {
                let once = output(input, allowed);
                let twice = output(&once, allowed);
                assert_eq!(twice, once, "input {input:?}");
            }
        }
    }

    #[test]
    fn an_allowed_text_keeps_every_byte() {
        for input in INPUTS {
            assert_eq!(&output(input, &anything), input, "input {input:?}");
        }
    }

    #[test]
    fn an_empty_map_keeps_every_byte() {
        for input in INPUTS {
            let fixed = fix(input, &Map::default(), &ascii);
            let text = fixed.map(|done| done.output).unwrap_or_default();
            assert_eq!(&text, input, "input {input:?}");
        }
    }

    #[test]
    fn a_granted_character_is_left_alone() {
        let text = "a\u{2014}\u{2013}b";
        assert_eq!(output(text, &ascii_or_em_dash), "a\u{2014}-b");
    }

    #[test]
    fn a_character_with_no_mapping_is_kept_and_reported() {
        let report = check("a\u{2764}b", &map(), &ascii).unwrap_or_default();
        let found = report.unmapped.first().copied();
        assert_eq!(found.map(|hit| hit.character), Some('\u{2764}'));
        assert_eq!(found.map(|hit| hit.position.column), Some(2));
        assert!(!report.drifted());
    }

    #[test]
    fn the_longest_declared_sequence_wins_and_chains() {
        let text = "a\u{1F44D}\u{1F3FD}b";
        let report = check(text, &map(), &ascii).unwrap_or_default();
        assert_eq!(report.rewrites.len(), 1);
        assert_eq!(
            report.rewrites.first().map(|done| done.to.as_str()),
            Some("+1")
        );
        assert!(report.drifted());
    }

    #[test]
    fn a_sequence_opening_with_an_allowed_character_is_a_violation() {
        let report = check("1\u{20E3}", &map(), &ascii).unwrap_or_default();
        assert_eq!(report.rewrites.len(), 1);
        assert_eq!(report.unmapped.len(), 0);
    }

    #[test]
    fn a_rewrite_is_reported_where_it_was_found() {
        let report =
            check("one\n\u{2014}\ntwo\n", &map(), &ascii).unwrap_or_default();
        let at = report.rewrites.first().map(|done| done.hit.position);
        assert_eq!(
            at.map(|pos| (pos.line, pos.column, pos.byte)),
            Some((2, 1, 4))
        );
    }

    #[test]
    fn a_cyclic_map_is_an_error_rather_than_a_half_rewrite() {
        let source = "U+2014 U+2013\nU+2013 U+2014\n";
        let cyclic = Map::parse(source, &|line| Origin::Builtin { line })
            .unwrap_or_default();
        assert_eq!(fix("\u{2014}", &cyclic, &ascii), Err(Error::MapCycle));
    }

    /// A map that rewrites through CLASSES rather than entries, including a
    /// family the builtin tree does not carry.
    const CLASSES: &str = "\
        family nerd emoji\n\
        = tick ascii:[x] text:U+2713 emoji:U+2705 nerd:U+F00C\n\
        = hand emoji:U+1F44D+U+1F3FD,U+1F44D ascii:+1\n";

    fn classed(fidelity: &str) -> Map {
        Map::parse(CLASSES, &|line| Origin::Builtin { line })
            .and_then(|map| map.with_fidelity(fidelity))
            .unwrap_or_default()
    }

    fn ascii_or_emoji(ch: char) -> bool {
        ch.is_ascii() || ch == '\u{2705}' || ch == '\u{1F44D}'
    }

    fn ascii_or_nerd(ch: char) -> bool {
        ch.is_ascii() || ch == '\u{F00C}'
    }

    #[test]
    fn a_class_falls_back_along_the_path_to_ascii() {
        let done = fix("a\u{2713}b", &classed("emoji"), &ascii);
        assert_eq!(done.map(|f| f.output).unwrap_or_default(), "a[x]b");
    }

    #[test]
    fn a_class_compresses_into_the_fidelity_family() {
        let done = fix("\u{2713}", &classed("emoji"), &ascii_or_emoji);
        assert_eq!(done.map(|f| f.output).unwrap_or_default(), "\u{2705}");
    }

    #[test]
    fn a_declared_family_takes_part_in_resolution() {
        let done = fix("\u{2713}", &classed("nerd"), &ascii_or_nerd);
        assert_eq!(done.map(|f| f.output).unwrap_or_default(), "\u{F00C}");
    }

    #[test]
    fn the_longest_class_member_wins() {
        let text = "\u{1F44D}\u{1F3FD}";
        let report = check(text, &classed(ROOT), &ascii).unwrap_or_default();
        assert_eq!(report.rewrites.len(), 1);
        let first = report.rewrites.first().map(|done| done.to.clone());
        assert_eq!(first, Some(String::from("+1")));
    }

    #[test]
    fn a_class_with_no_allowed_member_is_kept_and_reported() {
        let source = "= tick text:U+2713 emoji:U+2705\n";
        let map = Map::parse(source, &|line| Origin::Builtin { line })
            .unwrap_or_default();
        let report = check("\u{2713}", &map, &ascii).unwrap_or_default();
        assert_eq!(report.unmapped.len(), 1);
        assert!(!report.drifted());
        let done = fix("\u{2713}", &map, &ascii).unwrap_or_default();
        assert_eq!(done.output, "\u{2713}");
    }

    #[test]
    fn check_reports_without_handing_back_text_to_write() {
        let report = check("a\u{2014}b", &map(), &ascii).unwrap_or_default();
        assert!(report.drifted());
        assert_eq!(report.rewrites.len(), 1);
    }

    /// The V6 guard has to be able to FAIL, or asserting it proves nothing.
    #[test]
    fn the_untouched_guard_catches_a_changed_byte() {
        let good = pass_of("a--b", vec![span(1, 3, "--")]);
        assert!(untouched_bytes_match("a\u{2014}b", &good));
        let bad = pass_of("a--B", vec![span(1, 3, "--")]);
        assert!(!untouched_bytes_match("a\u{2014}b", &bad));
    }

    #[test]
    fn a_fix_with_nothing_to_do_reports_nothing() {
        let done = fix("plain", &map(), &ascii).unwrap_or_default();
        assert_eq!(done, Fixed::default_with("plain"));
    }

    #[test]
    fn the_cut_reads_both_sides_in_step() {
        let mut cut = Cut::default();
        assert!(cut.gap_matches("abc", "abCC", 2));
        cut.skip(&span(2, 1, "CC"));
        assert!(cut.tail_matches("abc", "abCC"));
    }

    impl Fixed {
        fn default_with(text: &str) -> Self {
            Self {
                output: String::from(text),
                ..Self::default()
            }
        }
    }
}
