use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use crate::ledger::{ExecutionState, LedgerEntry, LedgerStore, SessionCheckpoint};

pub struct MemoryLedgerStore {
    entries: Mutex<HashMap<Uuid, LedgerEntry>>,
    session_entries: Mutex<HashMap<Uuid, Vec<Uuid>>>,
    checkpoints: Mutex<HashMap<Uuid, SessionCheckpoint>>,
}

impl MemoryLedgerStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            session_entries: Mutex::new(HashMap::new()),
            checkpoints: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryLedgerStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LedgerStore for MemoryLedgerStore {
    async fn append(&self, entry: LedgerEntry) -> Result<()> {
        let mut entries = self.entries.lock().unwrap();
        let mut session_entries = self.session_entries.lock().unwrap();

        session_entries
            .entry(entry.session_id)
            .or_default()
            .push(entry.id);

        entries.insert(entry.id, entry);
        Ok(())
    }

    async fn get_entries(&self, session_id: Uuid) -> Result<Vec<LedgerEntry>> {
        let entries = self.entries.lock().unwrap();
        let session_entries = self.session_entries.lock().unwrap();

        if let Some(entry_ids) = session_entries.get(&session_id) {
            let mut result = Vec::new();
            for id in entry_ids {
                if let Some(entry) = entries.get(id) {
                    result.push(entry.clone());
                }
            }
            result.sort_by_key(|a| a.timestamp);
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_entry(&self, entry_id: Uuid) -> Result<Option<LedgerEntry>> {
        let entries = self.entries.lock().unwrap();
        Ok(entries.get(&entry_id).cloned())
    }

    async fn update_state(
        &self,
        entry_id: Uuid,
        state: ExecutionState,
        payload: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<()> {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.get_mut(&entry_id) {
            entry.state = state;
            entry.payload = payload;
            entry.error = error;
        }
        Ok(())
    }

    async fn save_checkpoint(&self, checkpoint: SessionCheckpoint) -> Result<()> {
        let mut checkpoints = self.checkpoints.lock().unwrap();
        checkpoints.insert(checkpoint.session_id, checkpoint);
        Ok(())
    }

    async fn load_checkpoint(&self, session_id: Uuid) -> Result<Option<SessionCheckpoint>> {
        let checkpoints = self.checkpoints.lock().unwrap();
        Ok(checkpoints.get(&session_id).cloned())
    }
}
