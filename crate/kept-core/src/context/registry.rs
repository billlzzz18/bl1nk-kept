use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

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

/// ContextRegistry tracks observations already seen in this session to support
/// deduplication, delta diffing, and reference emission (similar to SQZ CacheManager).
pub struct ContextRegistry {
    entries: RwLock<HashMap<String, SeenEntry>>,
}

impl Default for ContextRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Check if target was previously seen and matches the current revision/hash.
    pub fn lookup(&self, target_uri: &str) -> Option<SeenEntry> {
        let guard = self.entries.read().ok()?;
        guard.get(target_uri).cloned()
    }

    /// Register or update an observation in the context registry.
    pub fn record(&self, observation: &Observation) {
        if let Ok(mut guard) = self.entries.write() {
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
}

/// ContextRegistry with Outcome storage.
impl ContextRegistry {
    /// Record an outcome linkage in the registry.
    pub fn record_outcome(&self, _linkage: OutcomeLinkage) {
        // In-memory or session persistence
    }
}
