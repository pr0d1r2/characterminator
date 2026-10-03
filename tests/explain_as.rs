//! `explain --as` refuses `--format` (`src/cli/explain:V32`), pinned at
//! the process boundary where the exit code is the contract.

use std::path::Path;
use std::process::Command;

/// `--as` writes text -- flags, data-file lines or a prompt -- so a json
/// or sarif form beside it is a usage error, exit 2, for every form.
#[test]
fn explain_as_refuses_any_format() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    for form in ["args", "lines", "prompt"] {
        for format in ["json", "sarif"] {
            let ran = Command::new(env!("CARGO_BIN_EXE_ctrm"))
                .args(["explain", "--as", form, "--format", format])
                .current_dir(here)
                .output();
            let code = ran.ok().and_then(|o| o.status.code());
            assert_eq!(code, Some(2), "--as {form} --format {format}");
        }
    }
}
