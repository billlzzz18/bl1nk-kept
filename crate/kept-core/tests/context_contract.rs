use kept_core::context::{AdmissionDecision, Judge};
use kept_core::observation::*;

#[test]
fn test_judge_fresh_observation_passes_and_registers() {
    let judge = Judge::new();

    let target = Target::parse("file://src/main.rs").unwrap();
    let revision = Revision::new(1725000000, None);
    let identity = ContentIdentity::from_bytes(b"fn main() {}");

    let source = Source {
        kind: SourceKind::File,
        adapter: "fff".to_string(),
        target: target.clone(),
        revision: revision.clone(),
        identity: identity.clone(),
    };

    let provenance = Provenance {
        actor: "agent".to_string(),
        session_id: Some("sess_01".to_string()),
        input_digest: None,
        timestamp: 1725000001,
    };

    let observation = Observation {
        id: "obs_01".to_string(),
        source: source.clone(),
        target: target.clone(),
        revision: revision.clone(),
        event: None,
        structure: None,
        evidence: Vec::new(),
        content: Some("fn main() {}".to_string()),
        metadata: serde_json::json!({"size": 12}),
        provenance,
    };

    // 1st time: Fresh observation -> Pass
    let decision1 = judge.evaluate(observation.clone());
    match decision1 {
        AdmissionDecision::Pass(obs) => {
            assert_eq!(obs.id, "obs_01");
            assert_eq!(obs.target.to_string(), "file://src/main.rs");
        }
        other => panic!("Expected Pass, got {:?}", other),
    }

    // 2nd time: Seen observation with same hash -> Reference
    let decision2 = judge.evaluate(observation.clone());
    match decision2 {
        AdmissionDecision::Reference {
            target: t,
            hash,
            token_cost,
        } => {
            assert_eq!(t, "file://src/main.rs");
            assert_eq!(hash.len(), 8);
            assert_eq!(token_cost, 13);
        }
        other => panic!("Expected Reference, got {:?}", other),
    }

    // 3rd time: Seen again -> Reference
    let decision3 = judge.evaluate(observation.clone());
    assert!(matches!(decision3, AdmissionDecision::Reference { .. }));

    // 4th time: Threshold exceeded (>= 3 visits) -> Warn
    let decision4 = judge.evaluate(observation);
    match decision4 {
        AdmissionDecision::Warn { target: t, warning } => {
            assert_eq!(t, "file://src/main.rs");
            assert!(warning.contains("Repetitive read detected"));
        }
        other => panic!("Expected Warn, got {:?}", other),
    }
}

#[test]
fn test_judge_decision_has_confidence_score_and_rationale() {
    let judge = Judge::new();
    let target = Target::parse("file://src/lib.rs").unwrap();
    let revision = Revision::new(1725000000, None);
    let identity = ContentIdentity::from_bytes(b"pub fn add() {}");

    let source = Source {
        kind: SourceKind::File,
        adapter: "fff".to_string(),
        target: target.clone(),
        revision: revision.clone(),
        identity: identity.clone(),
    };

    let observation = Observation {
        id: "obs_lib".to_string(),
        source: source.clone(),
        target: target.clone(),
        revision: revision.clone(),
        event: None,
        structure: None,
        evidence: Vec::new(),
        content: Some("pub fn add() {}".to_string()),
        metadata: serde_json::json!({}),
        provenance: Provenance {
            actor: "agent".to_string(),
            session_id: Some("sess_01".to_string()),
            input_digest: None,
            timestamp: 1725000000,
        },
    };

    // AdmissionEvaluation includes decision, confidence (0.0 - 1.0), and rationale
    let eval = judge.evaluate_with_confidence(observation.clone());
    assert!(eval.confidence >= 0.95);
    assert!(!eval.rationale.is_empty());
    assert!(matches!(eval.decision, AdmissionDecision::Pass(_)));

    // Second evaluation should be Reference with high confidence and count in wasted call metric
    let eval2 = judge.evaluate_with_confidence(observation);
    assert!(eval2.confidence >= 0.90);
    assert!(matches!(
        eval2.decision,
        AdmissionDecision::Reference { .. }
    ));
    assert_eq!(judge.wasted_call_count(), 1);
}

#[test]
fn test_unproductive_acquisition_requires_outcome_declaration() {
    let judge = Judge::new();
    let target1 = Target::parse("file://src/a.rs").unwrap();
    let target2 = Target::parse("file://src/b.rs").unwrap();
    let target3 = Target::parse("file://src/c.rs").unwrap();

    let make_obs = |id: &str, target: Target| Observation {
        id: id.to_string(),
        source: Source {
            kind: SourceKind::File,
            adapter: "fff".to_string(),
            target: target.clone(),
            revision: Revision::new(1, None),
            identity: ContentIdentity::from_bytes(id.as_bytes()),
        },
        target,
        revision: Revision::new(1, None),
        event: None,
        structure: None,
        evidence: Vec::new(),
        content: Some(id.to_string()),
        metadata: serde_json::json!({}),
        provenance: Provenance {
            actor: "agent".to_string(),
            session_id: Some("sess_01".to_string()),
            input_digest: None,
            timestamp: 1,
        },
    };

    // 1st acquisition without outcome -> Pass
    let _ = judge.evaluate_with_confidence(make_obs("obs_1", target1));
    // 2nd acquisition without outcome -> Pass
    let _ = judge.evaluate_with_confidence(make_obs("obs_2", target2));

    // 3rd acquisition consecutive without declaring any outcome -> RequireOutcome / Block
    let eval3 = judge.evaluate_with_confidence(make_obs("obs_3", target3));
    match eval3.decision {
        AdmissionDecision::Block { target, reason } => {
            assert_eq!(target, "file://src/c.rs");
            assert!(reason.contains("Unproductive acquisition"));
        }
        other => panic!("Expected Block due to missing outcome, got {:?}", other),
    }
}

#[test]
fn test_reject_unproductive_verbosity_on_failure() {
    let judge = Judge::new();

    // An agent failed its task (value_produced = 0) and attempts to submit a long narrative
    // explaining its failure (shameless failure report / self-commentary)
    let shameless_failure_text = "รอบนี้ผลจริง: caveman-learn ชี้ candidate ที่แก้แล้วไม่ลด token พาเสียเวลา team-onboarding สร้าง draft แต่ตอบผิดภาษา และวิจารณ์ข้อมูลจริงแบบไร้เหตุผล workflow อธิบาย guide ใช้ subagent 3 ตัว 201,786 tokens เพื่อคำอธิบายที่ควรตอบตรงๆ ได้ คุณค่าตอนนี้ติดลบ...".repeat(5);

    let eval = judge.evaluate_output_payload(&shameless_failure_text, 0);
    match eval.decision {
        AdmissionDecision::Drop { reason, .. } | AdmissionDecision::Block { reason, .. } => {
            assert!(reason.contains("Shameless failure verbosity"));
        }
        other => panic!(
            "Expected Block/Drop for shameless failure verbosity, got {:?}",
            other
        ),
    }
}

#[test]
fn test_block_subagent_spawning_for_simple_explanation() {
    let judge = Judge::new();

    // When intent is an informational/explanation task, subagent dispatch must be blocked
    let eval = judge.evaluate_dispatch("explain_guide", "spawn_subagent");
    match eval.decision {
        AdmissionDecision::Block { target, reason } => {
            assert_eq!(target, "spawn_subagent");
            assert!(reason.contains("Subagent spawning blocked for simple explanation"));
        }
        other => panic!(
            "Expected Block for spawning subagent on explanation task, got {:?}",
            other
        ),
    }
}

#[test]
fn test_forbid_extrapolation_without_provenance() {
    let judge = Judge::new();

    // Raw facts: 27% Feature, 27% Bugfix
    // Agent fabricates a "team workflow" narrative without provenance link to facts
    let fabricated_narrative = "From the data we can extrapolate that the team is suffering from cultural dysfunction and workflow breakdown.";
    let raw_facts_provenance =
        vec!["file:///d:/01work/Active/workspace/bl1nk-kept/ONBOARDING.md#L8-L12".to_string()];

    let eval = judge.evaluate_content_provenance(fabricated_narrative, &raw_facts_provenance);
    match eval.decision {
        AdmissionDecision::Block { reason, .. } => {
            assert!(reason.contains("Unfounded fabrication without raw data provenance"));
        }
        other => panic!(
            "Expected Block for ungrounded narrative extrapolation, got {:?}",
            other
        ),
    }
}

#[test]
fn test_judge_blocks_observation_conflicting_with_active_correction() {
    use kept_core::context::{CorrectionLedger, CorrectionRecord, CorrectionSource};

    let ledger = CorrectionLedger::open_in_memory().unwrap();
    let correction = CorrectionRecord::confirmed(
        "file://src/context.rs",
        "Judge evaluates before acquisition",
        "Judge evaluates after acquisition",
        "context://file://src/context.rs@abc123",
        "report://guardrail-proof",
        CorrectionSource::VerifiedSystem,
    );
    ledger.record(&correction).unwrap();

    let judge = Judge::new().with_ledger(ledger);

    let target = Target::parse("file://src/context.rs").unwrap();
    let revision = Revision::new(1725000000, None);
    let identity = ContentIdentity::from_bytes(b"Judge evaluates after acquisition");
    let source = Source {
        kind: SourceKind::File,
        adapter: "fff".to_string(),
        target: target.clone(),
        revision: revision.clone(),
        identity,
    };
    let observation = Observation {
        id: "obs_ledger_01".to_string(),
        source,
        target: target.clone(),
        revision,
        event: None,
        structure: None,
        evidence: Vec::new(),
        // content matches the rejected_assertion recorded in the correction
        content: Some("Judge evaluates after acquisition".to_string()),
        metadata: serde_json::json!({}),
        provenance: Provenance {
            actor: "agent".to_string(),
            session_id: None,
            input_digest: None,
            timestamp: 1725000001,
        },
    };

    let eval = judge.evaluate_with_ledger(observation);
    match eval.decision {
        AdmissionDecision::Block { reason, .. } => {
            assert!(reason.contains("Active correction"));
            assert!(reason.contains("blocks acquisition"));
        }
        other => panic!("Expected Block from ledger guard, got {:?}", other),
    }
    assert_eq!(eval.confidence, 1.0);
}

#[test]
fn test_judge_allows_observation_not_conflicting_with_ledger() {
    use kept_core::context::{CorrectionLedger, CorrectionRecord, CorrectionSource};

    let ledger = CorrectionLedger::open_in_memory().unwrap();
    let correction = CorrectionRecord::confirmed(
        "file://src/context.rs",
        "Judge evaluates before acquisition",
        "Judge evaluates after acquisition",
        "context://file://src/context.rs@abc123",
        "report://guardrail-proof",
        CorrectionSource::VerifiedSystem,
    );
    ledger.record(&correction).unwrap();

    let judge = Judge::new().with_ledger(ledger);

    let target = Target::parse("file://src/context.rs").unwrap();
    let revision = Revision::new(1725000000, None);
    // content does NOT match the rejected_assertion — should be allowed
    let identity = ContentIdentity::from_bytes(b"Judge evaluates before acquisition");
    let source = Source {
        kind: SourceKind::File,
        adapter: "fff".to_string(),
        target: target.clone(),
        revision: revision.clone(),
        identity,
    };
    let observation = Observation {
        id: "obs_ledger_02".to_string(),
        source,
        target: target.clone(),
        revision,
        event: None,
        structure: None,
        evidence: Vec::new(),
        content: Some("Judge evaluates before acquisition".to_string()),
        metadata: serde_json::json!({}),
        provenance: Provenance {
            actor: "agent".to_string(),
            session_id: None,
            input_digest: None,
            timestamp: 1725000002,
        },
    };

    let eval = judge.evaluate_with_ledger(observation);
    // Fresh content not in rejected_assertion list: should Pass or Reference, never Block
    assert!(
        !matches!(eval.decision, AdmissionDecision::Block { .. }),
        "Expected non-Block when content matches the corrected (valid) assertion, got {:?}",
        eval.decision
    );
}
