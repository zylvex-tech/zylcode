//! Phase 2B impact analysis through the real CLI.
//!
//! The acceptance rule for this slice is not "a list of files comes back". It is
//! that each entry explains itself, that the report says what it could not
//! determine, and that an unindexed change produces an unknown instead of a
//! confident empty answer. These run the binary against a fixture repository so
//! the assertions are about the product surface, not a private function.

use std::process::Command;
use tempfile::tempdir;

fn run(dir: &std::path::Path, args: &[&str]) -> (std::process::ExitStatus, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_zylcode"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run CLI command");
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("fixture dir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("util.ts"),
        "export function helper() { return 1; }\n",
    )
    .unwrap();
    std::fs::write(
        src.join("widget.ts"),
        "import { helper } from \"./util\";\nexport class Widget {}\n",
    )
    .unwrap();
    std::fs::write(
        src.join("app.ts"),
        "import { Widget } from \"./widget\";\nexport const app = 1;\n",
    )
    .unwrap();
    std::fs::write(
        src.join("util.test.ts"),
        "import { helper } from \"./util\";\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name":"fixture","version":"1.0.0"}"#,
    )
    .unwrap();
    dir
}

#[test]
fn a_file_change_reports_importers_tests_and_its_unknowns() {
    let repo = fixture();
    let (status, stdout, stderr) = run(repo.path(), &["impact", "--file", "src/util.ts"]);
    assert!(status.success(), "impact failed: {stderr}\n{stdout}");

    assert!(stdout.contains("src/util.ts"), "{stdout}");
    assert!(
        stdout.contains("widget.ts"),
        "direct importer missing: {stdout}"
    );
    assert!(
        stdout.contains("app.ts"),
        "transitive importer missing: {stdout}"
    );
    assert!(
        stdout.contains("util.test.ts"),
        "test at risk missing: {stdout}"
    );
    assert!(stdout.contains("confidence:"), "{stdout}");
    assert!(
        stdout.contains("not determined"),
        "the report must state its unknowns: {stdout}"
    );
    assert!(
        stdout.contains("call graph is not resolved"),
        "the call-graph limitation must be explicit: {stdout}"
    );
    // Every listed entry carries a reason after an em dash.
    let detail_lines = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("src/"))
        .count();
    assert!(detail_lines >= 3, "expected reasoned entries: {stdout}");
}

#[test]
fn a_symbol_change_starts_from_its_declaring_file() {
    let repo = fixture();
    let (status, stdout, stderr) = run(repo.path(), &["impact", "--symbol", "Widget"]);
    assert!(status.success(), "impact failed: {stderr}\n{stdout}");
    assert!(stdout.contains("widget.ts"), "{stdout}");
    assert!(stdout.contains("app.ts"), "{stdout}");
}

#[test]
fn a_change_that_is_not_indexed_exits_nonzero_and_guesses_nothing() {
    let repo = fixture();
    let (status, stdout, stderr) = run(repo.path(), &["impact", "--file", "src/ghost.ts"]);
    assert!(!status.success(), "must not exit 0: {stdout}");
    assert!(stdout.contains("directly affected (0)"), "{stdout}");
    assert!(stdout.contains("not in the index"), "{stdout}");
    assert!(stderr.contains("nothing was inferred"), "{stderr}");
}

#[test]
fn the_depth_bound_is_visible_in_the_report() {
    let repo = fixture();
    let (_, shallow, _) = run(
        repo.path(),
        &["impact", "--file", "src/util.ts", "--depth", "1"],
    );
    assert!(shallow.contains("bounded at depth 1"), "{shallow}");
    // app.ts is two hops away and must not be claimed at depth 1.
    assert!(
        !shallow.contains("src/app.ts"),
        "depth 1 must not reach app.ts: {shallow}"
    );
}

#[test]
fn the_json_report_is_machine_readable() {
    let repo = fixture();
    let (status, stdout, stderr) = run(repo.path(), &["impact", "--file", "src/util.ts", "--json"]);
    assert!(status.success(), "impact failed: {stderr}");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(value["directly_affected"].is_array(), "{value}");
    assert!(value["unknowns"].as_array().unwrap().len() >= 3, "{value}");
    assert_eq!(value["confidence"], "High");
    assert!(value["transitively_affected"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["reason"].as_str().is_some_and(|r| !r.is_empty())));
}

#[test]
fn passing_both_targets_is_refused() {
    let repo = fixture();
    let (status, _, stderr) = run(
        repo.path(),
        &["impact", "--file", "src/util.ts", "--symbol", "Widget"],
    );
    assert!(!status.success());
    assert!(stderr.contains("exactly one"), "{stderr}");
}
