use anyhow::Result;
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::sync::Mutex;
use uuid::Uuid;

use crate::ledger::{ExecutionState, LedgerEntry, LedgerStore, SessionCheckpoint};

pub struct SqliteLedgerStore {
    conn: Mutex<Connection>,
}

impl SqliteLedgerStore {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS ledger_entries (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                action_id TEXT NOT NULL,
                arguments TEXT NOT NULL,
                state TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                payload TEXT,
                error TEXT
            );
            
            CREATE TABLE IF NOT EXISTS session_checkpoints (
                session_id TEXT PRIMARY KEY,
                last_entry_id TEXT NOT NULL,
                agent_state TEXT NOT NULL,
                context TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_session_id ON ledger_entries(session_id);
            ",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

#[async_trait]
impl LedgerStore for SqliteLedgerStore {
    async fn append(&self, entry: LedgerEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO ledger_entries (id, session_id, action_id, arguments, state, prev_hash, timestamp, payload, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id.to_string(),
                entry.session_id.to_string(),
                entry.action_id,
                entry.arguments.to_string(),
                serde_json::to_string(&entry.state)?,
                entry.prev_hash,
                entry.timestamp.to_rfc3339(),
                entry.payload.map(|p| p.to_string()),
                entry.error,
            ],
        )?;
        Ok(())
    }

    async fn get_entries(&self, session_id: Uuid) -> Result<Vec<LedgerEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, action_id, arguments, state, prev_hash, timestamp, payload, error
             FROM ledger_entries
             WHERE session_id = ?1
             ORDER BY timestamp ASC"
        )?;

        let entries = stmt
            .query_map(params![session_id.to_string()], |row| {
                Ok(LedgerEntry {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    session_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    action_id: row.get(2)?,
                    arguments: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    state: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                    prev_hash: row.get(5)?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    payload: row
                        .get::<_, Option<String>>(7)?
                        .map(|s| serde_json::from_str(&s).unwrap()),
                    error: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    async fn get_entry(&self, entry_id: Uuid) -> Result<Option<LedgerEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, action_id, arguments, state, prev_hash, timestamp, payload, error
             FROM ledger_entries
             WHERE id = ?1"
        )?;

        let mut entries = stmt.query_map(params![entry_id.to_string()], |row| {
            Ok(LedgerEntry {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                session_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                action_id: row.get(2)?,
                arguments: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                state: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                prev_hash: row.get(5)?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                payload: row
                    .get::<_, Option<String>>(7)?
                    .map(|s| serde_json::from_str(&s).unwrap()),
                error: row.get(8)?,
            })
        })?;

        entries.next().transpose().map_err(Into::into)
    }

    async fn update_state(
        &self,
        entry_id: Uuid,
        state: ExecutionState,
        payload: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE ledger_entries SET state = ?1, payload = ?2, error = ?3 WHERE id = ?4",
            params![
                serde_json::to_string(&state)?,
                payload.map(|p| p.to_string()),
                error,
                entry_id.to_string()
            ],
        )?;
        Ok(())
    }

    async fn save_checkpoint(&self, checkpoint: SessionCheckpoint) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO session_checkpoints (session_id, last_entry_id, agent_state, context, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                checkpoint.session_id.to_string(),
                checkpoint.last_entry_id.to_string(),
                checkpoint.agent_state,
                checkpoint.context.to_string(),
                checkpoint.timestamp.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    async fn load_checkpoint(&self, session_id: Uuid) -> Result<Option<SessionCheckpoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT session_id, last_entry_id, agent_state, context, timestamp
             FROM session_checkpoints
             WHERE session_id = ?1",
        )?;

        let mut checkpoints = stmt.query_map(params![session_id.to_string()], |row| {
            Ok(SessionCheckpoint {
                session_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                last_entry_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                agent_state: row.get(2)?,
                context: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        })?;

        checkpoints.next().transpose().map_err(Into::into)
    }
}
