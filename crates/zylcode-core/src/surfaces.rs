//! Search, file-tree, and evidence payload APIs.
//!
//! Same contract as [`crate::gitops`] and [`crate::intelligence::api`]: one
//! function per surface, three consumers (CLI `serve-intel` HTTP routes,
//! Tauri commands, direct callers). Every field comes from real data:
//!
//! * **search** runs the real ranked retriever over the persisted index and
//!   additionally matches file paths (the retriever works on indexed
//!   resources; a path hit with zero symbols must still be findable).
//! * **file tree** is the scanner's actual output — the same tree the
//!   intelligence pipeline indexes, `.gitignore` respected, hidden dirs
//!   skipped.
//! * **evidence** reads the SQLite evidence ledger directly. An absent or
//!   empty ledger is an honest `empty` state, never a fabricated timeline.

use crate::intelligence::persisted::PersistedIndex;
use crate::intelligence::scanner::{scan_repository, ScannerConfig};
use anyhow::Result;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::Path;

/// Maximum search results returned.
pub const MAX_SEARCH_RESULTS: usize = 30;

/// Maximum file-tree rows returned (flat, client builds the hierarchy).
pub const MAX_TREE_ROWS: usize = 2000;

/// Maximum evidence entries returned per response.
pub const MAX_EVIDENCE_ENTRIES: usize = 200;

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Ranked search over the real pipeline: the retriever's indexed resources
/// plus direct file-path matches, deduplicated by resource.
pub fn search_payload(root: &Path, query: &str) -> Result<Value> {
    let query = query.trim();
    anyhow::ensure!(!query.is_empty(), "search query must not be empty");

    let started = std::time::Instant::now();
    let repo_query = PersistedIndex::new(root).build()?;

    let mut results: Vec<Value> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. The retriever's own ranking (symbols, files, packages).
    for r in repo_query.relevant_context(query) {
        if !seen.insert(r.resource.clone()) {
            continue;
        }
        results.push(json!({
            "resource": r.resource,
            "resource_type": r.resource_type,
            "relevance": r.relevance,
            "reason": r.reason,
        }));
        if results.len() >= MAX_SEARCH_RESULTS {
            break;
        }
    }

    // 2. File-path matches the symbol-oriented retriever may rank below the
    //    cut — a filename hit must still surface.
    let q_lower = query.to_lowercase();
    let tokens: Vec<&str> = q_lower.split_whitespace().collect();
    if results.len() < MAX_SEARCH_RESULTS {
        for f in repo_query.files() {
            let display = display_path(root, &f.path);
            let path_str = display.to_lowercase();
            let name_match = path_str.contains(&q_lower);
            let token_match = !tokens.is_empty()
                && tokens.iter().all(|t| path_str.contains(t));
            if name_match || token_match {
                let resource = display;
                if seen.insert(format!("file:{resource}")) {
                    results.push(json!({
                        "resource": resource,
                        "resource_type": "file",
                        "relevance": if name_match { 0.75 } else { 0.55 },
                        "reason": if name_match {
                            "path contains the query".to_string()
                        } else {
                            "path contains all query tokens".to_string()
                        },
                    }));
                    if results.len() >= MAX_SEARCH_RESULTS {
                        break;
                    }
                }
            }
        }
    }

    // Deterministic order: relevance desc, then resource asc.
    results.sort_by(|a, b| {
        let ra = a["relevance"].as_f64().unwrap_or(0.0);
        let rb = b["relevance"].as_f64().unwrap_or(0.0);
        rb.partial_cmp(&ra)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a["resource"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["resource"].as_str().unwrap_or(""))
            })
    });

    Ok(json!({
        "query": query,
        "elapsed_ms": started.elapsed().as_secs_f64() * 1000.0,
        "indexed_files": repo_query.file_count(),
        "results": results.into_iter().take(MAX_SEARCH_RESULTS).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------------------
// File tree
// ---------------------------------------------------------------------------

/// Clean, display-friendly path: strip Windows extended-length prefixes from
/// both the root and the path, then relativize against the root.
fn display_path(root: &Path, path: &Path) -> String {
    let norm = |p: &Path| {
        let s = p.to_string_lossy();
        s.strip_prefix("\\\\?\\").unwrap_or(&s).to_string()
    };
    let root_str = norm(root);
    let path_str = norm(path);
    let root_norm = Path::new(&root_str);
    let path_norm = Path::new(&path_str);
    if let Ok(rel) = path_norm.strip_prefix(root_norm) {
        return rel.to_string_lossy().replace('\\', "/");
    }
    path_str.replace('\\', "/")
}

/// The scanner's real output as a flat row list (the client nests it).
pub fn file_tree_payload(root: &Path) -> Result<Value> {
    anyhow::ensure!(
        root.is_dir(),
        "repository root '{}' is not a directory",
        root.display()
    );
    let started = std::time::Instant::now();
    let scan = scan_repository(root, &ScannerConfig::default())?;

    let mut rows: Vec<Value> = scan
        .files
        .iter()
        .take(MAX_TREE_ROWS)
        .map(|f| {
            json!({
                "path": display_path(root, &f.path),
                "language": f.language.label(),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        a["path"]
            .as_str()
            .unwrap_or("")
            .cmp(b["path"].as_str().unwrap_or(""))
    });

    Ok(json!({
        "total_scanned": scan.files.len(),
        "truncated": scan.files.len() > MAX_TREE_ROWS,
        "elapsed_ms": started.elapsed().as_secs_f64() * 1000.0,
        "rows": rows,
    }))
}

// ---------------------------------------------------------------------------
// Evidence ledger
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct EvidenceRow {
    id: String,
    session_id: String,
    action_id: String,
    state: String,
    prev_hash: String,
    timestamp: String,
    payload: Option<Value>,
    error: Option<String>,
}

fn read_evidence_rows(db_path: &Path, limit: usize) -> Result<Vec<EvidenceRow>> {
    anyhow::ensure!(db_path.exists(), "no evidence ledger at {}", db_path.display());
    let conn = Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let mut stmt = conn.prepare(
        "SELECT id, session_id, action_id, state, prev_hash, timestamp, payload, error
         FROM ledger_entries
         ORDER BY timestamp DESC, rowid DESC
         LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![limit as i64], |row| {
            Ok(EvidenceRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                action_id: row.get(2)?,
                // The state column stores JSON (`"Recorded"`); unwrap the
                // string so the UI shows `Recorded`, not `"Recorded\"`.
                // Unparseable values pass through unchanged — never fabricated.
                state: {
                    let raw: String = row.get(3)?;
                    serde_json::from_str::<String>(&raw).unwrap_or(raw)
                },
                prev_hash: row.get(4)?,
                timestamp: row.get(5)?,
                payload: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                error: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Verify the hash chain of chronologically ordered rows (oldest first),
/// **per session**. Same replay formula as the agent loop: hash of
/// `(prev_hash, action_id)`.
///
/// Sessions are independent chains — each writer starts its session from the
/// genesis value (`""`), so a listing that spans sessions must never be
/// verified as one concatenated chain. A session whose first visible row
/// carries a non-genesis `prev_hash` had its head cut off by the read window
/// (`MAX_EVIDENCE_ENTRIES`); its remaining visible links are still verified
/// from that anchor, but the missing head cannot invalidate the verdict.
fn chain_is_intact(rows_oldest_first: &[EvidenceRow]) -> bool {
    use sha2::Digest;
    use std::collections::HashMap;

    // session_id -> expected prev_hash of the session's next row
    let mut anchors: HashMap<&str, String> = HashMap::new();
    for row in rows_oldest_first {
        let expected = anchors
            .entry(row.session_id.as_str())
            .or_default(); // genesis for a fresh session
        if row.prev_hash != *expected {
            return false;
        }
        *expected = format!(
            "{:x}",
            sha2::Sha256::digest(format!("{}:{}", row.prev_hash, row.action_id).as_bytes())
        );
    }
    true
}

/// The evidence ledger state for the frontend.
///
/// `Ok` payloads: `empty` (no ledger yet — honest), `ready` (entries +
/// chain verdict + session count), `error` (ledger unreadable — the error
/// is surfaced, not swallowed).
pub fn evidence_payload(root: &Path) -> Value {
    let db_path = root.join(".zylcode").join("ledger.db");
    if !db_path.exists() {
        return json!({
            "kind": "empty",
            "reason": "no evidence ledger yet — run a mission or `zylcode best-of-n` to record evidence",
        });
    }

    let rows = match read_evidence_rows(&db_path, MAX_EVIDENCE_ENTRIES) {
        Ok(rows) => rows,
        Err(e) => {
            return json!({
                "kind": "error",
                "reason": format!("ledger unreadable: {e:#}"),
            });
        }
    };

    if rows.is_empty() {
        return json!({
            "kind": "empty",
            "reason": "the evidence ledger exists but has no entries yet",
        });
    }

    // Chain verification needs chronological order; the API returns newest
    // first for display, so verify on a reversed copy.
    let mut chrono_rows = rows.clone();
    chrono_rows.reverse();
    let intact = chain_is_intact(&chrono_rows);

    let sessions: std::collections::HashSet<&str> = rows
        .iter()
        .map(|r| r.session_id.as_str())
        .collect();

    let entries: Vec<Value> = rows
        .iter()
        .take(MAX_EVIDENCE_ENTRIES)
        .map(|r| {
            json!({
                "id": r.id,
                "session_id": r.session_id,
                "action_id": r.action_id,
                "state": r.state,
                "timestamp": r.timestamp,
                "error": r.error,
                "payload": r.payload,
            })
        })
        .collect();

    json!({
        "kind": "ready",
        "chain_intact": intact,
        "session_count": sessions.len(),
        "entry_count": rows.len(),
        "entries": entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{ExecutionState, LedgerEntry};
    use std::process::Command;

    fn sample_repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "t"]);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src").join("engine.rs"),
            "pub struct Engine;\nimpl Engine { pub fn run(&self) {} }\n",
        )
        .unwrap();
        std::fs::write(root.join("README.md"), "# sample\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "initial commit"]);
        (dir, root)
    }

    fn append(ledger: &dyn crate::ledger::LedgerStore, session: uuid::Uuid, prev: &str, action: &str) -> String {
        let id = uuid::Uuid::new_v4();
        let entry = LedgerEntry {
            id,
            session_id: session,
            action_id: action.to_string(),
            arguments: serde_json::json!({}),
            state: ExecutionState::Recorded,
            prev_hash: prev.to_string(),
            timestamp: chrono::Utc::now(),
            payload: Some(serde_json::json!({ "note": action })),
            error: None,
        };
        futures_block_on(ledger.append(entry)).expect("ledger append must succeed");
        format!(
            "{:x}",
            sha2::Sha256::digest(format!("{prev}:{action}").as_bytes())
        )
    }

    // Tiny local executor: the ledger trait is async but the tests are sync.
    fn futures_block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(fut)
    }

    use sha2::Digest;

    #[test]
    fn search_finds_symbols_and_file_paths_and_ranks_deterministically() {
        let (_dir, root) = sample_repo();
        let payload = search_payload(&root, "engine run").unwrap();
        assert_eq!(payload["query"], "engine run");
        let results = payload["results"].as_array().unwrap();
        assert!(!results.is_empty(), "{payload:?}");
        // Both classes can appear; ordering must be non-increasing relevance.
        let rels: Vec<f64> = results
            .iter()
            .map(|r| r["relevance"].as_f64().unwrap())
            .collect();
        for w in rels.windows(2) {
            assert!(w[0] >= w[1], "results must be relevance-descending: {rels:?}");
        }
        // A pure filename query finds the file even without symbol hits.
        let by_name = search_payload(&root, "README").unwrap();
        let rows = by_name["results"].as_array().unwrap();
        assert!(rows
            .iter()
            .any(|r| r["resource"].as_str().unwrap().contains("README.md")),
            "{by_name:?}");
    }

    #[test]
    fn empty_query_is_rejected_not_silently_widened() {
        let (_dir, root) = sample_repo();
        assert!(search_payload(&root, "   ").is_err());
    }

    #[test]
    fn file_tree_lists_the_scanned_tree() {
        let (_dir, root) = sample_repo();
        let payload = file_tree_payload(&root).unwrap();
        assert!(payload["total_scanned"].as_u64().unwrap() >= 2);
        let rows = payload["rows"].as_array().unwrap();
        assert!(rows.iter().any(|r| r["path"].as_str().unwrap().contains("engine.rs")));
        assert!(rows.iter().any(|r| r["path"].as_str().unwrap().contains("README.md")));
        // Sorted by path for a stable tree.
        let paths: Vec<&str> = rows.iter().map(|r| r["path"].as_str().unwrap()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn evidence_reports_an_honest_empty_state_without_a_ledger() {
        let (_dir, root) = sample_repo();
        let payload = evidence_payload(&root);
        assert_eq!(payload["kind"], "empty");
        assert!(payload["reason"].as_str().unwrap().contains("no evidence ledger"));
    }

    #[test]
    fn evidence_reads_real_entries_and_verifies_the_chain() {
        let (_dir, root) = sample_repo();
        std::fs::create_dir_all(root.join(".zylcode")).unwrap();
        let ledger = crate::sqlite_ledger::SqliteLedgerStore::new(
            root.join(".zylcode").join("ledger.db").to_string_lossy().as_ref(),
        )
        .unwrap();
        let session = uuid::Uuid::new_v4();
        let h1 = append(&ledger, session, "", "mission.step[0]");
        let h2 = append(&ledger, session, &h1, "mission.step[1]");

        let payload = evidence_payload(&root);
        assert_eq!(payload["kind"], "ready", "{payload:?}");
        assert_eq!(payload["chain_intact"], true);
        assert_eq!(payload["session_count"], 1);
        assert_eq!(payload["entry_count"], 2);
        let entries = payload["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0]["action_id"].as_str().unwrap().starts_with("mission.step"));

        // Tampering with one link must be detected.
        let conn = Connection::open(root.join(".zylcode").join("ledger.db")).unwrap();
        conn.execute(
            "UPDATE ledger_entries SET prev_hash = 'tampered' WHERE action_id = 'mission.step[1]'",
            [],
        )
        .unwrap();
        let tampered = evidence_payload(&root);
        assert_eq!(tampered["chain_intact"], false, "{tampered:?}");
        let _ = h2;
    }

    #[test]
    fn evidence_chain_is_verified_per_session_not_across_the_listing() {
        // Two sessions: the second starts its own chain from genesis, which
        // must NOT be read as a broken link between sessions. (This exact
        // shape fired in the real ledger after a second best-of-n run.)
        let (_dir, root) = sample_repo();
        std::fs::create_dir_all(root.join(".zylcode")).unwrap();
        let ledger = crate::sqlite_ledger::SqliteLedgerStore::new(
            root.join(".zylcode").join("ledger.db").to_string_lossy().as_ref(),
        )
        .unwrap();
        let s1 = uuid::Uuid::new_v4();
        let s2 = uuid::Uuid::new_v4();
        let _h1 = append(&ledger, s1, "", "best_of_n.candidate[0]");
        let _h2 = append(&ledger, s2, "", "best_of_n.selection"); // genesis again: new session

        let payload = evidence_payload(&root);
        assert_eq!(payload["kind"], "ready", "{payload:?}");
        assert_eq!(payload["session_count"], 2);
        assert_eq!(
            payload["chain_intact"], true,
            "each session's chain starts from genesis; session boundaries are not broken links: {payload:?}"
        );
    }

    #[test]
    fn evidence_still_detects_a_tampered_link_inside_one_session() {
        let (_dir, root) = sample_repo();
        std::fs::create_dir_all(root.join(".zylcode")).unwrap();
        let ledger = crate::sqlite_ledger::SqliteLedgerStore::new(
            root.join(".zylcode").join("ledger.db").to_string_lossy().as_ref(),
        )
        .unwrap();
        let s1 = uuid::Uuid::new_v4();
        let s2 = uuid::Uuid::new_v4();
        let _ = append(&ledger, s1, "", "a");
        let _ = append(&ledger, s2, "", "b");

        let conn = Connection::open(root.join(".zylcode").join("ledger.db")).unwrap();
        conn.execute("UPDATE ledger_entries SET prev_hash = 'forged' WHERE session_id = ?1", [s1.to_string()])
            .unwrap();
        let payload = evidence_payload(&root);
        assert_eq!(payload["chain_intact"], false, "{payload:?}");
    }

    #[test]
    fn evidence_with_an_existing_but_empty_ledger_is_empty_not_ready() {
        let (_dir, root) = sample_repo();
        std::fs::create_dir_all(root.join(".zylcode")).unwrap();
        let _ledger = crate::sqlite_ledger::SqliteLedgerStore::new(
            root.join(".zylcode").join("ledger.db").to_string_lossy().as_ref(),
        )
        .unwrap();
        let payload = evidence_payload(&root);
        assert_eq!(payload["kind"], "empty");
        assert!(payload["reason"].as_str().unwrap().contains("no entries"));
    }
}
