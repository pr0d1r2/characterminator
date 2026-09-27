//! Writing a report to stdout (V47).
//!
//! `println!` PANICS when stdout is closed, and stdout closes early every
//! time a reader has seen enough: `ctrm check | head`. Found by dogfood
//! wave 1 (B3), where the first thing anyone does with 4,129 violations is
//! pipe them into something that stops reading.

use super::Outcome;
use std::io::{ErrorKind, Write};

/// Print `text` and keep `verdict`, or turn a failed write into a usage
/// exit that names what failed.
///
/// A closed pipe KEEPS the verdict: the reader stopped listening, which
/// changes nothing about whether the tree was clean, so a `set -o pipefail`
/// caller still learns the answer.
pub(super) fn shown(text: &str, verdict: Outcome) -> Outcome {
    match written(&mut std::io::stdout().lock(), text) {
        Ok(()) => verdict,
        Err(message) => {
            eprintln!("ctrm: {message}");
            Outcome::Usage
        }
    }
}

/// One report to `out`. Only a broken pipe is forgiven: a full disk behind
/// `> report.txt` is output that was LOST, and saying nothing about it
/// would be a report that silently stopped halfway.
fn written(out: &mut impl Write, text: &str) -> Result<(), String> {
    match writeln!(out, "{text}").and_then(|()| out.flush()) {
        Err(e) if e.kind() != ErrorKind::BrokenPipe => {
            Err(format!("stdout: {e}"))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::written;
    use std::io::{Error, ErrorKind, Write};

    /// A writer that fails every write with one kind of error.
    struct Failing(ErrorKind);

    impl Write for Failing {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(Error::from(self.0))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_report_is_written_with_its_newline() {
        let mut out = Vec::new();
        assert!(written(&mut out, "a.md:1:1 U+2014 ascii").is_ok());
        assert_eq!(out, b"a.md:1:1 U+2014 ascii\n");
    }

    /// B3: the panic `println!` raised here was the whole bug.
    #[test]
    fn a_reader_that_stopped_listening_is_not_an_error() {
        let mut out = Failing(ErrorKind::BrokenPipe);
        assert!(written(&mut out, "x").is_ok());
    }

    #[test]
    fn any_other_failed_write_is_named() {
        let mut out = Failing(ErrorKind::StorageFull);
        let said = written(&mut out, "x").err().unwrap_or_default();
        assert!(said.starts_with("stdout: "), "{said}");
    }
}
