//! P15 W4 — index freshness: the signal behind the streaming observation
//! surface (`GET /api/repo-context-stream`, `GET /api/repo-context`, and the
//! desktop freshness poll).
//!
//! Three states, each meaning exactly what it says:
//!
//! * **fresh** — the fingerprint measured right now equals the one the stored
//!   index was built at. The index answers for the current tree.
//! * **stale** — the measured fingerprint differs (files really changed since
//!   the index was written), or no trusted index exists at all.
//! * **reindexing** — a full scan for this root is *running in this process*
//!   right now (registered by [`ReindexGuard`], which every
//!   [`build_repo_query`](crate::intelligence::query::build_repo_query) call
//!   holds). This state is observed, never inferred: nothing claims a rebuild
//!   that is not in flight, and the state disappears the moment the scan
//!   ends — including when it ends in an error.
//!
//! The probe itself never builds and never writes: it measures the current
//! fingerprint through the stat memo (reading only files whose stat facts
//! drifted) and compares against the cache header. See
//! [`PersistedIndex::freshness`](crate::intelligence::persisted::PersistedIndex::freshness)
//! for the measurement rules.

use crate::intelligence::persisted::PersistedIndex;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The observation surface's state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoContextState {
    /// Measured fingerprint equals the indexed one.
    Fresh,
    /// The tree changed since the index was written (or no index exists).
    Stale,
    /// A full re-index for this root is running in this process.
    Reindexing,
}

impl RepoContextState {
    /// The wire spelling of the state (also the payload's `state` field).
    pub fn as_str(&self) -> &'static str {
        match self {
            RepoContextState::Fresh => "fresh",
            RepoContextState::Stale => "stale",
            RepoContextState::Reindexing => "reindexing",
        }
    }
}

/// One freshness observation of `root`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoContextStatus {
    /// The resolved state (see [`RepoContextState`]).
    pub state: RepoContextState,
    /// The fingerprint measured right now.
    pub fingerprint: String,
    /// The fingerprint the stored index was built at — `None` when no
    /// trusted index exists.
    pub indexed_fingerprint: Option<String>,
    /// Files whose content hash differs from the stored measurement —
    /// `None` when there is no trusted index to compare against. Always
    /// `Some(0)` when the state is fresh: a content change cannot be.
    pub changed_files: Option<usize>,
}

/// Short (12-hex) spelling of a full SHA-256 fingerprint, for display.
pub fn short_fingerprint(full: &str) -> String {
    full.chars().take(12).collect()
}

impl RepoContextStatus {
    /// The JSON payload every surface renders: state plus *short* hashes.
    /// Absences stay explicit — `indexed_fingerprint` and `changed_files`
    /// are `null` (never an invented hash or a fabricated zero) when no
    /// trusted index exists.
    pub fn payload(&self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state.as_str(),
            "fingerprint": short_fingerprint(&self.fingerprint),
            "indexed_fingerprint": self.indexed_fingerprint.as_deref().map(short_fingerprint),
            "changed_files": self.changed_files,
        })
    }
}

/// Probe `root` once: cheap (stat memo — drifted files only), read-only,
/// and honest about a missing root or an unbuilt index.
pub fn probe(root: &Path) -> Result<RepoContextStatus> {
    let measured = PersistedIndex::new(root).freshness()?;
    let state = if is_reindexing(root) {
        RepoContextState::Reindexing
    } else if measured.stored.as_deref() == Some(measured.measured.as_str()) {
        RepoContextState::Fresh
    } else {
        RepoContextState::Stale
    };
    Ok(RepoContextStatus {
        state,
        fingerprint: measured.measured,
        indexed_fingerprint: measured.stored,
        changed_files: measured.changed_files,
    })
}

// ---------------------------------------------------------------------------
// The reindex registry
//
// A counter per root (nested/concurrent scans for one root all count), keyed
// on the canonicalized path so two spellings of the same repository share one
// entry. The lock is a plain process-wide mutex: scans take it only on
// start/end, and a poisoned lock degrades to the surviving map rather than
// panicking a build mid-scan.
// ---------------------------------------------------------------------------

static REINDEXING: Mutex<Option<HashMap<PathBuf, u32>>> = Mutex::new(None);

fn registry_key(root: &Path) -> PathBuf {
    std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())
}

fn lock_registry() -> std::sync::MutexGuard<'static, Option<HashMap<PathBuf, u32>>> {
    REINDEXING.lock().unwrap_or_else(|e| e.into_inner())
}

fn is_reindexing(root: &Path) -> bool {
    lock_registry()
        .as_ref()
        .map(|m| m.contains_key(&registry_key(root)))
        .unwrap_or(false)
}

/// Marks `root` as reindexing for its own lifetime. Held across every
/// `build_repo_query` call, so any surface observing the same root — agent
/// gather, `repo-context` CLI, serve-intel stream — sees `reindexing` for
/// exactly as long as the scan runs, then never again. Dropping the guard
/// (including through `?` returns or a panic) unregisters it.
pub struct ReindexGuard {
    key: PathBuf,
}

impl ReindexGuard {
    pub(crate) fn new(root: &Path) -> Self {
        let key = registry_key(root);
        let mut guard = lock_registry();
        let map = guard.get_or_insert_with(HashMap::new);
        *map.entry(key.clone()).or_insert(0) += 1;
        drop(guard);
        Self { key }
    }
}

impl Drop for ReindexGuard {
    fn drop(&mut self) {
        let mut guard = lock_registry();
        let Some(map) = guard.as_mut() else {
            return;
        };
        match map.get_mut(&self.key) {
            Some(n) if *n > 1 => *n -= 1,
            Some(_) => {
                map.remove(&self.key);
            }
            None => {}
        }
        if map.is_empty() {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn temp_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "pub fn alpha() {}\n").unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        dir
    }

    fn build(root: &Path) {
        PersistedIndex::new(root).build().expect("index builds");
    }

    #[test]
    fn before_any_index_there_is_nothing_to_call_fresh() {
        let dir = temp_repo();
        let status = probe(dir.path()).expect("probe runs");
        assert_eq!(status.state, RepoContextState::Stale);
        assert_eq!(status.indexed_fingerprint, None, "no index, no baseline");
        assert_eq!(status.changed_files, None, "no baseline, no 'changed'");
        assert_eq!(status.payload()["state"], "stale");
        assert!(
            status.payload()["indexed_fingerprint"].is_null(),
            "an absent index must serialize as null, never an invented hash: {}",
            status.payload()
        );
    }

    #[test]
    fn build_then_probe_is_fresh_with_zero_changes() {
        let dir = temp_repo();
        build(dir.path());
        let status = probe(dir.path()).expect("probe runs");
        assert_eq!(status.state, RepoContextState::Fresh, "{status:?}");
        assert_eq!(status.changed_files, Some(0), "fresh means no drift");
        let indexed = status.indexed_fingerprint.clone().expect("index exists");
        assert_eq!(status.fingerprint, indexed, "the two hashes must agree");
        assert_eq!(status.payload()["fingerprint"].as_str().unwrap().len(), 12);
    }

    #[test]
    fn a_content_change_turns_it_stale_and_counts_the_change() {
        let dir = temp_repo();
        build(dir.path());
        fs::write(
            dir.path().join("src/lib.rs"),
            "pub fn alpha() {}\npub fn beta() {}\n",
        )
        .unwrap();

        let status = probe(dir.path()).expect("probe runs");
        assert_eq!(status.state, RepoContextState::Stale, "{status:?}");
        assert_ne!(
            status.indexed_fingerprint.as_deref(),
            Some(status.fingerprint.as_str()),
            "the tree really changed, so the hashes must differ"
        );
        assert_eq!(status.changed_files, Some(1), "{status:?}");

        // …and a rebuild brings it back, with the new fingerprint stored.
        build(dir.path());
        let status = probe(dir.path()).expect("probe runs");
        assert_eq!(status.state, RepoContextState::Fresh, "{status:?}");
        assert_eq!(status.changed_files, Some(0));
    }

    #[test]
    fn an_mtime_only_touch_is_not_a_change() {
        let dir = temp_repo();
        build(dir.path());

        let f = dir.path().join("src/lib.rs");
        let file = fs::OpenOptions::new().append(true).open(&f).unwrap();
        file.set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(120)),
        )
        .unwrap();
        drop(file);

        let status = probe(dir.path()).expect("probe runs");
        assert_eq!(
            status.state,
            RepoContextState::Fresh,
            "a touch without content change must not report staleness: {status:?}"
        );
        assert_eq!(status.changed_files, Some(0), "{status:?}");
    }

    #[test]
    fn a_scan_in_flight_is_observed_as_reindexing_and_ends() {
        let dir = temp_repo();
        build(dir.path());

        {
            let _guard = ReindexGuard::new(dir.path());
            let status = probe(dir.path()).expect("probe runs");
            assert_eq!(status.state, RepoContextState::Reindexing, "{status:?}");
        }

        let status = probe(dir.path()).expect("probe runs");
        assert_eq!(
            status.state,
            RepoContextState::Fresh,
            "the guard's drop must unregister the scan: {status:?}"
        );
    }

    #[test]
    fn probes_of_different_roots_do_not_share_state() {
        let dir = temp_repo();
        let other = temp_repo();
        build(dir.path());
        build(other.path());

        let _guard = ReindexGuard::new(dir.path());
        assert_eq!(
            probe(dir.path()).unwrap().state,
            RepoContextState::Reindexing
        );
        assert_eq!(probe(other.path()).unwrap().state, RepoContextState::Fresh);
    }

    #[test]
    fn probing_a_missing_root_is_an_error_not_a_state() {
        let missing = std::env::temp_dir().join(format!(
            "freshness-missing-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(probe(&missing).is_err(), "a missing root must fail closed");
    }
}
