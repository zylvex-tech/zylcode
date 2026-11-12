//! The D1 acceptance contract asks for a CLI demonstration on a real
//! repository with the coverage figure in the output. These tests hold the
//! command to that contract: both denominators printed, a non-zero exit for an
//! unusable invocation, and no fabricated figures when there is nothing to
//! measure.

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture directory");
    }
    fs::write(path, contents).expect("write fixture file");
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\n\n[package]\nname = \"semantic-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        root,
        "src/lib.rs",
        "mod thing;\n\nuse thing::Thing;\n\npub fn f() -> Thing {\n    Thing\n}\n",
    );
    write(root, "src/thing.rs", "pub struct Thing;\n");
    dir
}

#[test]
fn the_command_reports_both_coverage_figures() {
    let dir = fixture();
    let out = Command::new(env!("CARGO_BIN_EXE_zylcode"))
        .args(["repo-semantic", "--path"])
        .arg(dir.path())
        .output()
        .expect("run the CLI");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("entry-level coverage:"), "output: {stdout}");
    assert!(
        stdout.contains("declaration-level coverage:"),
        "both denominators are required, output: {stdout}"
    );
    assert!(stdout.contains("import statements:"), "output: {stdout}");
    assert!(
        stdout.contains("partial"),
        "partial-parse counts are required, output: {stdout}"
    );
}

#[test]
fn a_missing_repository_is_a_non_zero_exit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("does-not-exist");
    let out = Command::new(env!("CARGO_BIN_EXE_zylcode"))
        .args(["repo-semantic", "--path"])
        .arg(&missing)
        .output()
        .expect("run the CLI");

    assert!(
        !out.status.success(),
        "an unresolvable path must not exit successfully"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cannot resolve repository"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn an_empty_directory_is_not_reported_as_zero_percent_success() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = Command::new(env!("CARGO_BIN_EXE_zylcode"))
        .args(["repo-semantic", "--path"])
        .arg(dir.path())
        .output()
        .expect("run the CLI");

    assert!(
        !out.status.success(),
        "an empty tree must not be reported as a successful 0% run"
    );
}
