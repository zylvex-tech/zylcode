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
}

const SCHEMA_VERSION: u32 = 1;

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
    pub fn build(&self) -> Result<RepoQuery> {
        let fingerprint = self.measure_fingerprint()?;

        if let Some(cached) = self.load_if_fresh(&fingerprint) {
            return Ok(Self::into_query(cached));
        }

        let query = crate::intelligence::query::build_repo_query(&self.root)?;
        let cached = Self::from_query(&query, fingerprint);
        self.store(&cached)
            .with_context(|| "index built but the cache write failed")?;
        Ok(query)
    }

    /// Force a full re-index and refresh the cache, ignoring any existing
    /// cache file. Used when a consumer knows the repository changed.
    pub fn rebuild(&self) -> Result<RepoQuery> {
        let query = crate::intelligence::query::build_repo_query(&self.root)?;
        let fingerprint = self.measure_fingerprint()?;
        let cached = Self::from_query(&query, fingerprint);
        self.store(&cached)?;
        Ok(query)
    }

    /// SHA-256 over the sorted (relative path, content hash) inventory.
    ///
    /// Sorted so the fingerprint is independent of the walker's traversal
    /// order; content hashes so mtimes cannot fake freshness or invalidate
    /// real freshness.
    fn measure_fingerprint(&self) -> Result<String> {
        let config = crate::intelligence::scanner::ScannerConfig {
            max_depth: 10,
            max_file_size: 512 * 1024,
            follow_symlinks: false,
        };
        let scan = crate::intelligence::scanner::scan_repository(&self.root, &config)?;
        let mut pairs: Vec<(String, String)> = scan
            .files
            .iter()
            .map(|f| (f.id.clone(), f.content_hash.clone()))
            .collect();
        pairs.sort();
        let mut hasher = Sha256::new();
        for (path, hash) in &pairs {
            hasher.update(path.as_bytes());
            hasher.update([0u8]);
            hasher.update(hash.as_bytes());
            hasher.update([0u8]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn load_if_fresh(&self, fingerprint: &str) -> Option<CachedIndex> {
        let raw = std::fs::read(&self.cache_path).ok()?;
        let cached: CachedIndex = serde_json::from_slice(&raw).ok()?;
        if cached.schema != SCHEMA_VERSION || cached.repo_fingerprint != fingerprint {
            return None;
        }
        Some(cached)
    }

    fn from_query(query: &RepoQuery, fingerprint: String) -> CachedIndex {
        CachedIndex {
            schema: SCHEMA_VERSION,
            repo_fingerprint: fingerprint,
            files: query.files().to_vec(),
            symbols: query.symbols().to_vec(),
            packages: query.packages().to_vec(),
            entry_points: query.entry_points_list().to_vec(),
            architecture: query.architecture().clone(),
            git_commits: query.git_commits().to_vec(),
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

        fs::write(dir.path().join("src/lib.rs"), "pub fn alpha() {}\npub fn beta() {}\n")
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
        file.set_times(std::fs::FileTimes::new().set_modified(
            std::time::SystemTime::now() + std::time::Duration::from_secs(60),
        ))
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
}
