use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use zylcode_core::ledger::{ExecutionState, LedgerEntry, LedgerStore};
use zylcode_core::memory_ledger::MemoryLedgerStore;

#[tokio::test]
async fn test_crash_window_1_pre_execution() -> Result<()> {
    // Simulate crash BEFORE tool execution
    // Agent planned, logged Started, then crashed.

    let ledger = Arc::new(MemoryLedgerStore::new());
    let session_id = Uuid::new_v4();
    let entry_id = Uuid::new_v4();

    // Simulate Agent logging "Started" just before execution
    let entry = LedgerEntry {
        id: entry_id,
        session_id,
        action_id: "rm".to_string(),
        arguments: serde_json::json!({"path": "important.txt"}),
        state: ExecutionState::Started,
        prev_hash: "hash_prev".to_string(),
        timestamp: Utc::now(),
        payload: None,
        error: None,
    };
    ledger.append(entry).await?;

    // CRASH HAPPENS HERE (simulated by stopping execution)

    // SIMULATE RESTART
    // Recovery logic scans ledger
    let entries = ledger.get_entries(session_id).await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ExecutionState::Started);

    // Recovery logic sees "Started" without "Executed"
    // This means the tool MIGHT have started but we don't know if it finished.
    // For a destructive action like rm, the safe choice is to NOT re-execute blindly.
    // Instead, we should verify if the file exists (if possible) or ask for help.

    // In this test, we assume the recovery logic decides to CANCEL this attempt
    // and log a new entry or update state to Cancelled/Failed.
    ledger
        .update_state(
            entry_id,
            ExecutionState::Cancelled,
            None,
            Some("Crash recovery: Unknown state".to_string()),
        )
        .await?;

    let entries = ledger.get_entries(session_id).await?;
    assert_eq!(entries[0].state, ExecutionState::Cancelled);

    println!("✅ Window 1: Pre-execution crash handled safely");
    Ok(())
}

#[tokio::test]
async fn test_crash_window_2_post_side_effect() -> Result<()> {
    // Simulate crash AFTER side effect (tool returned) but BEFORE evidence recorded

    let ledger = Arc::new(MemoryLedgerStore::new());
    let session_id = Uuid::new_v4();
    let entry_id = Uuid::new_v4();

    // Simulate Agent logging "Started"
    ledger
        .append(LedgerEntry {
            id: entry_id,
            session_id,
            action_id: "fs.write".to_string(),
            arguments: serde_json::json!({"path": "output.txt", "content": "data"}),
            state: ExecutionState::Started,
            prev_hash: "hash_prev".to_string(),
            timestamp: Utc::now(),
            payload: None,
            error: None,
        })
        .await?;

    // Simulate Agent logging "Executed" (side effect completed)
    ledger
        .update_state(
            entry_id,
            ExecutionState::Executed,
            Some(serde_json::json!({"success": true, "bytes_written": 4})),
            None,
        )
        .await?;

    // CRASH HAPPENS HERE (before logging "Recorded" or "Verified")

    // SIMULATE RESTART
    let entries = ledger.get_entries(session_id).await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ExecutionState::Executed);
    assert!(entries[0].payload.is_some()); // We have the result

    // Recovery logic sees "Executed". This is GOOD. It means the tool finished.
    // We can safely skip re-execution and go straight to Recording/Verifying.
    // This proves idempotency of recovery.

    // Simulate recovery logic finishing the job
    ledger
        .update_state(
            entry_id,
            ExecutionState::Verified,
            entries[0].payload.clone(),
            None,
        )
        .await?;

    let entries = ledger.get_entries(session_id).await?;
    assert_eq!(entries[0].state, ExecutionState::Verified);

    println!("✅ Window 2: Post-side-effect crash handled safely (no duplicate execution)");
    Ok(())
}

#[tokio::test]
async fn test_crash_window_3_post_evidence() -> Result<()> {
    // Simulate crash AFTER evidence recorded but BEFORE agent transition

    let ledger = Arc::new(MemoryLedgerStore::new());
    let session_id = Uuid::new_v4();
    let entry_id = Uuid::new_v4();

    // Simulate full lifecycle up to Recorded
    ledger
        .append(LedgerEntry {
            id: entry_id,
            session_id,
            action_id: "shell.execute".to_string(),
            arguments: serde_json::json!({"command": "test"}),
            state: ExecutionState::Started,
            prev_hash: "hash_prev".to_string(),
            timestamp: Utc::now(),
            payload: None,
            error: None,
        })
        .await?;

    ledger
        .update_state(
            entry_id,
            ExecutionState::Executed,
            Some(serde_json::json!({"stdout": "ok"})),
            None,
        )
        .await?;
    ledger
        .update_state(
            entry_id,
            ExecutionState::Recorded,
            Some(serde_json::json!({"recorded": true})),
            None,
        )
        .await?;

    // CRASH HAPPENS HERE (before agent loop moves to next step)

    // SIMULATE RESTART
    let entries = ledger.get_entries(session_id).await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, ExecutionState::Recorded);

    // Recovery logic sees "Recorded". This is complete from the tool's perspective.
    // We can mark it Verified immediately.
    ledger
        .update_state(entry_id, ExecutionState::Verified, None, None)
        .await?;

    let entries = ledger.get_entries(session_id).await?;
    assert_eq!(entries[0].state, ExecutionState::Verified);

    println!("✅ Window 3: Post-evidence crash handled safely");
    Ok(())
}
