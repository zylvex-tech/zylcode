//! Mission queue with Build/Plan modes and a file-backed state store.
//!
//! # Why this exists
//!
//! The frontend had a mission composer that only talked to a simulated
//! stream. This module is the real thing: tasks submitted in `build` or
//! `plan` mode are appended to a queue on disk, the service drains the
//! queue by actually running Best-of-N verification (working-tree mode),
//! and every state transition is recorded — so the UI's activity feed is
//! the evidence ledger's truth, not a fiction.
//!
//! # Design notes
//!
//! Queue state lives in `<root>/.zylcode/missions.json` — human-readable,
//! survive restarts, and zero new dependencies. The file is rewritten
//! atomically (write temp + rename) on every mutation; concurrent access
//! is serialized through the hub's mutex. A `plan` mission records intent
//! only: it never runs code, by design — its "execution" is the recorded
//! plan the operator reads.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Where mission state is persisted.
pub fn missions_path(root: &Path) -> PathBuf {
    root.join(".zylcode").join("missions.json")
}

/// Execution mode of a queued mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissionMode {
    /// Run the real pipeline end-to-end.
    Build,
    /// Produce a plan without executing anything.
    Plan,
}

/// Lifecycle state of a mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissionState {
    Queued,
    Running,
    Done,
    Failed,
}

/// One queued mission and its outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: String,
    pub task: String,
    pub mode: MissionMode,
    pub state: MissionState,
    pub created_at: String,
    pub updated_at: String,
    /// For `done` build missions: the ledger session id best-of-n recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_session: Option<String>,
    /// Human-readable result summary (selection reason, plan text, or the
    /// actual failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// File-backed queue store.
#[derive(Debug)]
pub struct MissionQueue {
    root: PathBuf,
    lock: Mutex<()>,
}

impl MissionQueue {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            lock: Mutex::new(()),
        }
    }

    fn load(&self) -> Vec<Mission> {
        let path = missions_path(&self.root);
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn store(&self, missions: &[Mission]) -> Result<()> {
        let path = missions_path(&self.root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(missions)?)?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    /// Append a mission. Empty tasks are rejected — an intentless queue
    /// entry is noise, not data.
    pub fn enqueue(&self, task: &str, mode: MissionMode) -> Result<Mission> {
        anyhow::ensure!(
            !task.trim().is_empty(),
            "mission task must not be empty"
        );
        let _g = self.lock.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let mission = Mission {
            id: Uuid::new_v4().to_string(),
            task: task.trim().to_string(),
            mode,
            state: MissionState::Queued,
            created_at: now.clone(),
            updated_at: now,
            ledger_session: None,
            summary: None,
        };
        let mut missions = self.load();
        missions.push(mission.clone());
        self.store(&missions)?;
        Ok(mission)
    }

    /// Claim the oldest queued mission.
    pub fn claim_next(&self) -> Option<Mission> {
        let _g = self.lock.lock().unwrap();
        let mut missions = self.load();
        let next = missions
            .iter_mut()
            .find(|m| m.state == MissionState::Queued)?;
        next.state = MissionState::Running;
        next.updated_at = chrono::Utc::now().to_rfc3339();
        let claimed = next.clone();
        self.store(&missions).ok()?;
        Some(claimed)
    }

    /// Record the outcome of a claimed mission.
    pub fn finish(&self, id: &str, ok: bool, summary: &str, ledger_session: Option<String>) {
        let _g = self.lock.lock().unwrap();
        let mut missions = self.load();
        if let Some(m) = missions.iter_mut().find(|m| m.id == id) {
            m.state = if ok {
                MissionState::Done
            } else {
                MissionState::Failed
            };
            m.summary = Some(summary.to_string());
            m.ledger_session = ledger_session;
            m.updated_at = chrono::Utc::now().to_rfc3339();
            let _ = self.store(&missions);
        }
    }

    /// Complete list, newest first.
    pub fn list(&self) -> Vec<Mission> {
        let mut missions = self.load();
        missions.reverse();
        missions
    }

    /// Remove everything. The file is rewritten, not deleted — an emptied
    /// queue is state, not the absence of one.
    pub fn clear(&self) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        self.store(&[])
    }
}

/// Build the plan document for a `plan` mission: the persisted intelligence
/// index ranks the repo's own architecture against the task — no code runs.
pub fn build_plan(root: &Path, task: &str) -> Result<String> {
    let payload = crate::intelligence::api::repo_intel_payload(root, task)?;
    let ranked = payload["results"]
        .as_array()
        .map(|a| a.iter().take(5).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut plan = String::new();
    plan.push_str("PLAN (no code executed — mode: plan)\n");
    plan.push_str(&format!("task: {task}\n\n"));
    plan.push_str("1. Study the ranked context below; each item cites why the retriever surfaced it.\n");
    plan.push_str("2. Design the change; identify the files to touch and the tests that must pass.\n");
    plan.push_str("3. Switch to BUILD mode to execute — candidates will be verified against the real suite.\n\n");
    if ranked.is_empty() {
        plan.push_str("no ranked context available for this task\n");
    } else {
        plan.push_str("ranked context:\n");
        for r in &ranked {
            plan.push_str(&format!(
                "  - {} ({}) — {}\n",
                r["resource"].as_str().unwrap_or("?"),
                r["resource_type"].as_str().unwrap_or("?"),
                r["reason"].as_str().unwrap_or("")
            ));
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        (dir, root)
    }

    #[test]
    fn enqueue_persists_across_queue_instances() {
        let (_d, root) = temp_root();
        let q = MissionQueue::new(root.clone());
        let m = q.enqueue("fix the ledger", MissionMode::Build).unwrap();
        let q2 = MissionQueue::new(root);
        let list = q2.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, m.id);
        assert_eq!(list[0].mode, MissionMode::Build);
        assert_eq!(list[0].state, MissionState::Queued);
    }

    #[test]
    fn empty_task_is_rejected() {
        let (_d, root) = temp_root();
        let q = MissionQueue::new(root);
        assert!(q.enqueue("   ", MissionMode::Build).is_err());
    }

    #[test]
    fn claim_is_fifo_and_marks_running() {
        let (_d, root) = temp_root();
        let q = MissionQueue::new(root);
        let a = q.enqueue("first", MissionMode::Build).unwrap();
        let _b = q.enqueue("second", MissionMode::Plan).unwrap();
        let claimed = q.claim_next().unwrap();
        assert_eq!(claimed.id, a.id);
        assert_eq!(claimed.state, MissionState::Running);
        // FIFO order: the second queued mission claims next, not the one
        // already running (a single-worker drain claims one at a time).
        let second = q.claim_next().unwrap();
        assert_eq!(second.task, "second");
        assert_eq!(second.state, MissionState::Running);
        assert!(q.claim_next().is_none(), "no queued missions left");
    }

    #[test]
    fn finish_records_outcome_and_ledger_session() {
        let (_d, root) = temp_root();
        let q = MissionQueue::new(root);
        let m = q.enqueue("do it", MissionMode::Build).unwrap();
        q.claim_next();
        q.finish(&m.id, true, "candidate 0 selected: 3 of 3 checks", Some("sess-1".into()));
        let list = q.list();
        assert_eq!(list[0].state, MissionState::Done);
        assert_eq!(list[0].ledger_session.as_deref(), Some("sess-1"));
        assert!(list[0].summary.as_deref().unwrap().contains("3 of 3"));
    }

    #[test]
    fn failed_missions_are_recorded_as_failed() {
        let (_d, root) = temp_root();
        let q = MissionQueue::new(root);
        let m = q.enqueue("break it", MissionMode::Build).unwrap();
        q.claim_next();
        q.finish(&m.id, false, "suite failed: 2 tests red", None);
        assert_eq!(q.list()[0].state, MissionState::Failed);
    }

    #[test]
    fn clear_empties_but_keeps_the_file() {
        let (_d, root) = temp_root();
        let q = MissionQueue::new(root.clone());
        q.enqueue("one", MissionMode::Build).unwrap();
        q.enqueue("two", MissionMode::Plan).unwrap();
        q.clear().unwrap();
        assert!(q.list().is_empty());
        assert!(missions_path(&root).exists(), "queue file must survive clear");
    }

    #[test]
    fn plan_mode_never_executes_and_cites_real_context() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("engine.rs"), "pub fn run_engine() -> u32 { 1 }\n").unwrap();
        let plan = build_plan(root, "engine run").unwrap();
        assert!(plan.contains("PLAN"), "{plan}");
        assert!(plan.contains("engine run"), "{plan}");
        assert!(plan.contains("run_engine"), "plan must cite real symbols: {plan}");
    }
}
