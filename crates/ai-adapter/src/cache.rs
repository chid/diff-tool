//! Hash-keyed result cache for AI responses.
//!
//! Caches LLM responses on disk keyed by `SHA256(input + model + prompt_version)`.
//! Cache location: `~/.cache/dt/ai-cache/` (or `$XDG_CACHE_HOME/dt/ai-cache/`).
//!
//! This module will be wired into `AiAdapter` in Phase 2.
#![allow(dead_code)]
//!
//! # Cache Invalidation
//!
//! - Bumping a prompt template version (see `prompts.rs`) invalidates all entries
//!   for that prompt type.
//! - Entries older than the configured TTL are considered stale and will be
//!   evicted on read.
//! - Users can clear the cache via settings.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::AiError;

/// Default cache TTL: 7 days in seconds.
const DEFAULT_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Metadata stored alongside each cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMetadata {
    /// Prompt template version at time of caching.
    prompt_version: u32,
    /// UNIX timestamp (seconds) when the entry was created.
    created_at: u64,
    /// The model used for this request.
    model: String,
}

/// A disk-backed AI response cache.
pub struct AiCache {
    root: PathBuf,
    ttl: Duration,
}

impl AiCache {
    /// Create a new cache at the given root directory.
    ///
    /// Creates the directory if it does not exist.
    pub fn new(root: PathBuf) -> Result<Self, AiError> {
        std::fs::create_dir_all(&root)
            .map_err(|e| AiError::Cache(format!("failed to create cache dir: {e}")))?;
        Ok(Self {
            root,
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
        })
    }

    /// Create a new cache with a custom TTL.
    pub fn with_ttl(root: PathBuf, ttl: Duration) -> Result<Self, AiError> {
        std::fs::create_dir_all(&root)
            .map_err(|e| AiError::Cache(format!("failed to create cache dir: {e}")))?;
        Ok(Self { root, ttl })
    }

    /// Build a cache key from input text, model name, and prompt version.
    pub fn make_key(input: &str, model: &str, prompt_version: u32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hasher.update(model.as_bytes());
        hasher.update(&prompt_version.to_le_bytes());
        hex::encode(hasher.finalize())
    }

    /// Look up a cached response. Returns `None` if:
    /// - The entry does not exist.
    /// - The entry is older than the TTL (it will be deleted).
    /// - The entry metadata cannot be read.
    pub fn get(&self, key: &str) -> Option<String> {
        let entry_dir = self.root.join(key);
        let content_path = entry_dir.join("response.txt");
        let meta_path = entry_dir.join("meta.json");

        if !content_path.exists() || !meta_path.exists() {
            return None;
        }

        // Check TTL via metadata.
        if let Ok(meta_bytes) = std::fs::read(&meta_path) {
            if let Ok(meta) = serde_json::from_slice::<CacheMetadata>(&meta_bytes) {
                let age = self.age_secs(meta.created_at);
                if age > self.ttl.as_secs() {
                    // Entry is stale — evict it.
                    let _ = std::fs::remove_dir_all(&entry_dir);
                    return None;
                }
            }
        }

        // Read the cached response.
        std::fs::read_to_string(&content_path).ok()
    }

    /// Store a response in the cache.
    pub fn put(
        &self,
        key: &str,
        response: &str,
        model: &str,
        prompt_version: u32,
    ) -> Result<(), AiError> {
        let entry_dir = self.root.join(key);
        std::fs::create_dir_all(&entry_dir).map_err(|e| {
            AiError::Cache(format!("failed to create cache entry dir: {e}"))
        })?;

        let meta = CacheMetadata {
            prompt_version,
            created_at: now_secs(),
            model: model.into(),
        };

        let meta_json = serde_json::to_vec(&meta).map_err(|e| {
            AiError::Cache(format!("failed to serialize cache metadata: {e}"))
        })?;

        std::fs::write(entry_dir.join("meta.json"), &meta_json).map_err(|e| {
            AiError::Cache(format!("failed to write cache metadata: {e}"))
        })?;

        std::fs::write(entry_dir.join("response.txt"), response).map_err(|e| {
            AiError::Cache(format!("failed to write cached response: {e}"))
        })?;

        Ok(())
    }

    /// Remove a specific cache entry.
    pub fn remove(&self, key: &str) -> Result<(), AiError> {
        let entry_dir = self.root.join(key);
        if entry_dir.exists() {
            std::fs::remove_dir_all(&entry_dir).map_err(|e| {
                AiError::Cache(format!("failed to remove cache entry: {e}"))
            })?;
        }
        Ok(())
    }

    /// Clear all cache entries.
    pub fn clear_all(&self) -> Result<(), AiError> {
        if self.root.exists() {
            for entry in std::fs::read_dir(&self.root).map_err(|e| {
                AiError::Cache(format!("failed to read cache dir: {e}"))
            })? {
                let entry = entry.map_err(|e| {
                    AiError::Cache(format!("failed to read cache entry: {e}"))
                })?;
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    std::fs::remove_dir_all(entry.path()).map_err(|e| {
                        AiError::Cache(format!("failed to remove cache entry: {e}"))
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Return the number of cached entries.
    pub fn entry_count(&self) -> Result<usize, AiError> {
        if !self.root.exists() {
            return Ok(0);
        }
        let count = std::fs::read_dir(&self.root)
            .map_err(|e| AiError::Cache(format!("failed to read cache dir: {e}")))?
            .filter(|e| {
                e.as_ref()
                    .ok()
                    .and_then(|e| e.file_type().ok())
                    .map(|t| t.is_dir())
                    .unwrap_or(false)
            })
            .count();
        Ok(count)
    }

    fn age_secs(&self, created_at: u64) -> u64 {
        now_secs().saturating_sub(created_at)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CACHE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn test_cache() -> AiCache {
        let id = CACHE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("ai-cache-test-{}-{}", std::process::id(), id));
        // Ensure clean state (remove stale dir from a previous run).
        let _ = std::fs::remove_dir_all(&dir);
        AiCache::new(dir).unwrap()
    }

    #[test]
    fn put_and_get_round_trip() {
        let cache = test_cache();
        let key = AiCache::make_key("hello world", "gpt-4o", 1);

        cache
            .put(&key, "cached response", "gpt-4o", 1)
            .unwrap();

        let result = cache.get(&key);
        assert_eq!(result, Some("cached response".into()));
    }

    #[test]
    fn get_missing_key_returns_none() {
        let cache = test_cache();
        let result = cache.get("nonexistent-key");
        assert_eq!(result, None);
    }

    #[test]
    fn remove_deletes_entry() {
        let cache = test_cache();
        let key = AiCache::make_key("test", "model", 1);

        cache.put(&key, "data", "model", 1).unwrap();
        assert!(cache.get(&key).is_some());

        cache.remove(&key).unwrap();
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn clear_all_removes_everything() {
        let cache = test_cache();
        let key1 = AiCache::make_key("a", "m", 1);
        let key2 = AiCache::make_key("b", "m", 1);

        cache.put(&key1, "x", "m", 1).unwrap();
        cache.put(&key2, "y", "m", 1).unwrap();

        assert_eq!(cache.entry_count().unwrap(), 2);

        cache.clear_all().unwrap();

        assert_eq!(cache.entry_count().unwrap(), 0);
        assert!(cache.get(&key1).is_none());
        assert!(cache.get(&key2).is_none());
    }

    #[test]
    fn make_key_is_deterministic() {
        let k1 = AiCache::make_key("input", "gpt-4", 1);
        let k2 = AiCache::make_key("input", "gpt-4", 1);
        assert_eq!(k1, k2);
    }

    #[test]
    fn make_key_differs_by_input() {
        let k1 = AiCache::make_key("input A", "gpt-4", 1);
        let k2 = AiCache::make_key("input B", "gpt-4", 1);
        assert_ne!(k1, k2);
    }

    #[test]
    fn make_key_differs_by_model() {
        let k1 = AiCache::make_key("input", "gpt-4", 1);
        let k2 = AiCache::make_key("input", "claude-3", 1);
        assert_ne!(k1, k2);
    }

    #[test]
    fn make_key_differs_by_version() {
        let k1 = AiCache::make_key("input", "gpt-4", 1);
        let k2 = AiCache::make_key("input", "gpt-4", 2);
        assert_ne!(k1, k2);
    }
}
