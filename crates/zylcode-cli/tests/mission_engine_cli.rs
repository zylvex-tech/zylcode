//! Mission Engine (Phase 3B) CLI tests.
//!
//! The benchmark in the roadmap is concrete: kill the process mid-`EXECUTING`,
//! restart, resume, and finish with a full evidence bundle — and a mission with
//! one unsatisfied criterion must never report COMPLETE. These are real process
//! boundaries: each step below is a separate `zylcode` invocation, so nothing
//! survives in memory between them.

use std::process::Command;

use tempfile::tempdir;

fn run_cli(dir: &std::path::Path, args: &[&str]) -> (std::process::ExitStatus, String, String) {
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

#[test]
fn kill_mid_executing_then_resume_completes_with_cited_evidence() {
    let workdir = tempdir().expect("temp dir");
    let w = workdir.path().to_string_lossy().to_string();
    let w = w.as_str();

    // 1. Create: objective + two criteria + three steps.
    let (status, stdout, stderr) = run_cli(
        workdir.path(),
        &[
            "missions",
            "create",
            "--workdir",
            w,
            "--objective",
            "ship the mission engine",
            "--criterion",
            "the engine tests pass",
            "--criterion",
            "the state machine is documented",
            "--step",
            "write the engine",
            "--step",
            "write the tests",
            "--step",
            "write the docs",
        ],
    );
    assert!(status.success(), "create failed: {stderr}");
    assert!(stdout.contains("CREATED"), "{stdout}");

    // 2. Resume without approval: nothing executes, and the gate is visible.
    let (status, stdout, _) = run_cli(workdir.path(), &["missions", "resume", "--workdir", w]);
    assert!(status.success(), "{stdout}");
    assert!(stdout.contains("AWAITING_APPROVAL"), "{stdout}");

    let (_, status_out, _) = run_cli(workdir.path(), &["missions", "status", "--workdir", w]);
    assert!(
        status_out.contains("phase: AWAITING_APPROVAL"),
        "{status_out}"
    );
    assert!(status_out.contains("steps: 0/3"), "{status_out}");

    // 3. Approve, but complete only one step: the process exits 9 as if killed.
    let (status, stdout, _) = run_cli(
        workdir.path(),
        &[
            "missions",
            "resume",
            "--workdir",
            w,
            "--approve",
            "--crash-after-steps",
            "1",
        ],
    );
    assert_eq!(status.code(), Some(9), "expected interruption; {stdout}");
    assert!(
        stdout.contains("EXECUTING: 1 step(s) done, 2 remaining"),
        "{stdout}"
    );

    // 4. A fresh process sees the persisted cursor, not a restart from zero.
    let (_, status_out, _) = run_cli(workdir.path(), &["missions", "status", "--workdir", w]);
    assert!(status_out.contains("phase: EXECUTING"), "{status_out}");
    assert!(status_out.contains("steps: 1/3"), "{status_out}");

    // 5. Resume finishes execution but must stop at VERIFYING: neither
    //    criterion has a cited proof yet, so COMPLETE would be a lie.
    let (status, stdout, _) = run_cli(
        workdir.path(),
        &["missions", "resume", "--workdir", w, "--approve"],
    );
    assert_eq!(status.code(), Some(2), "expected 'not complete'; {stdout}");
    assert!(stdout.contains("not COMPLETE"), "{stdout}");
    assert!(!stdout.contains("COMPLETE:"), "{stdout}");

    let (_, status_out, _) = run_cli(workdir.path(), &["missions", "status", "--workdir", w]);
    assert!(status_out.contains("phase: VERIFYING"), "{status_out}");
    assert!(status_out.contains("steps: 3/3"), "{status_out}");
    assert_eq!(status_out.matches("UNSATISFIED").count(), 2, "{status_out}");

    // 6. Cite one proof: still not complete.
    let (status, stdout, stderr) = run_cli(
        workdir.path(),
        &[
            "missions",
            "cite",
            "--workdir",
            w,
            "--criterion",
            "c1",
            "--evidence",
            "test:mission_engine::complete_requires_a_citation_for_every_criterion",
            "--kind",
            "test",
            "--summary",
            "16 engine tests green",
        ],
    );
    assert!(status.success(), "cite failed: {stderr} {stdout}");
    let (status, stdout, _) = run_cli(
        workdir.path(),
        &["missions", "resume", "--workdir", w, "--approve"],
    );
    assert_eq!(status.code(), Some(2), "{stdout}");
    assert!(
        stdout.contains("c2"),
        "unsatisfied criterion must be named: {stdout}"
    );

    // 7. Cite the second proof, then the mission can honestly complete.
    let (status, _, stderr) = run_cli(
        workdir.path(),
        &[
            "missions",
            "cite",
            "--workdir",
            w,
            "--criterion",
            "c2",
            "--evidence",
            "docs/architecture/MISSION_ENGINE.md",
            "--kind",
            "file",
            "--summary",
            "states and completion rule documented",
        ],
    );
    assert!(status.success(), "cite failed: {stderr}");

    let (status, stdout, _) = run_cli(
        workdir.path(),
        &["missions", "resume", "--workdir", w, "--approve"],
    );
    assert!(status.success(), "{stdout}");
    assert!(stdout.contains("COMPLETE:"), "{stdout}");

    let (_, status_out, _) = run_cli(workdir.path(), &["missions", "status", "--workdir", w]);
    assert!(status_out.contains("phase: COMPLETE"), "{status_out}");
    assert_eq!(status_out.matches("UNSATISFIED").count(), 0, "{status_out}");

    // The journal and snapshot are both on disk for inspection.
    assert!(workdir
        .path()
        .join(".zylcode")
        .join("mission.journal.jsonl")
        .exists());
    assert!(workdir
        .path()
        .join(".zylcode")
        .join("mission.json")
        .exists());
}

#[test]
fn create_without_acceptance_criteria_is_refused() {
    let workdir = tempdir().expect("temp dir");
    let w = workdir.path().to_string_lossy().to_string();
    let (status, _, stderr) = run_cli(
        workdir.path(),
        &[
            "missions",
            "create",
            "--workdir",
            w.as_str(),
            "--objective",
            "something vague",
            "--step",
            "do it",
        ],
    );
    assert!(!status.success(), "must fail without criteria");
    assert!(
        stderr.contains("criterion") || stderr.contains("required"),
        "{stderr}"
    );
}

#[test]
fn cancellation_is_recorded_and_final() {
    let workdir = tempdir().expect("temp dir");
    let w = workdir.path().to_string_lossy().to_string();
    let (status, _, stderr) = run_cli(
        workdir.path(),
        &[
            "missions",
            "create",
            "--workdir",
            w.as_str(),
            "--objective",
            "abandon me",
            "--criterion",
            "never",
            "--step",
            "none",
        ],
    );
    assert!(status.success(), "{stderr}");

    let (status, stdout, stderr) = run_cli(
        workdir.path(),
        &[
            "missions",
            "cancel",
            "--workdir",
            w.as_str(),
            "--reason",
            "owner changed direction",
        ],
    );
    assert!(status.success(), "cancel failed: {stderr}");
    assert!(stdout.contains("CANCELLED"), "{stdout}");

    // Cancelling twice is refused: history is not rewritten.
    let (status, _, stderr) = run_cli(
        workdir.path(),
        &[
            "missions",
            "cancel",
            "--workdir",
            w.as_str(),
            "--reason",
            "again",
        ],
    );
    assert!(!status.success(), "second cancel must fail");
    assert!(stderr.contains("CANCELLED"), "{stderr}");
}
