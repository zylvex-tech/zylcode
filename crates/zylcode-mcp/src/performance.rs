use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::enhanced_bridge::ToolDefinition;

/// Cached tool with expiration
#[derive(Debug, Clone)]
struct CachedTool {
    tool: ToolDefinition,
    cached_at: Instant,
    ttl: Duration,
}

impl CachedTool {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// Tool cache for improved performance
pub struct ToolCache {
    cache: RwLock<HashMap<String, CachedTool>>,
    default_ttl: Duration,
    max_size: usize,
}

impl ToolCache {
    /// Create a new tool cache
    pub fn new(default_ttl: Duration, max_size: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            default_ttl,
            max_size,
        }
    }

    /// Get a tool from cache
    pub async fn get(&self, tool_id: &str) -> Option<ToolDefinition> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.get(tool_id) {
            if !cached.is_expired() {
                return Some(cached.tool.clone());
            }
        }

        None
    }

    /// Put a tool in cache
    pub async fn put(&self, tool_id: &str, tool: ToolDefinition) {
        let mut cache = self.cache.write().await;

        // Check if cache is full
        if cache.len() >= self.max_size {
            // Remove expired entries first
            cache.retain(|_, cached| !cached.is_expired());

            // If still full, remove oldest entry
            if cache.len() >= self.max_size {
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }
        }

        cache.insert(
            tool_id.to_string(),
            CachedTool {
                tool,
                cached_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Put a tool in cache with custom TTL
    pub async fn put_with_ttl(&self, tool_id: &str, tool: ToolDefinition, ttl: Duration) {
        let mut cache = self.cache.write().await;

        // Check if cache is full
        if cache.len() >= self.max_size {
            // Remove expired entries first
            cache.retain(|_, cached| !cached.is_expired());

            // If still full, remove oldest entry
            if cache.len() >= self.max_size {
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }
        }

        cache.insert(
            tool_id.to_string(),
            CachedTool {
                tool,
                cached_at: Instant::now(),
                ttl,
            },
        );
    }

    /// Remove a tool from cache
    pub async fn remove(&self, tool_id: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(tool_id);
    }

    /// Clear all expired entries
    pub async fn cleanup(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, cached| !cached.is_expired());
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|c| c.is_expired()).count();

        CacheStats {
            total_entries: total,
            expired_entries: expired,
            active_entries: total - expired,
            max_size: self.max_size,
        }
    }

    /// Clear all entries
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub max_size: usize,
}

/// Tool execution pool for reusing execution contexts
pub struct ToolExecutionPool {
    pool: RwLock<Vec<ToolExecutionContext>>,
    max_size: usize,
    creation_count: RwLock<u64>,
    reuse_count: RwLock<u64>,
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    pub id: String,
    pub created_at: Instant,
    pub last_used: Instant,
    pub use_count: u64,
}

impl ToolExecutionPool {
    /// Create a new tool execution pool
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: RwLock::new(Vec::with_capacity(max_size)),
            max_size,
            creation_count: RwLock::new(0),
            reuse_count: RwLock::new(0),
        }
    }

    /// Get an execution context from the pool
    pub async fn get(&self) -> ToolExecutionContext {
        let mut pool = self.pool.write().await;

        if let Some(mut ctx) = pool.pop() {
            // Reuse existing context
            ctx.last_used = Instant::now();
            ctx.use_count += 1;

            let mut reuse_count = self.reuse_count.write().await;
            *reuse_count += 1;

            ctx
        } else {
            // Create new context
            let mut creation_count = self.creation_count.write().await;
            *creation_count += 1;

            ToolExecutionContext {
                id: uuid::Uuid::new_v4().to_string(),
                created_at: Instant::now(),
                last_used: Instant::now(),
                use_count: 1,
            }
        }
    }

    /// Return an execution context to the pool
    pub async fn put(&self, ctx: ToolExecutionContext) {
        let mut pool = self.pool.write().await;

        if pool.len() < self.max_size {
            pool.push(ctx);
        }
        // If pool is full, just drop the context
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let pool = self.pool.read().await;
        let creation_count = *self.creation_count.read().await;
        let reuse_count = *self.reuse_count.read().await;

        PoolStats {
            available: pool.len(),
            max_size: self.max_size,
            total_created: creation_count,
            total_reused: reuse_count,
            reuse_ratio: if creation_count > 0 {
                reuse_count as f64 / creation_count as f64
            } else {
                0.0
            },
        }
    }

    /// Clear the pool
    pub async fn clear(&self) {
        let mut pool = self.pool.write().await;
        pool.clear();
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub available: usize,
    pub max_size: usize,
    pub total_created: u64,
    pub total_reused: u64,
    pub reuse_ratio: f64,
}

/// Memory pool for reusing memory allocations
pub struct MemoryPool {
    pool: RwLock<Vec<Vec<u8>>>,
    max_size: usize,
    allocation_count: RwLock<u64>,
    reuse_count: RwLock<u64>,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: RwLock::new(Vec::with_capacity(max_size)),
            max_size,
            allocation_count: RwLock::new(0),
            reuse_count: RwLock::new(0),
        }
    }

    /// Get a buffer from the pool
    pub async fn get(&self, size: usize) -> Vec<u8> {
        let mut pool = self.pool.write().await;

        // Try to find a suitable buffer
        if let Some(pos) = pool.iter().position(|buf| buf.capacity() >= size) {
            let mut buf = pool.swap_remove(pos);
            buf.clear();

            let mut reuse_count = self.reuse_count.write().await;
            *reuse_count += 1;

            buf
        } else {
            // Create new buffer
            let mut allocation_count = self.allocation_count.write().await;
            *allocation_count += 1;

            Vec::with_capacity(size)
        }
    }

    /// Return a buffer to the pool
    pub async fn put(&self, buf: Vec<u8>) {
        let mut pool = self.pool.write().await;

        if pool.len() < self.max_size {
            pool.push(buf);
        }
        // If pool is full, just drop the buffer
    }

    /// Get pool statistics
    pub async fn stats(&self) -> MemoryPoolStats {
        let pool = self.pool.read().await;
        let allocation_count = *self.allocation_count.read().await;
        let reuse_count = *self.reuse_count.read().await;

        MemoryPoolStats {
            available: pool.len(),
            max_size: self.max_size,
            total_allocated: allocation_count,
            total_reused: reuse_count,
            reuse_ratio: if allocation_count > 0 {
                reuse_count as f64 / allocation_count as f64
            } else {
                0.0
            },
            total_memory: pool.iter().map(|buf| buf.capacity()).sum(),
        }
    }

    /// Clear the pool
    pub async fn clear(&self) {
        let mut pool = self.pool.write().await;
        pool.clear();
    }
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    pub available: usize,
    pub max_size: usize,
    pub total_allocated: u64,
    pub total_reused: u64,
    pub reuse_ratio: f64,
    pub total_memory: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_tool_cache() {
        let cache = ToolCache::new(Duration::from_secs(60), 100);

        let tool = ToolDefinition {
            id: "test.tool".to_string(),
            name: "Test Tool".to_string(),
            description: "A test tool".to_string(),
            category: "test".to_string(),
            parameters: serde_json::json!({}),
            required_permissions: vec![],
        };

        // Test put and get
        cache.put("test.tool", tool.clone()).await;
        let cached = cache.get("test.tool").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().id, "test.tool");

        // Test stats
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.active_entries, 1);

        // Test remove
        cache.remove("test.tool").await;
        let cached = cache.get("test.tool").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_tool_execution_pool() {
        let pool = ToolExecutionPool::new(10);

        // Test get and put
        let ctx1 = pool.get().await;
        assert_eq!(ctx1.use_count, 1);

        pool.put(ctx1).await;

        let ctx2 = pool.get().await;
        assert_eq!(ctx2.use_count, 2); // Reused

        // Test stats
        let stats = pool.stats().await;
        assert_eq!(stats.total_created, 1);
        assert_eq!(stats.total_reused, 1);
    }

    #[tokio::test]
    async fn test_memory_pool() {
        let pool = MemoryPool::new(10);

        // Test get and put
        let buf1 = pool.get(1024).await;
        assert!(buf1.capacity() >= 1024);

        pool.put(buf1).await;

        let buf2 = pool.get(512).await;
        assert!(buf2.capacity() >= 512);

        // Test stats
        let stats = pool.stats().await;
        assert_eq!(stats.total_allocated, 1);
        assert_eq!(stats.total_reused, 1);
    }
}
