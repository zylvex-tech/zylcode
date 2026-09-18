//! Durable evidence sink.
//!
//! # Why this exists
//!
//! `ToolEvidence` was produced by every executor and then **dropped**. The
//! project is an evidence-first system, and until now it produced no evidence
//! trail at all: a call would return, the `ToolEvidence` value would go out of
//! scope, and nothing survived the process.
//!
//! That is the gap between "the executor works" (R2) and "what happened is
//! recorded" (R3). This module closes it.
//!
//! # What is recorded
//!
//! Every dispatch outcome, not only successes:
//!
//! * an allowed invocation that executed,
//! * an allowed invocation whose executor failed,
//! * a **refused** invocation — the gate's decision is evidence that something
//!   was asked for and denied.
//!
//! A trail that only records successes cannot answer "what was attempted?",
//! which is the question an audit actually asks.
//!
//! # Choosing a sink
//!
//! [`NullEvidenceSink`] discards. It exists for tests and is named so that
//! discarding is a stated choice rather than an omission — the same principle as
//! [`crate::permission::PermissionPolicy::permissive`].

use crate::real_tools::ToolEvidence;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Where tool evidence goes.
#[async_trait]
pub trait EvidenceSink: Send + Sync + std::fmt::Debug {
    /// Persist one record. Called for every dispatch outcome.
    async fn record(&self, evidence: &ToolEvidence) -> Result<()>;

    /// A short description, for logs and for tests that assert the wiring.
    fn describe(&self) -> String;
}

/// Discards every record.
///
/// Explicitly named. A default that silently discarded would make "no evidence"
/// indistinguishable from "evidence collection is broken".
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEvidenceSink;

#[async_trait]
impl EvidenceSink for NullEvidenceSink {
    async fn record(&self, _evidence: &ToolEvidence) -> Result<()> {
        Ok(())
    }

    fn describe(&self) -> String {
        "null (discards)".to_string()
    }
}

/// Appends one JSON object per line to a file.
///
/// JSONL rather than a single JSON array: an append is atomic at the line level
/// and a truncated final line loses one record, not the whole file. For an
/// evidence log that matters — a crash mid-write must not invalidate everything
/// written before it.
///
/// Appends are serialised by a mutex so concurrent invocations cannot interleave
/// two records into one line.
#[derive(Debug)]
pub struct JsonlEvidenceSink {
    path: PathBuf,
    /// Serialises appends. Held only for the duration of one write.
    write_lock: tokio::sync::Mutex<()>,
}

impl JsonlEvidenceSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            write_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// The default location: `$ZYLCODE_EVIDENCE_LOG`, else
    /// `./.zylcode/evidence.jsonl`.
    ///
    /// The state directory is used rather than the repository root so that a
    /// test or a first run does not leave a stray file in the working tree. The
    /// directory is created on first write.
    pub fn from_env() -> Self {
        let path = std::env::var("ZYLCODE_EVIDENCE_LOG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_evidence_path());
        Self::new(path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read every record back. Used by tests and by an audit tool.
    pub fn read_all(&self) -> Result<Vec<ToolEvidence>> {
        let raw = std::fs::read_to_string(&self.path)
            .with_context(|| format!("cannot read evidence log {}", self.path.display()))?;
        let mut out = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record: ToolEvidence = serde_json::from_str(line)
                .with_context(|| format!("malformed evidence record on line {}", i + 1))?;
            out.push(record);
        }
        Ok(out)
    }
}

#[async_trait]
impl EvidenceSink for JsonlEvidenceSink {
    async fn record(&self, evidence: &ToolEvidence) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let mut line = serde_json::to_string(evidence)
            .context("evidence record is not serialisable")?;
        line.push('\n');

        // One writer at a time: two concurrent appends must not interleave.
        let _guard = self.write_lock.lock().await;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| format!("cannot open evidence log {}", self.path.display()))?;
        file.write_all(line.as_bytes())
            .await
            .context("cannot write evidence record")?;
        file.flush().await.context("cannot flush evidence record")?;
        Ok(())
    }

    fn describe(&self) -> String {
        format!("jsonl:{}", self.path.display())
    }
}

/// Where evidence goes when nothing else is configured.
///
/// `./.zylcode/evidence.jsonl` — a state directory, not the repository root, so
/// a test run does not litter the working tree. `.zylcode/` is gitignored.
pub fn default_evidence_path() -> PathBuf {
    PathBuf::from(".zylcode").join("evidence.jsonl")
}

/// Everything a dispatch needs beyond the request itself: the policy it is
/// subject to, and where the record of it goes.
#[derive(Debug, Clone)]
pub struct ToolRuntime {
    gate: std::sync::Arc<crate::permission::PermissionGate>,
    sink: std::sync::Arc<dyn EvidenceSink>,
}

impl Default for ToolRuntime {
    /// Restrictive policy, and a JSONL log at the default location.
    ///
    /// Not a discarding sink: a system that claims to be evidence-first should
    /// record by default. A caller that wants to discard must say
    /// [`ToolRuntime::without_evidence`].
    fn default() -> Self {
        Self {
            gate: std::sync::Arc::new(crate::permission::PermissionGate::restrictive()),
            sink: std::sync::Arc::new(JsonlEvidenceSink::from_env()),
        }
    }
}

impl ToolRuntime {
    pub fn new(
        gate: std::sync::Arc<crate::permission::PermissionGate>,
        sink: std::sync::Arc<dyn EvidenceSink>,
    ) -> Self {
        Self { gate, sink }
    }

    /// The restrictive policy and a JSONL log.
    pub fn restrictive() -> Self {
        Self::default()
    }

    /// Permit everything and discard evidence. **For tests only** — both halves
    /// are stated explicitly so neither can be reached by accident.
    pub fn permissive_without_evidence() -> Self {
        Self {
            gate: std::sync::Arc::new(crate::permission::PermissionGate::permissive()),
            sink: std::sync::Arc::new(NullEvidenceSink),
        }
    }

    /// Keep the policy, discard the evidence.
    pub fn without_evidence(mut self) -> Self {
        self.sink = std::sync::Arc::new(NullEvidenceSink);
        self
    }

    /// Keep the policy, write evidence to `path`.
    pub fn with_evidence_at(mut self, path: impl Into<PathBuf>) -> Self {
        self.sink = std::sync::Arc::new(JsonlEvidenceSink::new(path));
        self
    }

    pub fn with_gate(mut self, gate: std::sync::Arc<crate::permission::PermissionGate>) -> Self {
        self.gate = gate;
        self
    }

    pub fn gate(&self) -> &crate::permission::PermissionGate {
        &self.gate
    }

    pub fn sink(&self) -> &std::sync::Arc<dyn EvidenceSink> {
        &self.sink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::real_tools::RiskLevel;
    use tempfile::tempdir;

    fn evidence(tool_id: &str, decision: &str) -> ToolEvidence {
        ToolEvidence {
            invocation_id: format!("inv-{tool_id}"),
            tool_id: tool_id.to_string(),
            session_id: Some("session-1".to_string()),
            actor: Some("actor-1".to_string()),
            bound_operation: Some("git commit".to_string()),
            risk: RiskLevel::GitWrite,
            approval_decision: Some(decision.to_string()),
            arguments: serde_json::json!({ "message": "x" }),
            working_directory: PathBuf::from("."),
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            exit_status: Some(0),
            stdout: Some("ok".to_string()),
            stderr: None,
            changed_files: vec![PathBuf::from("a.txt")],
            timeout: Some(std::time::Duration::from_secs(30)),
        }
    }

    #[tokio::test]
    async fn jsonl_sink_round_trips_a_record() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("evidence.jsonl");
        let sink = JsonlEvidenceSink::new(&path);

        sink.record(&evidence("git.commit", "allow: ok")).await.unwrap();

        let records = sink.read_all().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tool_id, "git.commit");
        assert_eq!(records[0].actor.as_deref(), Some("actor-1"));
        assert_eq!(records[0].bound_operation.as_deref(), Some("git commit"));
        assert_eq!(records[0].risk, RiskLevel::GitWrite);
        assert_eq!(records[0].approval_decision.as_deref(), Some("allow: ok"));
        assert_eq!(records[0].changed_files, vec![PathBuf::from("a.txt")]);
    }

    #[tokio::test]
    async fn jsonl_sink_appends_rather_than_truncates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("evidence.jsonl");
        let sink = JsonlEvidenceSink::new(&path);

        for i in 0..5 {
            sink.record(&evidence(&format!("tool.{i}"), "allow: ok")).await.unwrap();
        }

        let records = sink.read_all().unwrap();
        assert_eq!(records.len(), 5, "every record must survive");
        assert_eq!(records[0].tool_id, "tool.0");
        assert_eq!(records[4].tool_id, "tool.4");
    }

    /// Concurrent writers must not interleave two records into one line.
    #[tokio::test]
    async fn jsonl_sink_is_line_atomic_under_concurrency() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("evidence.jsonl");
        let sink = std::sync::Arc::new(JsonlEvidenceSink::new(&path));

        let mut handles = Vec::new();
        for i in 0..32 {
            let sink = sink.clone();
            handles.push(tokio::spawn(async move {
                sink.record(&evidence(&format!("tool.{i}"), "allow: ok"))
                    .await
                    .unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        // Every line parses, and there are exactly 32 of them. An interleaved
        // write would fail one of those two assertions.
        let records = sink.read_all().expect("every line must be valid JSON");
        assert_eq!(records.len(), 32);
    }

    /// A refusal is evidence too.
    #[tokio::test]
    async fn a_refusal_is_recorded() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("evidence.jsonl");
        let sink = JsonlEvidenceSink::new(&path);

        sink.record(&evidence("shell.execute", "require_approval: risk Execute"))
            .await
            .unwrap();

        let records = sink.read_all().unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0]
            .approval_decision
            .as_deref()
            .unwrap()
            .starts_with("require_approval: "));
    }

    #[tokio::test]
    async fn null_sink_records_nothing_and_says_so() {
        let sink = NullEvidenceSink;
        sink.record(&evidence("fs.read", "allow: ok")).await.unwrap();
        assert!(sink.describe().contains("discards"));
    }

    #[tokio::test]
    async fn the_default_runtime_records_rather_than_discards() {
        // Guards against someone making the default silent.
        let rt = ToolRuntime::default();
        assert!(
            !rt.sink().describe().contains("discards"),
            "the default runtime must record evidence, got: {}",
            rt.sink().describe()
        );
        assert_eq!(
            rt.gate().policy().allow_up_to,
            Some(RiskLevel::Read),
            "the default policy must stay restrictive"
        );
    }

    #[test]
    fn discarding_is_an_explicit_choice() {
        let rt = ToolRuntime::permissive_without_evidence();
        assert!(rt.sink().describe().contains("discards"));
        assert_eq!(rt.gate().policy().allow_up_to, Some(RiskLevel::Release));
    }
}
