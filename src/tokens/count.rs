//! What a figure counts, and how it was arrived at (V10).
//!
//! Two tiers, both `itok`'s: the cheap proxy over a file's bytes, and
//! o200k through a real tokenizer. The method rides along with every
//! number, so nothing downstream has to remember which tier ran.
//!
//! An unreadable file yields an error, never a zero. Zero is what an
//! empty file costs; spending it on "could not read" would quietly
//! understate every total that includes the file.

use crate::tokens::{Count, Error, Method};
use std::path::Path;

/// Count one file on `method`.
///
/// # Errors
/// When the file cannot be read.
pub fn of_file(path: &Path, method: Method) -> Result<Count, Error> {
    match method {
        Method::Estimate => estimated(path),
        Method::Bpe => encoded(path),
    }
}

/// Count text already in memory -- the "after" side of a rewrite, which
/// exists nowhere on disk to be read back.
#[must_use]
pub fn of_text(text: &str, method: Method) -> Count {
    let tokens = match method {
        Method::Estimate => itok::estimate::dummy(width(text)),
        Method::Bpe => itok::bpe::count(text),
    };
    Count { tokens, method }
}

/// The cheap tier reads a SIZE, not the text: itok's own bytes/4.
fn estimated(path: &Path) -> Result<Count, Error> {
    let bytes = itok::walk::bytes(path).ok_or_else(|| Error {
        path: path.to_path_buf(),
        reason: "cannot be read".to_owned(),
    })?;
    Ok(Count {
        tokens: itok::estimate::dummy(bytes),
        method: Method::Estimate,
    })
}

/// The real-tokenizer tier needs the text itself. itok encodes a `&str`,
/// so getting the string off disk is this crate's half of the job.
fn encoded(path: &Path) -> Result<Count, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| Error {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    Ok(Count {
        tokens: itok::bpe::count(&text),
        method: Method::Bpe,
    })
}

/// A text's byte length, saturating. The cheap tier takes a `u64`, and a
/// cast that wrapped would report a smaller file than the one in hand.
fn width(text: &str) -> u64 {
    u64::try_from(text.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const MISSING: &str = "/no/such/characterminator/file";

    /// V10's floor: an unreadable file has NO count, on either tier. A
    /// zero here would be indistinguishable from an empty file.
    #[test]
    fn an_unreadable_file_is_an_error_not_a_zero() {
        let path = Path::new(MISSING);
        assert!(of_file(path, Method::Estimate).is_err());
        assert!(of_file(path, Method::Bpe).is_err());
    }

    /// And the failure says WHICH file, so a caller can report it rather
    /// than drop the row.
    #[test]
    fn the_failure_names_the_file() {
        let err = of_file(Path::new(MISSING), Method::Bpe).err();
        assert_eq!(err.map(|e| e.path), Some(PathBuf::from(MISSING)));
    }

    /// A count carries the method it was asked for. Without this the two
    /// tiers become one number wearing either label.
    #[test]
    fn a_count_carries_its_method() {
        assert_eq!(of_text("hi", Method::Bpe).method, Method::Bpe);
        let cheap = of_text("hi", Method::Estimate);
        assert_eq!(cheap.method, Method::Estimate);
    }

    /// A readable file counts on both tiers. The NUMBERS are itok's and
    /// are not asserted here -- only that a real file yields a count.
    #[test]
    fn a_readable_file_counts_on_both_tiers() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = root.join("Cargo.toml");
        assert!(of_file(&path, Method::Estimate).is_ok());
        assert!(of_file(&path, Method::Bpe).is_ok());
    }
}
