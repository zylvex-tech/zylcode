//! Durable factory-job storage — directive Phase 3 / Phase 20.
//!
//! One job is one JSON document at
//! `<root>/.zylcode/factory/jobs/<id>.json`. Writes are atomic (temp file +
//! rename), so an interrupted write can corrupt at worst a `.tmp` file and never
//! the job. That is the same discipline [`crate::claim::ClaimStore`] and
//! [`crate::evidence_graph::EvidenceGraph`] already use, chosen deliberately so
//! a reader does not have to learn a fourth persistence convention.
//!
//! # Why a file, not a database
//!
//! A factory job is small, human-inspectable, and must survive a crash without a
//! migration. The execution *ledger* is already SQLite (`sqlite_ledger.rs`); the
//! job header does not need to be. Keeping it JSON means an operator can read
//! exactly what the factory believed at the moment it stopped.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::job::FactoryJob;

/// Directory holding job documents: `<root>/.zylcode/factory/jobs`.
pub fn jobs_dir(root: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("FACTORY_JOBS_DIR") {
        return PathBuf::from(p);
    }
    root.join(".zylcode").join("factory").join("jobs")
}

/// File-backed store for factory jobs.
#[derive(Debug)]
pub struct JobStore {
    dir: PathBuf,
    lock: Mutex<()>,
}

impl JobStore {
    /// Open the store rooted at a workspace.
    pub fn open(root: &Path) -> Result<Self> {
        let dir = jobs_dir(root);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating factory jobs dir {}", dir.display()))?;
        Ok(Self {
            dir,
            lock: Mutex::new(()),
        })
    }

    /// Open an explicit directory (tests, custom embeddings).
    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Persist a job atomically. Refuses a job whose graph does not validate —
    /// an invalid graph must never reach disk, because a resume would then
    /// execute nonsense.
    pub fn save(&self, job: &FactoryJob) -> Result<PathBuf> {
        if job.id.trim().is_empty() {
            bail!("factory job id must not be empty");
        }
        job.graph
            .validate()
            .context("refusing to persist an invalid task graph")?;
        let _g = self.lock.lock().unwrap();
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path_for(&job.id);
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_string_pretty(job)?;
        std::fs::write(&tmp, body)
            .with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("renaming into {}", path.display()))?;
        Ok(path)
    }

    /// Load one job by id.
    pub fn load(&self, id: &str) -> Result<FactoryJob> {
        let path = self.path_for(id);
        if !path.exists() {
            bail!("no factory job with id {id} at {}", path.display());
        }
        let raw = std::fs::read_to_string(&path)?;
        if raw.trim().is_empty() {
            bail!("factory job file {} is empty — refusing to read it as a job", path.display());
        }
        let job: FactoryJob = serde_json::from_str(&raw)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(job)
    }

    /// All jobs, newest first by `updated_at`. A malformed file is surfaced as
    /// an error rather than silently skipped — a store that hides a corrupt
    /// record cannot be trusted about the records it does show.
    pub fn list(&self) -> Result<Vec<FactoryJob>> {
        let mut jobs = Vec::new();
        if !self.dir.exists() {
            return Ok(jobs);
        }
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)?;
            let job: FactoryJob = serde_json::from_str(&raw)
                .with_context(|| format!("parsing {}", path.display()))?;
            jobs.push(job);
        }
        jobs.sort_by_key(|j| std::cmp::Reverse(j.updated_at));
        Ok(jobs)
    }

    /// Remove a job document. Returns whether a file was removed.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let _g = self.lock.lock().unwrap();
        let path = self.path_for(id);
        if path.exists() {
            std::fs::remove_file(&path)?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::graph::{TaskAction, TaskKind, TaskNode, TaskState};
    use crate::factory::job::{FactoryJob, JobStage};

    fn tmp_store(tag: &str) -> (tempfile::TempDir, JobStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = JobStore::with_dir(dir.path().join(format!("jobs-{tag}")));
        (dir, store)
    }

    fn sample_job() -> FactoryJob {
        let mut job = FactoryJob::new("build auth", "proj-1", "repo", "main", "/wt", "tester");
        job.graph
            .add_node(TaskNode::new(
                "t1",
                TaskKind::Requirements,
                "write requirements",
                TaskAction::Note { text: "req".into() },
            ))
            .unwrap();
        job
    }

    #[test]
    fn save_load_roundtrip_is_stable() {
        let (_d, store) = tmp_store("roundtrip");
        let job = sample_job();
        store.save(&job).unwrap();
        let back = store.load(&job.id).unwrap();
        assert_eq!(back, job, "a saved job must load byte-identical in value");
    }

    #[test]
    fn list_returns_jobs_and_survives_a_second_store() {
        let (_d, store) = tmp_store("list");
        let a = sample_job();
        let mut b = sample_job();
        b.id = "second".to_string();
        store.save(&a).unwrap();
        store.save(&b).unwrap();
        let reopened = JobStore::with_dir(store.dir().to_path_buf());
        let mut ids: Vec<String> = reopened.list().unwrap().into_iter().map(|j| j.id).collect();
        ids.sort();
        let mut expected = vec![a.id.clone(), b.id.clone()];
        expected.sort();
        assert_eq!(ids, expected);
    }

    #[test]
    fn saving_an_invalid_graph_is_refused() {
        let (_d, store) = tmp_store("invalid");
        let mut job = sample_job();
        job.graph
            .add_node(
                TaskNode::new(
                    "t2",
                    TaskKind::Test,
                    "orphan",
                    TaskAction::Note { text: "x".into() },
                )
                .depends_on(&["ghost"]),
            )
            .unwrap();
        let err = store.save(&job).unwrap_err().to_string();
        assert!(err.contains("invalid task graph"), "{err}");
    }

    #[test]
    fn loading_a_missing_job_is_an_error_not_an_empty_default() {
        let (_d, store) = tmp_store("missing");
        assert!(store.load("nope").is_err());
    }

    #[test]
    fn a_corrupt_job_file_is_surfaced_not_skipped() {
        let (_d, store) = tmp_store("corrupt");
        std::fs::create_dir_all(store.dir()).unwrap();
        std::fs::write(store.dir().join("bad.json"), "{ not json").unwrap();
        let err = store.list();
        assert!(err.is_err(), "a corrupt record must not be silently dropped");
    }

    #[test]
    fn persistence_captures_progress_for_resume() {
        let (_d, store) = tmp_store("progress");
        let mut job = sample_job();
        // A second, unfinished task so the job is genuinely mid-flight — a job
        // whose every node succeeded would be complete and correctly report
        // DELIVERY, which would not test resume at all.
        job.graph
            .add_node(
                TaskNode::new(
                    "t2",
                    TaskKind::Test,
                    "run the suite",
                    TaskAction::Note { text: "test".into() },
                )
                .depends_on(&["t1"]),
            )
            .unwrap();
        job.graph.get_mut("t1").unwrap().state = TaskState::Succeeded;
        job.recompute_stage();
        store.save(&job).unwrap();

        let resumed = store.load(&job.id).unwrap();
        assert_eq!(resumed.graph.get("t1").unwrap().state, TaskState::Succeeded);
        assert_eq!(resumed.graph.get("t2").unwrap().state, TaskState::Pending);
        assert_eq!(resumed.stage, JobStage::Requirements);
        assert!(!resumed.graph.is_complete());
    }

    #[test]
    fn delete_removes_the_document() {
        let (_d, store) = tmp_store("delete");
        let job = sample_job();
        store.save(&job).unwrap();
        assert!(store.delete(&job.id).unwrap());
        assert!(!store.delete(&job.id).unwrap());
        assert!(store.load(&job.id).is_err());
    }
}
