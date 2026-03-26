//! Job deduplication to prevent duplicate route computations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::job::JobId;

/// In-memory deduplication cache
/// Tracks jobs in-flight to prevent duplicate route computations
pub struct DeduplicationCache {
    cache: Arc<RwLock<HashMap<String, DeduplicationEntry>>>,
}

#[derive(Clone, Debug)]
struct DeduplicationEntry {
    job_id: JobId,
    created_at: std::time::Instant,
}

impl DeduplicationCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if job is currently being processed and add to cache if not
    /// Returns true if job was added (not a duplicate), false if already processing
    pub async fn try_add(&self, job_id: &JobId) -> bool {
        let key = job_id.as_hash_key();
        let mut cache = self.cache.write().await;

        if cache.contains_key(&key) {
            false // Duplicate
        } else {
            cache.insert(
                key,
                DeduplicationEntry {
                    job_id: job_id.clone(),
                    created_at: std::time::Instant::now(),
                },
            );
            true // Added successfully
        }
    }

    /// Remove job from deduplication cache after completion
    pub async fn remove(&self, job_id: &JobId) {
        let key = job_id.as_hash_key();
        let mut cache = self.cache.write().await;
        cache.remove(&key);
    }

    /// Cleanup expired entries (older than TTL)
    pub async fn cleanup_expired(&self, ttl_seconds: u64) {
        let mut cache = self.cache.write().await;
        let now = std::time::Instant::now();
        let ttl = std::time::Duration::from_secs(ttl_seconds);

        cache.retain(|_, entry| now.duration_since(entry.created_at) < ttl);
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

impl Default for DeduplicationCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deduplication() {
        let cache = DeduplicationCache::new();
        let job_id = JobId::new("native", "USDC", "100", "sell");

        // First add should succeed
        assert!(cache.try_add(&job_id).await);

        // Second add should fail (duplicate)
        assert!(!cache.try_add(&job_id).await);

        // After removal, should succeed again
        cache.remove(&job_id).await;
        assert!(cache.try_add(&job_id).await);
    }

    #[tokio::test]
    async fn test_different_jobs_not_duplicates() {
        let cache = DeduplicationCache::new();
        let job1 = JobId::new("native", "USDC", "100", "sell");
        let job2 = JobId::new("native", "BTC", "100", "sell");

        assert!(cache.try_add(&job1).await);
        assert!(cache.try_add(&job2).await); // Different job, should succeed
    }
}
