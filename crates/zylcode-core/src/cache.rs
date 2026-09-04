//! Phase 8.2 — Local Model Cache & Vector Indexing Engine.
//!
//! Lightweight SQLite-backed vector cache with cosine-similarity retrieval.
//! Offline-first: prompt vectors are hashed deterministically when no remote
//! embedding provider is configured.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// VectorEntry
// ---------------------------------------------------------------------------

/// Persisted cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: i64,
    pub prompt_hash: String,
    pub prompt_text: String,
    pub response_text: String,
    pub embedding_vector: Vec<f32>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Cosine similarity
// ---------------------------------------------------------------------------

/// Compute cosine similarity between two vectors (≈ dot over magnitudes).
/// Returns 0.0 if either vector is zero-magnitude. Result in [-1, 1].
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut mag_a = 0.0_f64;
    let mut mag_b = 0.0_f64;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += f64::from(*x) * f64::from(*y);
        mag_a += f64::from(*x) * f64::from(*x);
        mag_b += f64::from(*y) * f64::from(*y);
    }
    let mag_a = mag_a.sqrt();
    let mag_b = mag_b.sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    (dot / (mag_a * mag_b)) as f32
}

/// Deterministic mock embedding — hashes prompt into fixed dim unit vector.
/// Used when remote embedding provider is offline.
pub fn mock_embed(text: &str, dim: usize) -> Vec<f32> {
    let dim = dim.max(8);
    let mut vec = vec![0.0f32; dim];
    if text.is_empty() {
        return vec;
    }
    for (i, ch) in text.chars().enumerate() {
        let idx = i % dim;
        // Mix char code + position
        let v = (ch as u32 as f32 * 0.01) + (i as f32 * 0.001);
        vec[idx] += v;
    }
    // Add bigram signal
    for (i, pair) in text.as_bytes().windows(2).enumerate() {
        let idx = (i + dim / 2) % dim;
        vec[idx] += (pair[0] as f32 + pair[1] as f32) * 0.005;
    }
    // L2 normalize
    let mag = vec.iter().map(|x| f64::from(*x) * f64::from(*x)).sum::<f64>().sqrt() as f32;
    if mag > 0.0 {
        for v in &mut vec {
            *v /= mag;
        }
    }
    vec
}

/// SHA-256 hex of prompt text.
pub fn prompt_hash(prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    hex::encode(hasher.finalize())
}

// Minimal hex helper to avoid extra dep — inline
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let b = bytes.as_ref();
        let mut s = String::with_capacity(b.len() * 2);
        for byte in b {
            s.push(char::from_digit(u32::from(byte >> 4), 16).unwrap());
            s.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap());
        }
        s
    }
}

// ---------------------------------------------------------------------------
// VectorCacheStore — SQLite BLOB storage
// ---------------------------------------------------------------------------

/// SQLite-backed vector cache.
#[derive(Debug, Clone)]
pub struct VectorCacheStore {
    db_path: PathBuf,
}

impl VectorCacheStore {
    /// Open or create SQLite DB at `path`, ensuring table exists.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let db_path = path.into();
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        let store = Self { db_path };
        store.init_table()?;
        Ok(store)
    }

    /// Default path resolution: `ZYLCODE_VECTOR_CACHE_PATH` → `~/.zylcode/vector_cache.db` → `./vector_cache.db`
    pub fn with_default_path() -> Result<Self> {
        let p = std::env::var("ZYLCODE_VECTOR_CACHE_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| Path::new(&h).join(".zylcode/vector_cache.db"))
            })
            .unwrap_or_else(|| PathBuf::from("vector_cache.db"));
        Self::new(p)
    }

    fn connect(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("open vector_cache at {:?}", self.db_path))
    }

    fn init_table(&self) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vector_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prompt_hash TEXT NOT NULL UNIQUE,
                prompt_text TEXT NOT NULL,
                response_text TEXT NOT NULL,
                embedding_blob BLOB NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            )",
            [],
        )
        .context("create vector_cache table")?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_vector_cache_prompt_hash ON vector_cache(prompt_hash)",
            [],
        )
        .ok();
        Ok(())
    }

    fn serialize_embedding(v: &[f32]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(v.len() * 4);
        for f in v {
            buf.extend_from_slice(&f.to_le_bytes());
        }
        buf
    }

    fn deserialize_embedding(blob: &[u8]) -> Vec<f32> {
        blob.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    /// Insert or replace entry.
    pub fn insert_entry(&self, prompt: &str, response: &str, embedding: &[f32]) -> Result<()> {
        let hash = prompt_hash(prompt);
        let blob = Self::serialize_embedding(embedding);
        let conn = self.connect()?;
        conn.execute(
            "INSERT OR REPLACE INTO vector_cache (prompt_hash, prompt_text, response_text, embedding_blob) VALUES (?1, ?2, ?3, ?4)",
            params![hash, prompt, response, blob],
        )
        .context("insert vector_cache entry")?;
        Ok(())
    }

    /// Scan stored vectors and return top match if >= threshold.
    pub fn find_similar(&self, query_embedding: &[f32], threshold: f32) -> Result<Option<VectorEntry>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT id, prompt_hash, prompt_text, response_text, embedding_blob, created_at FROM vector_cache")
            .context("prepare vector scan")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .context("query vector_cache")?;

        let mut best: Option<(f32, VectorEntry)> = None;
        for row in rows {
            let (id, ph, pt, rt, blob, ca) = row.context("row decode")?;
            let vec = Self::deserialize_embedding(&blob);
            if vec.len() != query_embedding.len() {
                continue;
            }
            let score = cosine_similarity(query_embedding, &vec);
            if score >= threshold {
                let entry = VectorEntry {
                    id,
                    prompt_hash: ph,
                    prompt_text: pt,
                    response_text: rt,
                    embedding_vector: vec,
                    created_at: ca,
                };
                if best.as_ref().map_or(true, |(best_score, _)| score > *best_score) {
                    best = Some((score, entry));
                }
            }
        }
        Ok(best.map(|(_, e)| e))
    }

    /// Remove all entries.
    pub fn clear(&self) -> Result<usize> {
        let conn = self.connect()?;
        let n = conn
            .execute("DELETE FROM vector_cache", [])
            .context("clear vector_cache")?;
        Ok(n)
    }

    /// Stats: (entry count, estimated size bytes = sum(blob len + text len)).
    pub fn stats(&self) -> Result<(usize, usize)> {
        let conn = self.connect()?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM vector_cache", [], |r| r.get(0))
            .context("count vector_cache")?;
        let size: Option<i64> = conn
            .query_row(
                "SELECT COALESCE(SUM(length(embedding_blob) + length(prompt_text) + length(response_text)),0) FROM vector_cache",
                [],
                |r| r.get(0),
            )
            .ok();
        Ok((count as usize, size.unwrap_or(0) as usize))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn cosine_identical_is_one() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn cosine_opposite_is_minus_one() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_dim_mismatch_returns_zero() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0]), 0.0);
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn mock_embed_deterministic_and_normalized() {
        let a = mock_embed("hello world", 16);
        let b = mock_embed("hello world", 16);
        assert_eq!(a, b);
        let mag: f64 = a.iter().map(|x| f64::from(*x) * f64::from(*x)).sum::<f64>().sqrt();
        assert!((mag - 1.0).abs() < 1e-5);
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn sqlite_insert_and_find_similar() {
        let dir = tempdir().unwrap();
        let store = VectorCacheStore::new(dir.path().join("vec.db")).unwrap();
        let emb = mock_embed("build a todo app", 32);
        store.insert_entry("build a todo app", "<response>ok</response>", &emb).unwrap();
        let q = mock_embed("build a todo app", 32);
        let hit = store.find_similar(&q, 0.88).unwrap().expect("should hit");
        assert_eq!(hit.prompt_text, "build a todo app");
        assert_eq!(hit.response_text, "<response>ok</response>");
    }

    #[test]
    fn threshold_filters_low_similarity() {
        let dir = tempdir().unwrap();
        let store = VectorCacheStore::new(dir.path().join("vec.db")).unwrap();
        let emb = mock_embed("alpha prompt", 16);
        store.insert_entry("alpha prompt", "resp", &emb).unwrap();
        let q = mock_embed("completely different unrelated query xyz", 16);
        // high threshold should miss unless coincidentally similar
        let hit = store.find_similar(&q, 0.99).unwrap();
        assert!(hit.is_none() || cosine_similarity(&q, &emb) >= 0.99);
    }

    #[test]
    fn cache_miss_returns_none() {
        let dir = tempdir().unwrap();
        let store = VectorCacheStore::new(dir.path().join("vec.db")).unwrap();
        let q = mock_embed("nothing stored", 16);
        assert!(store.find_similar(&q, 0.88).unwrap().is_none());
    }

    #[test]
    fn clear_and_stats() {
        let dir = tempdir().unwrap();
        let store = VectorCacheStore::new(dir.path().join("vec.db")).unwrap();
        let e1 = mock_embed("p1", 8);
        let e2 = mock_embed("p2", 8);
        store.insert_entry("p1", "r1", &e1).unwrap();
        store.insert_entry("p2", "r2", &e2).unwrap();
        let (cnt, sz) = store.stats().unwrap();
        assert_eq!(cnt, 2);
        assert!(sz > 0);
        let n = store.clear().unwrap();
        assert_eq!(n, 2);
        let (cnt2, _) = store.stats().unwrap();
        assert_eq!(cnt2, 0);
    }

    #[test]
    fn prompt_hash_is_hex_and_stable() {
        let h1 = prompt_hash("hello");
        let h2 = prompt_hash("hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert_ne!(h1, prompt_hash("world"));
    }
}
