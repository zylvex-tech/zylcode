//! Speculative cache — LRU + TTL, keyed by hash(prompt,system,model).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lru::LruCache;

#[derive(Debug, Clone)]
struct CachedEntry {
    response: String,
    inserted_at: Instant,
}

#[derive(Debug)]
pub struct SpeculativeCache {
    inner: Mutex<LruCache<u64, CachedEntry>>,
    ttl: Duration,
}

impl SpeculativeCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: Mutex::new(LruCache::new(cap)),
            ttl,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(128, Duration::from_secs(600))
    }

    #[inline]
    pub fn hash_key(prompt: &str, system: &str, model: &str) -> u64 {
        // ahash is fast and DoS-resistant; fallback to DefaultHasher keeps
        // zero extra deps at call-site, but we use ahash::AHasher for speed.
        let mut hasher = ahash::AHasher::default();
        prompt.hash(&mut hasher);
        hasher.write_u8(0x1F);
        system.hash(&mut hasher);
        hasher.write_u8(0x1F);
        model.hash(&mut hasher);
        hasher.finish()
    }

    // Keep compatibility helper using DefaultHasher for callers that pass raw bytes
    #[allow(dead_code)]
    pub fn hash_bytes(bytes: &[u8]) -> u64 {
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        h.finish()
    }

    pub fn get(&self, key: u64) -> Option<String> {
        let mut guard = self.inner.lock().ok()?;
        let entry = guard.get(&key)?;
        if entry.inserted_at.elapsed() > self.ttl {
            guard.pop(&key);
            return None;
        }
        Some(entry.response.clone())
    }

    pub fn insert(&self, key: u64, response: String) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.put(
                key,
                CachedEntry {
                    response,
                    inserted_at: Instant::now(),
                },
            );
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.clear();
        }
    }
}

impl Default for SpeculativeCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_insert_and_hit() {
        let c = SpeculativeCache::new(4, Duration::from_secs(60));
        let k = SpeculativeCache::hash_key("p", "s", "m");
        assert!(c.get(k).is_none());
        c.insert(k, "hello".to_string());
        assert_eq!(c.get(k).as_deref(), Some("hello"));
    }
    #[test]
    fn cache_ttl_expiry() {
        let c = SpeculativeCache::new(4, Duration::from_millis(10));
        let k = SpeculativeCache::hash_key("p", "s", "m");
        c.insert(k, "v".to_string());
        std::thread::sleep(Duration::from_millis(20));
        assert!(c.get(k).is_none());
    }
    #[test]
    fn lru_evicts_oldest() {
        let c = SpeculativeCache::new(2, Duration::from_secs(60));
        let k1 = SpeculativeCache::hash_key("a", "s", "m");
        let k2 = SpeculativeCache::hash_key("b", "s", "m");
        let k3 = SpeculativeCache::hash_key("c", "s", "m");
        c.insert(k1, "1".into());
        c.insert(k2, "2".into());
        c.insert(k3, "3".into());
        assert!(c.get(k1).is_none());
        assert!(c.get(k2).is_some());
    }
}
