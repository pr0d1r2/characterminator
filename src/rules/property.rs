//! The property V18 states, tested: a file EQUALS one flag per line.
//!
//! A flag twin is only a twin if it can say everything a line of its file
//! can say, and nothing more. Example-based tests check the lines someone
//! thought of; this generates files from a corpus of line shapes -- valid
//! lines, comments, blanks, odd spacing -- and asserts that parsing the
//! file and parsing its lines one flag at a time reach the same entries.
//!
//! The generator is a plain linear congruential sequence with a fixed
//! seed rather than a property-testing dependency: the run has to be
//! deterministic and offline, and what varies here is which lines land in
//! which order, which needs no shrinking machinery to be readable.
//!
//! Both sides are compared as RESULTS, so a file that fails and a flag
//! sequence that fails have to fail the same way. Without that half, a
//! twin could be "equal" by being quietly more permissive.
//!
//! The whole file is test-only.

#[cfg(test)]
mod tests {
    use crate::rules::line::{ParseError, error, parse_file, parse_flag};
    use crate::rules::rule_line::parse_rule;
    use crate::rules::{Origin, Rule, Sources};
    use std::path::Path;

    /// The line shapes a generated file is built from. Comments, blanks
    /// and a whitespace-only line are IN the corpus on purpose: they are
    /// the lines that yield no entry, and the property has to survive
    /// them on both sides.
    const SHAPES: [&str; 12] = [
        "* caveman",
        "*.md ascii+caveman",
        "# a comment",
        "",
        "   ",
        "SPEC.md ascii+caveman+box",
        "docs/** marks @emoji",
        "src/**/*.rs ascii !deny",
        "vendor/** any !allow !pedantic=allow",
        "   *.po  pl+typography  ",
        "locales/** ascii+latin-ext @text",
        "Makefile\tascii+cr",
    ];

    /// Line shapes of a DIFFERENT grammar, to show the skeleton is not
    /// specific to `.ctrm`.
    const PAIRS: [&str; 6] = [
        "U+2014 --",
        "U+2019 '",
        "# typography",
        "",
        "U+2026 ...",
        "U+2212 -",
    ];

    /// A line no parser accepts, spliced in to test failure parity.
    const BAD: &str = "*.md !loud";

    /// A fixed-seed sequence. `wrapping_*` throughout: the arithmetic is
    /// meant to wrap, and saying so is what keeps it from being an
    /// overflow.
    struct Noise(u64);

    impl Noise {
        fn next(&mut self) -> usize {
            let stepped = self.0.wrapping_mul(6364136223846793005);
            self.0 = stepped.wrapping_add(1442695040888963407);
            usize::try_from(self.0 >> 33).unwrap_or(0)
        }

        /// One shape out of `shapes`.
        fn pick<'a>(&mut self, shapes: &[&'a str]) -> &'a str {
            let index = self.next().checked_rem(shapes.len()).unwrap_or(0);
            shapes.get(index).copied().unwrap_or_default()
        }
    }

    /// Build a file of `count` lines.
    fn file(noise: &mut Noise, shapes: &[&str], count: usize) -> String {
        let mut text = String::new();
        for _ in 0..count {
            text.push_str(noise.pick(shapes));
            text.push('\n');
        }
        text
    }

    fn path() -> &'static Path {
        Path::new(".ctrm")
    }

    /// A stand-in for a second kind's line parser.
    ///
    /// The map and sets grammars belong to `src/fix` and `src/charset`
    /// and are not written yet, and this node must not call into a
    /// sibling. What V18 asks of THIS node is that the skeleton be
    /// kind-agnostic, and a parser shaped like a different grammar -- two
    /// fields rather than one and a tail -- is what demonstrates it.
    fn pair(text: &str, origin: Origin) -> Result<String, ParseError> {
        match text.split_once(char::is_whitespace) {
            Some((from, to)) => Ok(format!("{from}>{}", to.trim())),
            None => Err(error(origin, "a pair line needs two fields")),
        }
    }

    /// Compare entries WITHOUT their origins.
    ///
    /// The origins necessarily differ -- that is V20 doing its job, a
    /// file line saying `.ctrm:4` where a flag says `argv[3]`. What V18
    /// claims is that everything else is identical.
    fn forget(rules: Result<Vec<Rule>, String>) -> Result<Vec<Rule>, String> {
        rules.map(|mut rules| {
            for rule in &mut rules {
                rule.origin = Origin::Builtin { line: 0 };
            }
            rules
        })
    }

    /// Every line of `text` as one flag, stopping where a file would.
    fn as_flags<T, P>(text: &str, parse: P) -> Result<Vec<T>, String>
    where
        P: Fn(&str, Origin) -> Result<T, ParseError> + Copy,
    {
        let mut entries = Vec::new();
        for (index, line) in text.lines().enumerate() {
            match parse_flag(line, index, parse) {
                Ok(entry) => entries.extend(entry),
                Err(failure) => return Err(failure.message),
            }
        }
        Ok(entries)
    }

    /// The same text as a whole file.
    fn as_file<T, P>(text: &str, parse: P) -> Result<Vec<T>, String>
    where
        P: Fn(&str, Origin) -> Result<T, ParseError>,
    {
        parse_file(text, path(), parse).map_err(|fail| fail.message)
    }

    /// One `Sources`' rules, origins erased and a failure reduced to its
    /// message, so two chains can be compared the way two parses are.
    fn chain(sources: &Sources) -> Result<Vec<Rule>, String> {
        forget(sources.rules().map_err(|fail| fail.message))
    }

    /// Every line of `text` as its own `--rule`, in argv order.
    fn as_argv(text: &str) -> Sources {
        let lines = text.lines().enumerate();
        Sources::from_argv(lines.map(|(at, line)| (at, line.to_string())))
    }

    /// Splice a line into `text` at a position the sequence chooses.
    fn splice(noise: &mut Noise, text: &str, line: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let at = noise.next().checked_rem(lines.len().max(1)).unwrap_or(0);
        let head = lines.iter().take(at).copied();
        let tail = lines.iter().skip(at).copied();
        let all: Vec<&str> = head.chain([line]).chain(tail).collect();
        all.join("\n")
    }

    #[test]
    fn a_rules_file_equals_one_flag_per_line() {
        let mut noise = Noise(0x5EED);
        for count in 0..64 {
            let text = file(&mut noise, &SHAPES, count);
            let as_file = forget(as_file(&text, parse_rule));
            assert_eq!(as_file, forget(as_flags(&text, parse_rule)));
        }
    }

    /// The property in the words V18 actually uses: `--no-files` plus
    /// one flag per line of F is F. The bare-parse version above is the
    /// same claim one layer down; this one runs through the precedence
    /// chain, where a file is a source and a flag is another.
    #[test]
    fn no_files_plus_one_flag_per_line_equals_the_file() {
        let mut noise = Noise(0xF11E5);
        for count in 0..64 {
            let text = file(&mut noise, &SHAPES, count);
            let as_file = Sources::new().dotfile(".ctrm", text.clone());
            assert_eq!(chain(&as_file), chain(&as_argv(&text)));
        }
    }

    #[test]
    fn the_equality_holds_for_a_second_grammar_too() {
        let mut noise = Noise(0xC0FFEE);
        for count in 0..64 {
            let text = file(&mut noise, &PAIRS, count);
            let from_file: Result<Vec<String>, String> = as_file(&text, pair);
            assert_eq!(from_file, as_flags(&text, pair));
        }
    }

    #[test]
    fn a_file_that_fails_fails_the_same_way_as_its_flags() {
        let mut noise = Noise(0xBADF00D);
        for count in 0..64 {
            let text = file(&mut noise, &SHAPES, count);
            let spliced = splice(&mut noise, &text, BAD);
            let from_file = forget(as_file(&spliced, parse_rule));
            assert!(from_file.is_err());
            assert_eq!(from_file, forget(as_flags(&spliced, parse_rule)));
        }
    }

    #[test]
    fn the_generated_files_are_not_all_empty() {
        let mut noise = Noise(0x5EED);
        let text = file(&mut noise, &SHAPES, 64);
        let rules: Vec<Rule> = as_file(&text, parse_rule).unwrap_or_default();
        assert!(rules.len() > 20);
    }

    #[test]
    fn a_file_without_a_final_newline_still_equals_its_flags() {
        let text = "* caveman\n# note\nSPEC.md box";
        let from_file = forget(as_file(text, parse_rule));
        assert_eq!(from_file, forget(as_flags(text, parse_rule)));
        assert_eq!(from_file.unwrap_or_default().len(), 2);
    }

    #[test]
    fn a_file_of_only_comments_equals_its_flags() {
        let text = "# one\n\n#two\n   \n";
        let from_file: Result<Vec<Rule>, String> = as_file(text, parse_rule);
        assert_eq!(from_file.unwrap_or_default().len(), 0);
        let from_flags = as_flags(text, parse_rule).unwrap_or_default();
        assert_eq!(from_flags.len(), 0);
    }

    #[test]
    fn the_two_failures_differ_only_in_their_origin() {
        let from_file = parse_file(BAD, path(), parse_rule).err();
        let from_flag = parse_flag(BAD, 0, parse_rule).err();
        let file_message = from_file.map(|failure| failure.message);
        let flag_message = from_flag.map(|failure| failure.message);
        assert_eq!(file_message, flag_message);
        assert_eq!(file_message, Some("unknown level `loud`".to_string()));
    }
}
