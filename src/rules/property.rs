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

/// The generator, shared: `src/cli/explain:V32`'s round trip draws its
/// files from the same corpus and the same sequence, so "a file of every
/// shape" means one thing wherever the property is claimed.
#[cfg(test)]
pub(crate) mod corpus {
    /// The line shapes a generated file is built from. Comments, blanks
    /// and a whitespace-only line are IN the corpus on purpose: they are
    /// the lines that yield no entry, and the property has to survive
    /// them on both sides.
    pub(crate) const SHAPES: [&str; 12] = [
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

    /// A fixed-seed sequence. `wrapping_*` throughout: the arithmetic is
    /// meant to wrap, and saying so is what keeps it from being an
    /// overflow.
    pub(crate) struct Noise(pub(crate) u64);

    impl Noise {
        pub(crate) fn next(&mut self) -> usize {
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
    pub(crate) fn file(
        noise: &mut Noise,
        shapes: &[&str],
        count: usize,
    ) -> String {
        let mut text = String::new();
        for _ in 0..count {
            text.push_str(noise.pick(shapes));
            text.push('\n');
        }
        text
    }
}

#[cfg(test)]
#[path = "property_test.rs"]
mod tests;
