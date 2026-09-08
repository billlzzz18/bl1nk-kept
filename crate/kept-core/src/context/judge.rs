use serde::{Deserialize, Serialize};

use super::correction_ledger::CorrectionLedger;
use super::registry::ContextRegistry;
use crate::observation::Observation;

/// Decision produced by the Judge engine for admission into the agent's context stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum AdmissionDecision {
    /// Action or dispatch is allowed without an attached observation.
    Allow,
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
    // NOTE-006: ตัวกรอง correction ledger เป็น optional เพื่อไม่ให้ Judge::new() เดิม break
    ledger: Option<CorrectionLedger>,
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
            ledger: None,
            repetition_threshold: 3,
            wasted_call_count: std::sync::atomic::AtomicUsize::new(0),
            consecutive_acquisitions: std::sync::atomic::AtomicUsize::new(0),
            consecutive_limit: 3,
        }
    }

    /// Attach a CorrectionLedger so the Judge blocks acquisition of targets with active corrections.
    pub fn with_ledger(mut self, ledger: CorrectionLedger) -> Self {
        self.ledger = Some(ledger);
        self
    }

    pub fn with_registry(registry: ContextRegistry) -> Self {
        Self {
            registry,
            ledger: None,
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

    // NOTE-007: ตรวจ correction ledger ก่อน acquisition จริง — ถ้ามี active correction ที่ขัดแย้งกับ
    // observation ที่กำลังจะเข้ามา ให้ Block ทันทีพร้อมแนบ correction_id และ evidence URI
    /// Evaluate an observation with correction ledger guard.
    ///
    /// Blocks acquisition if the ledger holds an active correction whose `rejected_assertion`
    /// matches the observation's content, preventing the agent from re-acquiring superseded facts.
    pub fn evaluate_with_ledger(&self, observation: Observation) -> AdmissionEvaluation {
        let target_uri = observation.source.target.to_string();

        // Pre-acquisition ledger guard: check for active correction against this target
        if let Some(ledger) = &self.ledger {
            // Use content as the assertion to check for conflicts
            let assertion_to_check = observation.content.as_deref().unwrap_or("");
            match ledger.conflict_for(&target_uri, assertion_to_check) {
                Ok(Some(correction)) => {
                    return AdmissionEvaluation {
                        decision: AdmissionDecision::Block {
                            target: target_uri,
                            reason: format!(
                                "Active correction [{id}] blocks acquisition: assertion '{}' was corrected to '{}'. Evidence: {evidence}",
                                correction.rejected_assertion,
                                correction.assertion,
                                id = correction.correction_id,
                                evidence = correction.evidence_target,
                            ),
                        },
                        confidence: 1.0,
                        rationale: "Correction ledger guard: acquisition blocked to prevent re-introducing a corrected fact".to_string(),
                    };
                }
                Ok(None) => {}
                Err(_) => {}
            }
        }

        // Ledger clear — proceed with normal admission pipeline
        self.evaluate_with_confidence(observation)
    }

    // NOTE-003: สกัดกั้นพฤติกรรม Agent ที่ทำงานล้มเหลว (value_produced = 0) แต่พยายามเขียนข้อความแก้ตัวหรือประจานความผิดพลาดของตัวเองยาวเหยียด
    /// Evaluate output payload from an agent or subagent before emitting to the user.
    /// Rejects shameless verbosity and self-commentary when zero value is produced.
    pub fn evaluate_output_payload(
        &self,
        text: &str,
        value_produced: usize,
    ) -> AdmissionEvaluation {
        if value_produced == 0 && text.chars().count() > 120 {
            // Check for self-commentary / failure excuses
            let failure_markers = [
                "รอบนี้ผลจริง",
                "คุณค่าตอนนี้ติดลบ",
                "ความผิดพลาด",
                "ล้มเหลว",
                "เสียเวลา",
            ];
            let contains_excuses = failure_markers.iter().any(|m| text.contains(m));
            if contains_excuses || text.len() > 200 {
                return AdmissionEvaluation {
                    decision: AdmissionDecision::Block {
                        target: "agent_output".to_string(),
                        reason: "Shameless failure verbosity: agent produced zero value but generated self-commentary or verbose excuses".to_string(),
                    },
                    confidence: 0.99,
                    rationale: "Behavioral guardrail: reject shameless failure reports and verbosity when task failed".to_string(),
                };
            }
        }

        AdmissionEvaluation {
            decision: AdmissionDecision::Allow,
            confidence: 1.0,
            rationale: "Output admitted".to_string(),
        }
    }

    // NOTE-004: สกัดกั้นการ spawn subagent อย่างฟุ่มเฟือยสำหรับงานที่ต้องการเพียงคำอธิบายทั่วไป
    /// Evaluate tool dispatch intention against requested intent.
    /// Blocks proportional sprawl (e.g. spawning subagents for simple explanation tasks).
    pub fn evaluate_dispatch(&self, intent: &str, tool_name: &str) -> AdmissionEvaluation {
        let is_simple_intent = intent.starts_with("explain")
            || intent.starts_with("read")
            || intent.starts_with("summary");
        let is_heavy_tool = tool_name == "spawn_subagent"
            || tool_name == "run_workflow"
            || tool_name == "multi_agent_orchestrate";

        if is_simple_intent && is_heavy_tool {
            return AdmissionEvaluation {
                decision: AdmissionDecision::Block {
                    target: tool_name.to_string(),
                    reason: "Subagent spawning blocked for simple explanation or reading tasks. Answer directly.".to_string(),
                },
                confidence: 1.0,
                rationale: "Proportional complexity guardrail: simple tasks must not spawn subagents".to_string(),
            };
        }

        AdmissionEvaluation {
            decision: AdmissionDecision::Allow,
            confidence: 0.95,
            rationale: "Dispatch permitted".to_string(),
        }
    }

    // NOTE-005: สกัดกั้นการแต่งเรื่อง (Fabrication / Extrapolation) ที่ไม่มี Provenance ชี้กลับไปยังข้อมูลจริง
    /// Evaluate whether generated assertions or narrative are backed by provenance evidence.
    /// Blocks unfounded narrative extrapolation.
    pub fn evaluate_content_provenance(
        &self,
        text: &str,
        raw_provenance_uris: &[String],
    ) -> AdmissionEvaluation {
        if raw_provenance_uris.is_empty() {
            return AdmissionEvaluation {
                decision: AdmissionDecision::Block {
                    target: "unfounded_narrative".to_string(),
                    reason: "Unfounded fabrication without raw data provenance: missing source evidence URIs".to_string(),
                },
                confidence: 0.95,
                rationale: "Accountability guardrail: assertions must cite raw evidence".to_string(),
            };
        }

        // Detect narrative extrapolation keywords
        let extrapolation_keywords = [
            "extrapolate",
            "assume",
            "conclude that the team",
            "cultural dysfunction",
        ];
        let has_extrapolation = extrapolation_keywords
            .iter()
            .any(|kw| text.to_lowercase().contains(kw));

        if has_extrapolation {
            return AdmissionEvaluation {
                decision: AdmissionDecision::Block {
                    target: "unfounded_narrative".to_string(),
                    reason: "Unfounded fabrication without raw data provenance: extrapolating narrative not grounded in raw facts".to_string(),
                },
                confidence: 0.98,
                rationale: "Factual boundary guardrail: narrative extrapolation without proof is prohibited".to_string(),
            };
        }

        AdmissionEvaluation {
            decision: AdmissionDecision::Allow,
            confidence: 0.95,
            rationale: "Provenance verified".to_string(),
        }
    }
}
