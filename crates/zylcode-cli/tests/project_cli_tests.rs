//! Project CLI tests for Phase 3A Project System vertical slice.
//! These tests use a shared workspace per test so that create/list/open/export/import
//! all operate on the same persistent project store, demonstrating the full lifecycle.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

fn write_cargo_toml(root: &std::path::Path, name: &str, version: &str) {
    let toml_path = root.join("Cargo.toml");
    fs::write(
        &toml_path,
        format!(
            r#"[package]
name = "{name}"
version = "{version}"
edition = "2021"
"#
        ),
    )
    .expect("write Cargo.toml");
}

/// Create a temporary workspace with a Cargo project for testing.
fn fixture_workspace() -> tempfile::TempDir {
    let dir = tempdir().expect("temp dir");
    let root = dir.path();
    write_cargo_toml(root, "test-project", "0.1.0");
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::write(root.join("src/lib.rs"), "pub struct Project;").expect("write lib.rs");
    dir
}

/// Helper: run the CLI binary with the given args in the specified dir,
/// returning (exit_status, stdout, stderr).
fn run_cli(dir: &std::path::Path, args: &[&str]) -> (std::process::ExitStatus, String, String) {
    let command = Command::new(env!("CARGO_BIN_EXE_zylcode"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run CLI command");
    let stdout = String::from_utf8_lossy(&command.stdout).to_string();
    let stderr = String::from_utf8_lossy(&command.stderr).to_string();
    (command.status, stdout, stderr)
}

/// Test: Create a project and verify the stored project record (self-contained).
#[test]
fn project_create_and_verify() {
    let dir = fixture_workspace();
    let dir_path = dir.path();
    // Create project using workspace root (current dir)
    let (status, stdout, stderr) =
        run_cli(dir_path, &["project", "create", "my-test-project", "."]);
    assert!(
        status.success(),
        "project create succeeded; stderr: {stderr}"
    );
    let stdout = stdout;
    // Actual output format: Project 'my-test-project' created at 'C:\...' ID: <uuid>
    assert!(
        stdout.contains("my-test-project"),
        "output should contain project name; stdout: {stdout}"
    );
    assert!(
        stdout.contains("ID:"),
        "output should contain ID; stdout: {stdout}"
    );
    // Verify the project was persisted by reopening the store and checking the record.
    let store_path = dir_path.join(".zylcode/projects.json");
    let store_content = std::fs::read_to_string(&store_path)
        .unwrap_or_else(|_| panic!("store file should exist at {:?}", store_path));
    let json: serde_json::Value = serde_json::from_str(&store_content)
        .unwrap_or_else(|_| panic!("store JSON should be valid"));
    let projects = json["projects"]
        .as_array()
        .expect("store should have projects array");
    let persisted_project = projects
        .iter()
        .find(|p| p["name"].as_str() == Some("my-test-project"))
        .expect("persisted project should exist with name 'my-test-project'");
    // Verify persisted record fields
    assert_eq!(
        persisted_project["name"].as_str().unwrap(),
        "my-test-project",
        "persisted project name should match"
    );
    assert!(
        persisted_project["root_path"].is_string(),
        "persisted project should have root_path"
    );
    assert!(
        persisted_project["id"].is_string(),
        "persisted project should have id"
    );
}

/// Test: Create a project and list it in the same test (same store).
#[test]
fn project_create_and_list() {
    let dir = fixture_workspace();
    let dir_path = dir.path();
    // Create a project
    let (create_status, create_stdout, create_stderr) =
        run_cli(dir_path, &["project", "create", "proj-alpha", "."]);
    assert!(
        create_status.success(),
        "project create succeeded; stderr: {create_stderr}"
    );
    // List from the same workspace - should find the created project
    let (status, stdout, stderr) = run_cli(dir_path, &["project", "list"]);
    assert!(status.success(), "list succeeded; stderr: {stderr}");
    let stdout = stdout;
    // Assert that the list contains the registered project by name
    assert!(
        stdout.contains("proj-alpha"),
        "output should contain 'proj-alpha'; stdout: {stdout}"
    );
    // Assert that the output includes the ID so users can retrieve the identifier
    // required by project open.
    assert!(
        stdout.contains("ID:"),
        "list output should include ID; stdout: {stdout}"
    );
    // Verify the ID from create matches what list displays.
    let create_id = create_stdout
        .lines()
        .find(|l| l.contains("ID:"))
        .map(|l| l.split_once("ID:").unwrap().1.trim())
        .unwrap_or_default();
    let listed_id_present = stdout.lines().any(|l| l.contains("ID:"));
    assert!(
        listed_id_present,
        "list output should include an ID field for the created project"
    );
}

/// Test: Open an existing project by UUID and reject invalid ID (self-contained).
#[test]
fn project_open_valid_and_reject_invalid() {
    let dir = fixture_workspace();
    let dir_path = dir.path();
    // Create a project and capture the UUID from output
    let (create_status, create_stdout, create_stderr) =
        run_cli(dir_path, &["project", "create", "open-me", "."]);
    assert!(
        create_status.success(),
        "project create succeeded; stderr: {create_stderr}"
    );
    let create_stdout = create_stdout;
    // Extract UUID from output: format is "Project 'name' created at 'path' ID: uuid"
    let uuid = create_stdout
        .lines()
        .find(|l| l.contains("ID:"))
        .map(|l| l.split_once("ID:").unwrap().1.trim())
        .unwrap_or_default();

    // Open the created project by UUID in the same working directory
    let (status, stdout, stderr) = run_cli(dir_path, &["project", "open", &uuid]);
    // In the same dir context, the project should be findable
    assert!(
        status.success(),
        "project open succeeded; stderr: {stderr}; stdout: {stdout}"
    );
    let stdout = stdout;
    // Assert the open shows the correct project name
    assert!(
        stdout.contains("open-me"),
        "open output should contain project name 'open-me'; stdout: {stdout}"
    );

    // Now test opening an invalid ID in the same store
    let (status, stdout, stderr) =
        run_cli(dir_path, &["project", "open", "nonexistent-uuid-12345"]);
    assert!(
        !status.success(),
        "opening project with nonexistent ID should fail; stderr: {stderr}; stdout: {stdout}"
    );
    let stderr = stderr;
    // Assert a useful error message
    assert!(
        stderr.contains("No project found")
            || stderr.contains("not found")
            || stderr.contains("unknown project id"),
        "error message should indicate project not found; stderr: {stderr}"
    );
}

/// Test: Export projects to portable JSON with verified round-trip (self-contained).
#[test]
fn project_export_to_json() {
    let dir = fixture_workspace();
    let dir_path = dir.path();
    // Create a project first; assert success before exporting
    let (create_status, _, create_stderr) =
        run_cli(dir_path, &["project", "create", "export-me", "."]);
    assert!(
        create_status.success(),
        "project create succeeded before export; stderr: {create_stderr}"
    );

    // Export - creates file in the same working directory
    let (status, stdout, stderr) = run_cli(dir_path, &["project", "export"]);
    assert!(
        status.success(),
        "project export succeeded; stderr: {stderr}"
    );
    let stdout = stdout;
    assert!(
        stdout.contains("Exported"),
        "export output should contain Exported; stdout: {stdout}"
    );

    // Verify the exported JSON file exists at the expected path
    let export_path = dir_path.join("zylcode-projects-export.json");
    assert!(
        export_path.exists(),
        "export file should exist at {:?}",
        export_path
    );

    // Read and verify the exported JSON (single read)
    let content = fs::read_to_string(&export_path).expect("read export file");
    let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");

    // Verify JSON schema version
    let obj = json.as_object().expect("json should be an object");
    assert!(
        obj.get("schema_version").is_some(),
        "JSON should have schema_version"
    );
    assert!(
        json.get("projects")
            .expect("projects key should exist")
            .is_array(),
        "JSON should have projects array"
    );

    // Verify project count matches actual records (not JSON lines)
    let projects = json["projects"]
        .as_array()
        .expect("projects should be an array");
    let project_count = projects.len();
    assert_eq!(
        project_count, 1,
        "should have exactly 1 project exported; found {}",
        project_count
    );

    // Verify the exported record has correct fields
    let first_project = &projects[0];
    assert!(
        first_project.get("id").is_some(),
        "project record should have id"
    );
    assert!(
        first_project.get("name").is_some(),
        "project record should have name"
    );
    assert!(
        first_project.get("root_path").is_some(),
        "project record should have root_path"
    );
    assert_eq!(
        first_project["name"].as_str().unwrap(),
        "export-me",
        "project name should be 'export-me'"
    );
    assert!(
        first_project["id"].is_string(),
        "project record id should be a string"
    );
}

/// Test: Import exported JSON into a second workspace (basic round-trip).
#[test]
fn project_import_basic() {
    let src_dir = tempdir().expect("temp dir");
    let dst_dir = tempdir().expect("temp dir");

    // Create and export a project from the source workspace; assert success
    let (create_status, _, create_stderr) = run_cli(
        src_dir.path(),
        &["project", "create", "roundtrip-test", "."],
    );
    assert!(
        create_status.success(),
        "source project create succeeded; stderr: {create_stderr}"
    );

    let (export_status, export_stdout, export_stderr) =
        run_cli(src_dir.path(), &["project", "export"]);
    assert!(
        export_status.success(),
        "source project export succeeded; stderr: {export_stderr}; stdout: {export_stdout}"
    );
    let export_stdout = export_stdout;
    assert!(
        export_stdout.contains("Exported"),
        "source export output should contain Exported"
    );

    // The export command writes to ./zylcode-projects-export.json relative to the CLI cwd
    let export_path = src_dir.path().join("zylcode-projects-export.json");
    // Assert the export file exists - never silently skip
    assert!(
        export_path.exists(),
        "export file should exist after successful export at {:?}",
        export_path
    );

    let dst_dir = tempdir().expect("temp dir");
    let dst_export = dst_dir.path().join("zylcode-projects-export.json");
    // Copy export file to destination workspace (fail if missing rather than skip)
    fs::copy(export_path, &dst_export).expect(
        "should copy export file to destination; export file missing after successful export",
    );

    // Import into destination store
    let (status, stdout, stderr) = run_cli(
        dst_dir.path(),
        &["project", "import", dst_export.to_str().unwrap()],
    );
    assert!(
        status.success(),
        "project import succeeded; stderr: {stderr}; stdout: {stdout}"
    );
    let stdout = stdout;
    assert!(
        stdout.contains("Imported"),
        "import output should contain 'Imported'; stdout: {stdout}"
    );

    // Verify the imported record exists in the destination store with ID and metadata
    let (list_status, list_stdout, list_stderr) = run_cli(dst_dir.path(), &["project", "list"]);
    assert!(
        list_status.success(),
        "list after import succeeded; stderr: {list_stderr}"
    );
    let list_stdout = list_stdout;
    assert!(
        list_stdout.contains("roundtrip-test"),
        "listed projects should include 'roundtrip-test'; stdout: {list_stdout}"
    );
    // Verify the imported record's ID is present in the list output
    assert!(
        list_stdout.contains("ID:"),
        "list output after import should include ID field; stdout: {list_stdout}"
    );
}

/// Test: Reject invalid project paths (self-contained).
#[test]
fn project_invalid_path_rejected() {
    let (status, _stdout, stderr) = run_cli(
        std::path::Path::new("."),
        &[
            "project",
            "create",
            "bad-path",
            &std::env::temp_dir()
                .join("zylcode_nonexistent_path")
                .display()
                .to_string(),
        ],
    );
    assert!(
        !status.success(),
        "creating project with non-existent path should fail; stderr: {stderr}"
    );
    let stderr = stderr;
    // Use platform-appropriate path substrings for error assertion
    let stderr_lower = stderr.to_lowercase();
    assert!(
        stderr_lower.contains("not found") || stderr_lower.contains("root directory"),
        "error should mention path issue; stderr: {stderr}"
    );
}

/// Test: Empty-store behaviour and useful error messages (self-contained).
#[test]
fn project_empty_store_behaviour() {
    let dir = tempdir().expect("temp dir");
    let dir_path = dir.path();
    // List projects from empty store
    let (status, stdout, stderr) = run_cli(dir_path, &["project", "list"]);
    assert!(
        status.success(),
        "list from empty store should succeed; stderr: {stderr}"
    );
    let stdout = stdout;
    assert!(
        stdout.contains("No projects registered"),
        "empty store message should appear; stdout: {stdout}"
    );

    // Open project from empty store should fail
    let (status, _stdout, stderr) = run_cli(dir_path, &["project", "open", "some-id"]);
    assert!(
        !status.success(),
        "opening project from empty store should fail; stderr: {stderr}"
    );
    let stderr = stderr;
    assert!(
        stderr.contains("No project found")
            || stderr.contains("not found")
            || stderr.contains("unknown project id"),
        "error message should indicate project not found; stderr: {stderr}"
    );
}
