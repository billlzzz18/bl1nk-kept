use serde::{Deserialize, Serialize};

use super::registry::ContextRegistry;
use crate::observation::Observation;

/// Decision produced by the Judge engine for admission into the agent's context stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum AdmissionDecision {
    /// Pass observation full content directly.
    Pass(Box<Observation>),
    /// Content was seen before with identical revision/hash; emit compact reference instead.
    Reference {
        target: String,
        hash: String,
        token_cost: usize,
    },
    /// Content is near-duplicate or modified; emit compact unified delta diff.
    Delta {
        target: String,
        diff: String,
        token_cost: usize,
    },
    /// Compress structured JSON or output via field pruning / array collapse.
    Compress {
        target: String,
        compressed: String,
        ratio: f64,
    },
    /// Low-value or noise observation; drop completely from context.
    Drop { target: String, reason: String },
    /// Safety or loop detection warning attached to context.
    Warn { target: String, warning: String },
    /// Dangerous operation or violation blocked.
    Block { target: String, reason: String },
}

/// The Judge admission engine inspired by SQZ CacheManager and ConfidenceRouter.
pub struct Judge {
    registry: ContextRegistry,
    repetition_threshold: usize,
}

impl Default for Judge {
    fn default() -> Self {
        Self::new()
    }
}

impl Judge {
    pub fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
            repetition_threshold: 3,
        }
    }

    pub fn with_registry(registry: ContextRegistry) -> Self {
        Self {
            registry,
            repetition_threshold: 3,
        }
    }

    pub fn registry(&self) -> &ContextRegistry {
        &self.registry
    }

    /// Evaluate an observation against the context history and produce an admission decision.
    pub fn evaluate(&self, observation: Observation) -> AdmissionDecision {
        let target_uri = observation.source.target.to_string();

        // 1. Check for exact match in context registry
        if let Some(seen) = self.registry.lookup(&target_uri) {
            let current_hash = observation.source.identity.hash();
            if seen.content_hash == current_hash {
                // Check if loop repetition threshold exceeded
                if seen.view_count >= self.repetition_threshold {
                    return AdmissionDecision::Warn {
                        target: target_uri,
                        warning: format!(
                            "Repetitive read detected (seen {} times). Prefer local code edits.",
                            seen.view_count
                        ),
                    };
                }

                // Record the visit count
                self.registry.record(&observation);

                return AdmissionDecision::Reference {
                    target: target_uri,
                    hash: current_hash[..8.min(current_hash.len())].to_string(),
                    token_cost: 13,
                };
            }
        }

        // 2. Fresh or modified observation
        self.registry.record(&observation);
        AdmissionDecision::Pass(Box::new(observation))
    }
}
