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
