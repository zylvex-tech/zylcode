use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the lifecycle of a tool execution or agent action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionState {
    /// The action was planned by the agent.
    Planned,
    /// The action was approved by the user or policy.
    Approved,
    /// The action has started execution (side effects may begin).
    Started,
    /// The action has completed execution (side effects have occurred).
    Executed,
    /// The evidence of the execution has been recorded.
    Recorded,
    /// The execution and evidence have been verified.
    Verified,
    /// The action was cancelled or rolled back.
    Cancelled,
    /// The action failed.
    Failed,
}

/// An immutable record in the Evidence Ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Unique ID for this specific execution attempt.
    pub id: Uuid,
    /// ID of the session this entry belongs to.
    pub session_id: Uuid,
    /// ID of the tool or action.
    pub action_id: String,
    /// Arguments passed to the action.
    pub arguments: serde_json::Value,
    /// Current state of the execution.
    pub state: ExecutionState,
    /// Hash of the previous entry to ensure integrity (simple chain).
    pub prev_hash: String,
    /// Timestamp of the entry creation.
    pub timestamp: DateTime<Utc>,
    /// Result or evidence data (if applicable).
    pub payload: Option<serde_json::Value>,
    /// Error data (if applicable).
    pub error: Option<String>,
}

/// A checkpoint for session recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCheckpoint {
    /// The session ID.
    pub session_id: Uuid,
    /// The ID of the last processed ledger entry.
    pub last_entry_id: Uuid,
    /// The current state of the agent state machine.
    pub agent_state: String,
    /// Serialized context data (e.g., current plan).
    pub context: serde_json::Value,
    /// Timestamp of the checkpoint.
    pub timestamp: DateTime<Utc>,
}

/// Trait for persisting the Evidence Ledger.
#[async_trait::async_trait]
pub trait LedgerStore: Send + Sync {
    /// Append a new entry to the ledger.
    async fn append(&self, entry: LedgerEntry) -> Result<()>;

    /// Get all entries for a session, ordered by timestamp.
    async fn get_entries(&self, session_id: Uuid) -> Result<Vec<LedgerEntry>>;

    /// Get a specific entry by ID.
    async fn get_entry(&self, entry_id: Uuid) -> Result<Option<LedgerEntry>>;

    /// Update an existing entry (only mutable fields like state/payload should change,
    /// but for append-only we might prefer adding a new entry that supersedes the old one.
    /// However, for state updates on the *same* execution attempt, we update in place
    /// to reflect the latest known state of that specific attempt).
    async fn update_state(
        &self,
        entry_id: Uuid,
        state: ExecutionState,
        payload: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<()>;

    /// Save a checkpoint.
    async fn save_checkpoint(&self, checkpoint: SessionCheckpoint) -> Result<()>;

    /// Load the latest checkpoint for a session.
    async fn load_checkpoint(&self, session_id: Uuid) -> Result<Option<SessionCheckpoint>>;
}

use anyhow::Result;
