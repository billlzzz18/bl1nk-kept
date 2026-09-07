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
