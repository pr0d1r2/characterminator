//! `ctrm` -- the binary. A shim over `cli::run` (`src:V38`).
//!
//! Dispatch, usage and exit codes live in the cli node, where a test can
//! reach them: an entry point that can only be exercised by launching a
//! process is one whose contracts go unchecked.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    characterminator::cli::run(&args)
}
