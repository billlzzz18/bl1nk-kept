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

/// Evaluation result containing the decision, a calibrated confidence score, and rationale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmissionEvaluation {
    pub decision: AdmissionDecision,
    pub confidence: f32,
    pub rationale: String,
}

/// The Judge admission engine inspired by SQZ CacheManager and ConfidenceRouter.
pub struct Judge {
    registry: ContextRegistry,
    repetition_threshold: usize,
    wasted_call_count: std::sync::atomic::AtomicUsize,
    consecutive_acquisitions: std::sync::atomic::AtomicUsize,
    consecutive_limit: usize,
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
            wasted_call_count: std::sync::atomic::AtomicUsize::new(0),
            consecutive_acquisitions: std::sync::atomic::AtomicUsize::new(0),
            consecutive_limit: 3,
        }
    }

    pub fn with_registry(registry: ContextRegistry) -> Self {
        Self {
            registry,
            repetition_threshold: 3,
            wasted_call_count: std::sync::atomic::AtomicUsize::new(0),
            consecutive_acquisitions: std::sync::atomic::AtomicUsize::new(0),
            consecutive_limit: 3,
        }
    }

    pub fn registry(&self) -> &ContextRegistry {
        &self.registry
    }

    pub fn wasted_call_count(&self) -> usize {
        self.wasted_call_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Declare a durable outcome (e.g. decision, evidence, report, edit) to reset the waste counter.
    pub fn declare_outcome(&self) {
        self.consecutive_acquisitions
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    /// Evaluate an observation against the context history and produce an admission decision with confidence.
    pub fn evaluate_with_confidence(&self, observation: Observation) -> AdmissionEvaluation {
        let target_uri = observation.source.target.to_string();

        // 1. Check for exact match in context registry (Memoization Layer)
        if let Some(seen) = self.registry.lookup(&target_uri) {
            let current_hash = observation.source.identity.hash();
            if seen.content_hash == current_hash {
                // Increment wasted call metric
                self.wasted_call_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                // Check if loop repetition threshold exceeded
                if seen.view_count >= self.repetition_threshold {
                    return AdmissionEvaluation {
                        decision: AdmissionDecision::Warn {
                            target: target_uri,
                            warning: format!(
                                "Repetitive read detected (seen {} times). Prefer local code edits.",
                                seen.view_count
                            ),
                        },
                        confidence: 0.95,
                        rationale: "Repeated acquisition of identical content exceeds threshold".to_string(),
                    };
                }

                // Record the visit count
                self.registry.record(&observation);

                return AdmissionEvaluation {
                    decision: AdmissionDecision::Reference {
                        target: target_uri,
                        hash: current_hash[..8.min(current_hash.len())].to_string(),
                        token_cost: 13,
                    },
                    confidence: 0.99,
                    rationale:
                        "Identical content found in active context registry memoization layer"
                            .to_string(),
                };
            }
        }

        // 2. Check for consecutive acquisitions without durable outcome (Behavioral Waste Gate)
        let count = self
            .consecutive_acquisitions
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        if count >= self.consecutive_limit {
            return AdmissionEvaluation {
                decision: AdmissionDecision::Block {
                    target: target_uri,
                    reason: "Unproductive acquisition: 3 consecutive reads without declaring a durable outcome".to_string(),
                },
                confidence: 1.0,
                rationale: "Behavioral waste gate triggered to prevent unrecorded context consumption".to_string(),
            };
        }

        // 3. Fresh or modified observation

        self.registry.record(&observation);
        AdmissionEvaluation {
            decision: AdmissionDecision::Pass(Box::new(observation)),
            confidence: 0.98,
            rationale: "Fresh observation admitted into context".to_string(),
        }
    }

    /// Evaluate an observation against the context history and produce an admission decision.
    pub fn evaluate(&self, observation: Observation) -> AdmissionDecision {
        self.evaluate_with_confidence(observation).decision
    }
}
