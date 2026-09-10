use kept_core::observation::*;

#[test]
fn test_target_uri_parsing_and_formatting() {
    // file target
    let file_target = Target::parse("file://src/main.rs").expect("parse file uri");
    assert_eq!(file_target.to_string(), "file://src/main.rs");
    assert_eq!(file_target.scheme(), "file");
    assert_eq!(file_target.path(), "src/main.rs");

    // symbol target
    let symbol_target =
        Target::parse("symbol://crate/kept-core/src/policy.rs#Policy").expect("parse symbol uri");
    assert_eq!(symbol_target.to_string(), "symbol://crate/kept-core/src/policy.rs#Policy");
    assert_eq!(symbol_target.scheme(), "symbol");
    assert_eq!(symbol_target.path(), "crate/kept-core/src/policy.rs#Policy");

    // search target
    let search_target = Target::parse("search://bm25?query=invoice").expect("parse search uri");
    assert_eq!(search_target.to_string(), "search://bm25?query=invoice");
    assert_eq!(search_target.scheme(), "search");

    // context target
    let context_target =
        Target::parse("context://session/current@rev4").expect("parse context uri");
    assert_eq!(context_target.to_string(), "context://session/current@rev4");
    assert_eq!(context_target.scheme(), "context");
}

#[test]
fn test_target_rejects_invalid_scheme() {
    assert!(Target::parse("http://example.com").is_err());
    assert!(Target::parse("invalid_uri_without_scheme").is_err());
}

#[test]
fn test_observation_and_identity_roundtrip() {
    let target = Target::parse("file://README.md").unwrap();
    let revision = Revision::new(1725000000, Some("rev-1".to_string()));
    let identity = ContentIdentity::from_bytes(b"# kept\nFast workspace indexer.");

    let source = Source {
        kind: SourceKind::File,
        adapter: "fff".to_string(),
        target: target.clone(),
        revision: revision.clone(),
        identity: identity.clone(),
    };

    let provenance = Provenance {
        actor: "test-runner".to_string(),
        session_id: Some("session-01".to_string()),
        input_digest: None,
        timestamp: 1725000001,
    };

    let observation = Observation {
        id: "obs_01".to_string(),
        source,
        target: target.clone(),
        revision: revision.clone(),
        event: Some(ResourceEvent::Modified),
        structure: Some(StructurePayload {
            outline: vec![StructureOutlineItem {
                name: "Section 1".to_string(),
                kind: "heading".to_string(),
                range: Some((0, 10)),
                children: vec![],
            }],
            symbols: vec!["symbol_a".to_string()],
        }),
        evidence: vec![],
        content: Some("# kept\nFast workspace indexer.".to_string()),
        metadata: serde_json::json!({
            "size": 30,
            "is_binary": false
        }),
        provenance,
    };

    let serialized = serde_json::to_string(&observation).expect("serialize observation");
    let deserialized: Observation =
        serde_json::from_str(&serialized).expect("deserialize observation");

    assert_eq!(deserialized.id, "obs_01");
    assert_eq!(deserialized.source.kind, SourceKind::File);
    assert_eq!(deserialized.source.adapter, "fff");
    assert_eq!(deserialized.source.target.to_string(), "file://README.md");
    assert_eq!(deserialized.source.identity.hash(), identity.hash());
    assert_eq!(deserialized.event, Some(ResourceEvent::Modified));
    assert!(deserialized.structure.is_some());
}
