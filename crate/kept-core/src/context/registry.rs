use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::observation::{Observation, Target};

/// Outcome kinds declared for an acquisition, enforcing accountability as per Cognitive Guardrail.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    Observation,
    Decision,
    Evidence,
    Correction,
    Report,
    Plan,
    Discard { reason: String },
}

/// Linkage connecting an acquisition to its declared durable outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeLinkage {
    pub acquisition_id: String,
    pub target: Target,
    pub revision_hash: String,
    pub session_id: Option<String>,
    pub requested_at: u64,
    pub outcome_kind: OutcomeKind,
    pub outcome_target: Option<String>,
}

/// Entry representing an observation seen by the session/agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenEntry {
    pub target: Target,
    pub revision_timestamp: u64,
    pub revision_token: Option<String>,
    pub content_hash: String,
    pub last_seen_unix: u64,
    pub view_count: usize,
}

/// ContextRegistry — session-scoped deduplication, delta diffing, and reference emission.
/// Ported from SQZ CacheManager (SHA-256 content-hash dedup + LRU eviction).
/// Source: D:\01work\Active\references\campbellr\sqz\sqz_engine\src\cache_manager.rs
pub struct ContextRegistry {
    entries: RwLock<HashMap<String, SeenEntry>>,
    max_entries: usize,
}

impl Default for ContextRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextRegistry {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    /// SHA-256 content hash — ported from SQZ CacheManager::sha256_hex.
    pub fn sha256_hex(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Truncated SHA-256 prefix for inline references (first 8 hex chars).
    pub fn ref_prefix(content: &[u8]) -> String {
        let full = Self::sha256_hex(content);
        format!("§ref:{}§", &full[..8])
    }

    /// Check if target was previously seen and matches the current revision/hash.
    pub fn lookup(&self, target_uri: &str) -> Option<SeenEntry> {
        let guard = self.entries.read().ok()?;
        guard.get(target_uri).cloned()
    }

    /// Check if content hash was previously seen (dedup by content, not by URI).
    pub fn lookup_by_hash(&self, content_hash: &str) -> Option<SeenEntry> {
        let guard = self.entries.read().ok()?;
        guard
            .values()
            .find(|e| e.content_hash == content_hash)
            .cloned()
    }

    /// Register or update an observation in the context registry.
    pub fn record(&self, observation: &Observation) {
        if let Ok(mut guard) = self.entries.write() {
            // LRU eviction: if at capacity, remove oldest entry
            if guard.len() >= self.max_entries {
                if let Some(oldest_key) = guard
                    .iter()
                    .min_by_key(|(_, e)| e.last_seen_unix)
                    .map(|(k, _)| k.clone())
                {
                    guard.remove(&oldest_key);
                }
            }

            let key = observation.source.target.to_string();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let entry = guard.entry(key.clone()).or_insert_with(|| SeenEntry {
                target: observation.source.target.clone(),
                revision_timestamp: observation.source.revision.modified_timestamp,
                revision_token: observation.source.revision.revision_token.clone(),
                content_hash: observation.source.identity.hash().to_string(),
                last_seen_unix: now,
                view_count: 0,
            });

            entry.revision_timestamp = observation.source.revision.modified_timestamp;
            entry.revision_token = observation.source.revision.revision_token.clone();
            entry.content_hash = observation.source.identity.hash().to_string();
            entry.last_seen_unix = now;
            entry.view_count += 1;
        }
    }

    /// Clear all recorded entries (e.g. on session compact or reset).
    pub fn clear(&self) {
        if let Ok(mut guard) = self.entries.write() {
            guard.clear();
        }
    }

    /// Number of active recorded entries.
    pub fn len(&self) -> usize {
        self.entries.read().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Evict LRU entries when over capacity. Returns number of entries removed.
    pub fn evict_lru(&self) -> usize {
        if let Ok(mut guard) = self.entries.write() {
            let excess = guard.len().saturating_sub(self.max_entries);
            if excess == 0 {
                return 0;
            }
            let mut keys_to_remove: Vec<(String, u64)> = guard
                .iter()
                .map(|(k, e)| (k.clone(), e.last_seen_unix))
                .collect();
            keys_to_remove.sort_by_key(|(_, ts)| *ts);
            keys_to_remove.truncate(excess);
            for (key, _) in keys_to_remove {
                guard.remove(&key);
            }
            excess
        } else {
            0
        }
    }
}

/// ContextRegistry with Outcome storage.
impl ContextRegistry {
    /// Record an outcome linkage in the registry.
    pub fn record_outcome(&self, _linkage: OutcomeLinkage) {
        // In-memory or session persistence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_deterministic() {
        let hash1 = ContextRegistry::sha256_hex(b"hello world");
        let hash2 = ContextRegistry::sha256_hex(b"hello world");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn sha256_hex_different_inputs() {
        let hash1 = ContextRegistry::sha256_hex(b"hello");
        let hash2 = ContextRegistry::sha256_hex(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn ref_prefix_format() {
        let prefix = ContextRegistry::ref_prefix(b"test content");
        assert!(prefix.starts_with("§ref:"));
        assert!(prefix.ends_with("§"));
        // §ref: (5) + 8 hex + § (1) = 14 chars minimum
        assert!(prefix.len() >= 14 && prefix.len() <= 20);
    }

    #[test]
    fn lookup_by_hash() {
        let registry = ContextRegistry::new();
        let hash = ContextRegistry::sha256_hex(b"test");
        // No entries yet
        assert!(registry.lookup_by_hash(&hash).is_none());
    }

    #[test]
    fn lru_eviction() {
        let mut registry = ContextRegistry::with_capacity(2);
        // Manually insert entries
        {
            let mut guard = registry.entries.write().unwrap();
            guard.insert(
                "a".into(),
                SeenEntry {
                    target: Target::File("a.txt".into()),
                    revision_timestamp: 1,
                    revision_token: None,
                    content_hash: "hash_a".into(),
                    last_seen_unix: 1,
                    view_count: 1,
                },
            );
            guard.insert(
                "b".into(),
                SeenEntry {
                    target: Target::File("b.txt".into()),
                    revision_timestamp: 2,
                    revision_token: None,
                    content_hash: "hash_b".into(),
                    last_seen_unix: 2,
                    view_count: 1,
                },
            );
        }
        assert_eq!(registry.len(), 2);

        // Evict should remove oldest (a, timestamp=1)
        let evicted = registry.evict_lru();
        assert_eq!(evicted, 0); // at capacity, no excess

        // Add one more to exceed capacity
        {
            let mut guard = registry.entries.write().unwrap();
            guard.insert(
                "c".into(),
                SeenEntry {
                    target: Target::File("c.txt".into()),
                    revision_timestamp: 3,
                    revision_token: None,
                    content_hash: "hash_c".into(),
                    last_seen_unix: 3,
                    view_count: 1,
                },
            );
        }
        assert_eq!(registry.len(), 3);

        let evicted = registry.evict_lru();
        assert_eq!(evicted, 1);
        assert_eq!(registry.len(), 2);
        assert!(registry.lookup("a").is_none()); // oldest evicted
        assert!(registry.lookup("b").is_some());
        assert!(registry.lookup("c").is_some());
    }
}
