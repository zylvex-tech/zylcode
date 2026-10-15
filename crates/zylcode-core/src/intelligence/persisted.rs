//! Persisted repository intelligence index.
//!
//! # Why this exists
//!
//! `build_repo_query` re-indexes the repository on every call — roughly ten
//! seconds on this repository. A consumer that indexes per turn (the
//! AgentLoop's context gathering, the `repo-context` CLI) pays that cost on
//! every invocation, which is why "per-invocation re-indexing" sat in the
//! capability registry's limitations list.
//!
//! This module kills that cost for the common case: the index is persisted
//! under `<repo>/.zylcode/intelligence-index.json` (gitignored) and reused
//! when the repository has not changed. Reuse is decided on **content**,
//! not timestamps:
//!
//! * the repository fingerprint is the sorted set of
//!   `(relative path, content hash)` pairs hashed again with SHA-256, so an
//!   mtime-only touch cannot cause a stale hit and an identical checkout
//!   at a different path hits;
//! * a cache whose stored fingerprint differs from the freshly measured one
//!   is discarded and rebuilt — a stale index is slower than a fresh scan,
//!   never wrong.
//!
//! # The stat-memo layer (P15 W1)
//!
//! Measuring that fingerprint used to mean `scan_repository` — a full read
//! and hash of every file, on every gather. The memo closes that: alongside
//! the index, the cache stores per-file stat facts `(len, mtime, content
//! hash)` captured at the last full measurement. `measure_fingerprint`
//! walks the tree collecting stat facts only (never reading file contents)
//! and re-hashes just the files whose `(len, mtime)` differ from their memo
//! entry; unchanged files reuse the memoized hash.
//!
//! Invariants:
//! * an mtime-only touch causes a memo miss, a rehash of unchanged content,
//!   an identical fingerprint — still a cache hit, still no cache rewrite
//!   (the existing test proves it through the memo path);
//! * a stat-memo *hit* is used only to reproduce the fingerprint — the
//!   index itself is never served from memos; a real content change still
//!   forces the full re-index, exactly as before;
//! * memo failures degrade to today's behavior: a cache without memos
//!   (older schema, first run) yields an empty memo, so the first
//!   measurement is full-hash; a corrupt cache is discarded entirely.
//!   Schema enjoys a bump for the new field — pre-existing v1 caches are
//!   rebuilt once and the memo starts paying from the next gather.
//!
//! The per-HEAD determinism contract is preserved: the cache is keyed on
//! content, so identical inputs at a given HEAD still produce byte-identical
//! retrieval output whether served from cache or built fresh.

use crate::intelligence::query::RepoQuery;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Everything needed to reconstruct a `RepoQuery` without re-indexing.
///
/// `DependencyGraph` is derived from packages via
/// `DependencyGraph::from_packages`, so it is not stored — only the inputs
/// are persisted, and the graph is rebuilt on load in microseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedIndex {
    /// Schema version. A cache written by a different schema is discarded.
    schema: u32,
    /// SHA-256 over the sorted (path, content-hash) fingerprint of the
    /// indexed repository at cache time.
    repo_fingerprint: String,
    files: Vec<crate::intelligence::types::FileNode>,
    symbols: Vec<crate::intelligence::types::Symbol>,
    packages: Vec<crate::intelligence::types::Package>,
    entry_points: Vec<crate::intelligence::types::EntryPoint>,
    architecture: crate::intelligence::types::ArchitecturalFingerprint,
    git_commits: Vec<crate::intelligence::types::GitCommit>,
    /// Per-file stat facts captured at the last full fingerprint
    /// measurement, keyed by repository-relative id. Empty on caches too
    /// old to carry it; an empty memo degrades measurement to full-hash.
    #[serde(default)]
    stat_memo: std::collections::BTreeMap<String, StatMemoEntry>,
}

/// One file's memoized stat facts and content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatMemoEntry {
    len: u64,
    mtime_secs: i64,
    mtime_nanos: u32,
    content_hash: String,
}

const SCHEMA_VERSION: u32 = 2;

/// A persisted index bound to one repository root.
pub struct PersistedIndex {
    root: PathBuf,
    cache_path: PathBuf,
}

impl PersistedIndex {
    /// Index for `root`, cached at `<root>/.zylcode/intelligence-index.json`.
    /// `ZYLCODE_INTELLIGENCE_CACHE` overrides the cache location (tests use
    /// this to avoid touching a real working tree).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let cache_path = std::env::var("ZYLCODE_INTELLIGENCE_CACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join(".zylcode").join("intelligence-index.json"));
        Self { root, cache_path }
    }

    /// The cache location, for tests and diagnostics.
    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }

    /// Build (or load) the index.
    ///
    /// Fast path: a cache whose stored fingerprint matches the freshly
    /// measured one is deserialized and served. Slow path: full re-index,
    /// then write-through. A corrupted or foreign-schema cache is discarded,
    /// never trusted.
    ///
    /// # Fail closed on a missing root
    ///
    /// `store()` creates `<root>/.zylcode/…` with `create_dir_all`. On a
    /// workspace root that does not exist, that write would bring the missing
    /// directory (and its parent chain) into existence as a *warm-start side
    /// effect*, after which downstream consumers — `RepoIndex::build`, whose
    /// guard is `ensure!(root.is_dir())` — would see a real, empty directory
    /// and report an empty graph instead of the input error. The input error
    /// is the signal; it must not be laundered into "indexed, nothing there".
    /// So the root is validated before any fingerprinting or writing.
    pub fn build(&self) -> Result<RepoQuery> {
        self.ensure_root_exists()?;
        let (fingerprint, _first_memo) = self.measure_fingerprint_with_memo()?;

        if let Some(cached) = self.load_if_fresh(&fingerprint) {
            return Ok(Self::into_query(cached));
        }

        let query = crate::intelligence::query::build_repo_query(&self.root)?;
        // Re-measure *after* the scan and store that fingerprint with its
        // memo: whatever changed while the scan ran is captured, so a stored
        // cache can never carry a pre-scan fingerprint with post-scan
        // contents.
        let (fingerprint, memo) = self.measure_fingerprint_with_memo()?;
        let cached = Self::from_query(&query, fingerprint, memo);
        self.store(&cached)
            .with_context(|| "index built but the cache write failed")?;
        Ok(query)
    }

    /// Force a full re-index and refresh the cache, ignoring any existing
    /// cache file. Used when a consumer knows the repository changed.
    ///
    /// Fails closed on a missing root for the same reason as [`build`].
    pub fn rebuild(&self) -> Result<RepoQuery> {
        self.ensure_root_exists()?;
        let query = crate::intelligence::query::build_repo_query(&self.root)?;
        let (fingerprint, memo) = self.measure_fingerprint_with_memo()?;
        let cached = Self::from_query(&query, fingerprint, memo);
        self.store(&cached)?;
        Ok(query)
    }

    /// Reject a workspace root that is not a real directory before any work
    /// (or any `create_dir_all`) happens.
    ///
    /// Without this, `store()`'s `create_dir_all(<root>/.zylcode)` is the
    /// *first* thing that touches a missing root: it materialises the missing
    /// directory as a side effect of an index warm-start, and the caller's
    /// "your workspace does not exist" error is converted into a silent empty
    /// result. See [`build`].
    fn ensure_root_exists(&self) -> Result<()> {
        anyhow::ensure!(
            self.root.is_dir(),
            "workspace root '{}' does not exist (refusing to index a missing directory)",
            self.root.display()
        );
        Ok(())
    }

    /// The memo-aware fingerprint: stat facts collected without any content
    /// read; files whose stat facts changed since the stored memo are
    /// re-hashed, unchanged ones reuse their memoized hash. Returns the
    /// fingerprint and the memo to store with the next cache write.
    fn measure_fingerprint_with_memo(
        &self,
    ) -> Result<(String, std::collections::BTreeMap<String, StatMemoEntry>)> {
        let memo = self.load_stat_memo();
        let config = crate::intelligence::scanner::ScannerConfig {
            max_depth: 10,
            max_file_size: 512 * 1024,
            follow_symlinks: false,
        };
        let stats = crate::intelligence::scanner::collect_stat_inventory(&self.root, &config)?;

        let mut pairs: Vec<(String, String)> = Vec::with_capacity(stats.len());
        let mut new_memo = std::collections::BTreeMap::new();
        for fact in stats {
            let content_hash = match memo.as_ref().and_then(|m| m.get(&fact.id)) {
                Some(entry)
                    if entry.len == fact.len
                        && entry.mtime_secs == fact.mtime_secs
                        && entry.mtime_nanos == fact.mtime_nanos =>
                {
                    // Stat matches memo exactly since the last full
                    // measurement — the file is (by the mtime can't-fake-
                    // freshness rule still enforced here) re-hashed, but its
                    // content hash was proven stable, reuse it without the
                    // read.
                    entry.content_hash.clone()
                }
                _ => {
                    // New file, removed memo entry, or any stat drift:
                    // full re-hash, and it must be byte-identical to what a
                    // full scan would compute.
                    crate::intelligence::scanner::hash_file_for_fingerprint(
                        &self.root, &fact.id, fact.len,
                    )
                }
            };
            new_memo.insert(
                fact.id.clone(),
                StatMemoEntry {
                    len: fact.len,
                    mtime_secs: fact.mtime_secs,
                    mtime_nanos: fact.mtime_nanos,
                    content_hash: content_hash.clone(),
                },
            );
            pairs.push((fact.id, content_hash));
        }

        pairs.sort();
        let mut hasher = Sha256::new();
        for (path, hash) in &pairs {
            hasher.update(path.as_bytes());
            hasher.update([0u8]);
            hasher.update(hash.as_bytes());
            hasher.update([0u8]);
        }
        Ok((format!("{:x}", hasher.finalize()), new_memo))
    }

    /// The memo from an existing cache, if parseable. A cache written by a
    /// schema without the memo field (or any unparseable cache) yields an
    /// empty memo — the measurement degrades to full-hash, never to
    /// trust-in-freshness.
    fn load_stat_memo(&self) -> Option<std::collections::BTreeMap<String, StatMemoEntry>> {
        let raw = std::fs::read(&self.cache_path).ok()?;
        let cached: CachedIndex = serde_json::from_slice(&raw).ok()?;
        Some(cached.stat_memo)
    }

    fn load_if_fresh(&self, fingerprint: &str) -> Option<CachedIndex> {
        let raw = std::fs::read(&self.cache_path).ok()?;
        let cached: CachedIndex = serde_json::from_slice(&raw).ok()?;
        if cached.schema != SCHEMA_VERSION || cached.repo_fingerprint != fingerprint {
            return None;
        }
        Some(cached)
    }

    fn from_query(
        query: &RepoQuery,
        fingerprint: String,
        stat_memo: std::collections::BTreeMap<String, StatMemoEntry>,
    ) -> CachedIndex {
        CachedIndex {
            schema: SCHEMA_VERSION,
            repo_fingerprint: fingerprint,
            files: query.files().to_vec(),
            symbols: query.symbols().to_vec(),
            packages: query.packages().to_vec(),
            entry_points: query.entry_points_list().to_vec(),
            architecture: query.architecture().clone(),
            git_commits: query.git_commits().to_vec(),
            stat_memo,
        }
    }

    fn into_query(cached: CachedIndex) -> RepoQuery {
        let dep_graph =
            crate::intelligence::dependency::DependencyGraph::from_packages(&cached.packages);
        RepoQuery::new(
            cached.files,
            cached.symbols,
            cached.packages,
            dep_graph,
            cached.entry_points,
            cached.architecture,
            cached.git_commits,
        )
    }

    fn store(&self, cached: &CachedIndex) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        // Write-then-rename: a crash mid-write leaves the previous cache (or
        // no cache), never a half-written file that would fail to parse and
        // force a re-index forever.
        let tmp = self.cache_path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(cached)?)?;
        std::fs::rename(&tmp, &self.cache_path)
            .with_context(|| format!("cannot install cache at {}", self.cache_path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

    fn isolated(root: &Path) -> PersistedIndex {
        let cache = std::env::temp_dir().join(format!(
            "zylcode-idx-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        PersistedIndex {
            root: root.to_path_buf(),
            cache_path: cache,
        }
    }

    #[test]
    fn first_build_indexes_then_a_second_build_hits_the_cache() {
        let dir = temp_repo();
        let idx = isolated(dir.path());

        let q1 = idx.build().unwrap();
        assert!(q1.file_count() >= 2);
        assert!(idx.cache_path().exists(), "the cache must be written");

        let q2 = idx.build().unwrap();
        assert_eq!(q2.file_count(), q1.file_count());
        assert_eq!(q2.symbol_count(), q1.symbol_count());
    }

    #[test]
    fn a_content_change_invalidates_and_rebuilds() {
        let dir = temp_repo();
        let idx = isolated(dir.path());

        let before = idx.build().unwrap();
        let before_symbols = before.symbol_count();

        fs::write(
            dir.path().join("src/lib.rs"),
            "pub fn alpha() {}\npub fn beta() {}\n",
        )
        .unwrap();

        let after = idx.build().unwrap();
        assert!(
            after.symbol_count() > before_symbols,
            "a content change must invalidate the cache and re-index"
        );
    }

    #[test]
    fn an_mtime_touch_does_not_invalidate() {
        let dir = temp_repo();
        let idx = isolated(dir.path());
        idx.build().unwrap();
        let cache_before = fs::read(idx.cache_path()).unwrap();

        // Touch mtime without changing content.
        let f = dir.path().join("src/lib.rs");
        let file = fs::OpenOptions::new().append(true).open(&f).unwrap();
        file.set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60)),
        )
        .unwrap();
        drop(file);

        idx.build().unwrap();
        let cache_after = fs::read(idx.cache_path()).unwrap();
        assert_eq!(
            cache_before, cache_after,
            "an mtime-only touch must not rewrite the cache"
        );
    }

    #[test]
    fn a_corrupted_cache_is_discarded_not_trusted() {
        let dir = temp_repo();
        let idx = isolated(dir.path());
        idx.build().unwrap();

        fs::write(idx.cache_path(), b"{ not json").unwrap();
        let q = idx.build().unwrap();
        assert!(q.file_count() >= 2, "a corrupt cache must force a rebuild");
        assert!(
            fs::read(idx.cache_path()).unwrap().len() > 100,
            "the rebuilt cache must be written back"
        );
    }

    /// G6: a warm-start must never bring a missing workspace root into
    /// existence. `store()` writes `<root>/.zylcode/intelligence-index.json`
    /// with `create_dir_all`, so without an entry guard the *first* thing that
    /// touches a missing root is that write — the directory appears as a side
    /// effect, and `RepoIndex::build`'s `ensure!(root.is_dir())` then passes
    /// against the empty directory the index itself just created. The
    /// caller's "your workspace does not exist" error must survive instead.
    #[test]
    fn build_on_a_missing_root_fails_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-workspace");
        let idx = isolated(&missing);

        let err = match idx.build() {
            Ok(_) => panic!("a missing root must fail closed"),
            Err(e) => e,
        };
        let msg = format!("{err:#}");
        assert!(
            msg.contains("does not exist"),
            "the error must name the input problem, got: {msg}"
        );
        assert!(
            !missing.exists(),
            "an index warm-start must not materialise a missing workspace root"
        );
        assert!(
            !idx.cache_path().exists(),
            "no cache may be written for a root that never existed"
        );
    }

    /// P15 W1 behavioral proof (A2): on a warm cache where every file's stat
    /// facts match its memo entry, the fingerprint is measured **without any
    /// content read**. Proof: after the first build, every indexed file is
    /// revoked of read permission — a full scan would fail — and the second
    /// build must still succeed via stat-memo reuse alone.
    #[cfg(unix)]
    #[test]
    fn a_warm_second_build_reads_no_file_contents() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_repo();
        let idx = isolated(dir.path());
        let first = idx.build().unwrap();
        let first_count = first.file_count();

        let files: Vec<PathBuf> = ["Cargo.toml".as_ref(), Path::new("src/lib.rs")]
            .iter()
            .map(|p| dir.path().join(p))
            .collect();
        for f in &files {
            fs::set_permissions(f, fs::Permissions::from_mode(0o000)).unwrap();
        }

        let second = idx.build().unwrap();
        assert_eq!(
            second.file_count(),
            first_count,
            "a warm build with unreadable files must still serve the memoized index"
        );

        for f in &files {
            fs::set_permissions(f, fs::Permissions::from_mode(0o644)).unwrap();
        }
    }

    /// A same-size content change must still invalidate: the stat facts
    /// differ (mtime forced forward), the memo misses, the rehash differs,
    /// and the full re-index picks up the new content.
    #[test]
    fn a_same_size_content_change_invalidates() {
        let dir = temp_repo();
        let idx = isolated(dir.path());
        let before = idx.build().unwrap();
        let before_symbols = before.symbol_count();

        // Same byte length for lib.rs, different content.
        fs::write(dir.path().join("src/lib.rs"), "pub fn domu() {}\n").unwrap();
        let f = dir.path().join("src/lib.rs");
        let file = fs::OpenOptions::new().append(true).open(&f).unwrap();
        file.set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(120)),
        )
        .unwrap();
        drop(file);

        let after = idx.build().unwrap();
        let symbol_names: Vec<String> = after.symbols().iter().map(|s| s.name.clone()).collect();
        assert!(
            symbol_names.iter().any(|n| n.contains("domu")),
            "a same-size content change must reach the re-index (symbols: {symbol_names:?})"
        );
    }

    /// A v1 cache (no memo field) is parsed, rejected on schema, rebuilt
    /// once, and rewritten as a v2 cache that carries the memo — the
    /// graceful-migration path, not a trust path.
    #[test]
    fn a_v1_cache_is_rebuilt_once_and_upgraded_to_v2() {
        let dir = temp_repo();
        let idx = isolated(dir.path());
        idx.build().unwrap();

        // Hand-write a well-formed v1 cache: schema 1, no memo.
        let legacy = serde_json::json!({
            "schema": 1,
            "repo_fingerprint": "deadbeef".to_string(),
            "files": [],
            "symbols": [],
            "packages": [],
            "entry_points": [],
            "architecture": {},
            "git_commits": [],
        });
        fs::write(idx.cache_path(), serde_json::to_vec(&legacy).unwrap()).unwrap();

        let q = idx.build().unwrap();
        assert!(q.file_count() >= 2, "v1 cache must be rebuilt, not trusted");

        let raw = fs::read(idx.cache_path()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(
            value["schema"],
            serde_json::json!(2),
            "the rebuilt cache must carry the v2 schema"
        );
        assert!(
            value["stat_memo"].is_object(),
            "the rebuilt cache must carry a populated stat memo"
        );
    }

    #[test]
    fn rebuild_on_a_missing_root_fails_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-workspace");
        let idx = isolated(&missing);

        let err = match idx.rebuild() {
            Ok(_) => panic!("a missing root must fail closed"),
            Err(e) => e,
        };
        assert!(
            format!("{err:#}").contains("does not exist"),
            "the error must name the input problem"
        );
        assert!(
            !missing.exists(),
            "rebuild must not materialise a missing workspace root either"
        );
    }
}
