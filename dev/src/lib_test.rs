//! The tests of `lib.rs`: `run` against a fixture repository built in the
//! temp dir, with `cargo tree`'s answer recorded (`dev:C`). Never this
//! repository's own README.

use std::path::PathBuf;

use super::{External, run};

const MANIFEST: &str = r#"[package]
name = "demo"
edition = "2024"
rust-version = "1.95"
license = "MIT"
repository = "https://github.com/me/demo"

[dependencies]
itok = "0.3"

[workspace.lints.rust]
unsafe_code = "forbid"
"#;

const TREE: &str = "demo v0.1.0 (/x)|MIT\nitok v0.3.1|MIT\n\
                    memchr v2.8.3|Unlicense OR MIT\nitok v0.3.1|MIT (*)\n";

const README: &str =
    "# demo\n<!-- BEGIN badges -->\nold\n<!-- END badges -->\n";
const NOTICES: &str =
    "# n\n<!-- BEGIN closure -->\n<!-- END closure -->\nend\n";

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let id = std::process::id();
        let root = std::env::temp_dir().join(format!("ctrm-dev-{id}-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        let fixture = Self { root };
        fixture.write("Cargo.toml", MANIFEST);
        fixture.write(".coverage", "key abc\nlines 97.53\n");
        fixture.write("README.md", README);
        fixture.write("docs/THIRD-PARTY-NOTICES.md", NOTICES);
        fixture.write("tree.txt", TREE);
        fixture
    }

    fn write(&self, file: &str, text: &str) {
        let path = self.root.join(file);
        let dir = path.parent().map(PathBuf::from).unwrap_or_default();
        assert!(std::fs::create_dir_all(dir).is_ok());
        assert!(std::fs::write(path, text).is_ok());
    }

    fn read(&self, file: &str) -> String {
        std::fs::read_to_string(self.root.join(file)).unwrap_or_default()
    }

    fn run(&self, args: &[&str]) -> (u8, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let ext = External {
            cargo_tree: Some(self.root.join("tree.txt")),
        };
        let mut err = Vec::new();
        let code = run(&args, Some(&self.root), &ext, &mut err);
        (code, String::from_utf8_lossy(&err).into_owned())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn readme_writes_every_badge_from_its_owner() {
    let f = Fixture::new("readme");
    assert_eq!(f.run(&["readme"]), (0, String::new()));
    let readme = f.read("README.md");
    assert!(readme.contains("[![coverage 97.5%]"), "{readme}");
    assert!(readme.contains("[![runtime closure 2]"), "{readme}");
    assert!(readme.contains("[![direct dependencies 1]"), "{readme}");
    assert!(readme.ends_with("<!-- END badges -->\n"), "{readme}");
}

#[test]
fn a_second_run_changes_nothing() {
    let f = Fixture::new("idempotent");
    assert_eq!(f.run(&["--fix"]).0, 0);
    let once = (f.read("README.md"), f.read("docs/THIRD-PARTY-NOTICES.md"));
    assert_eq!(f.run(&["--check"]), (0, String::new()));
    assert_eq!(f.run(&["--fix"]).0, 0);
    let twice = (f.read("README.md"), f.read("docs/THIRD-PARTY-NOTICES.md"));
    assert_eq!(once, twice);
}

#[test]
fn check_reports_a_stale_block_and_writes_nothing() {
    let f = Fixture::new("stale");
    let (code, err) = f.run(&["readme", "--check"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("stale: readme (`badges` in README.md)"),
        "{err}"
    );
    assert!(err.contains("  have: old"), "{err}");
    assert_eq!(f.read("README.md"), README);
}

#[test]
fn notices_writes_the_closure_without_the_package_itself() {
    let f = Fixture::new("notices");
    assert_eq!(f.run(&["notices"]).0, 0);
    let doc = f.read("docs/THIRD-PARTY-NOTICES.md");
    assert!(doc.contains("## The closure: 2 packages"), "{doc}");
    assert!(doc.contains("1 direct, 1 transitive"), "{doc}");
    assert!(!doc.contains("`demo`"), "{doc}");
    assert!(doc.ends_with("<!-- END closure -->\nend\n"), "{doc}");
}

#[test]
fn an_unrelated_change_reads_no_owner() {
    let f = Fixture::new("unrelated");
    f.write("Cargo.toml", "");
    assert_eq!(f.run(&["--check", "src/main.rs"]), (0, String::new()));
}

#[test]
fn an_absolute_path_selects_like_a_relative_one() {
    let f = Fixture::new("absolute");
    let coverage = f.root.join(".coverage").to_string_lossy().into_owned();
    let (code, err) = f.run(&["--check", &coverage]);
    assert_eq!(code, 1);
    assert!(err.contains("stale: readme"), "{err}");
    assert!(!err.contains("stale: notices"), "{err}");
}

#[test]
fn a_missing_owner_value_is_an_error_naming_it() {
    let f = Fixture::new("owner");
    f.write(
        "Cargo.toml",
        &MANIFEST.replace("rust-version = \"1.95\"\n", ""),
    );
    let (code, err) = f.run(&["readme"]);
    assert_eq!(code, 1);
    assert!(err.contains("Cargo.toml [package] rust-version has no value"));
    assert_eq!(f.read("README.md"), README);
}

#[test]
fn a_tree_that_omits_the_package_is_refused() {
    let f = Fixture::new("tree");
    f.write("tree.txt", "itok v0.3.1|MIT\n");
    let (code, err) = f.run(&["readme"]);
    assert_eq!(code, 1);
    assert!(err.contains("did not list `demo` itself"), "{err}");
}

#[test]
fn missing_markers_are_an_error_and_nothing_is_written() {
    let f = Fixture::new("markers");
    f.write("README.md", "# no markers\n");
    let (code, err) = f.run(&["readme"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("README.md: no `<!-- BEGIN badges -->`"),
        "{err}"
    );
}

#[test]
fn a_missing_file_is_an_error_naming_it() {
    let f = Fixture::new("file");
    let _ = std::fs::remove_file(f.root.join(".coverage"));
    let (code, err) = f.run(&["notices"]);
    assert_eq!(code, 1);
    assert!(err.starts_with("characterminator-dev: .coverage:"), "{err}");
}

#[test]
fn bad_arguments_are_usage() {
    let f = Fixture::new("usage");
    for args in [
        &[][..],
        &["nope"],
        &["readme", "--bogus"],
        &["--check", "-x"],
    ] {
        let (code, err) = f.run(args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.starts_with("usage:"), "{err}");
    }
    assert_eq!(f.run(&["--help"]).0, 0);
}

#[test]
fn outside_a_repository_is_usage() {
    let mut err = Vec::new();
    let args = ["readme".to_owned()];
    assert_eq!(run(&args, None, &External::default(), &mut err), 2);
}
