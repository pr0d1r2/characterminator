//! `ctrm` -- the binary. Dispatch only; a verb's logic belongs to its node
//! module (`src:V38`).
//!
//! No verb is implemented yet, and this says so rather than exiting 0: a
//! tool that answers "fine" before it can measure anything is the failure
//! mode the whole spec is written against.

use std::process::ExitCode;

fn version() -> ExitCode {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}

/// Exit 2 is the usage code the interface section fixes, and the message
/// names the surface that is planned rather than pretending to offer it.
fn usage() -> ExitCode {
    eprintln!(
        "ctrm: no verb is implemented yet. SPEC.md names the planned \
         surface: check, fix, stats, explain, sets, guard."
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => version(),
        _ => usage(),
    }
}
