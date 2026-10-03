//! `characterminator-dev` -- the binary. A shim over the library, where a
//! test can reach every verb without launching a process (`dev:C`).

use std::process::ExitCode;

fn main() -> ExitCode {
    characterminator_dev::main()
}
