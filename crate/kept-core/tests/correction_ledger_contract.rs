use kept_core::context::{
    CorrectionConfidence, CorrectionLedger, CorrectionRecord, CorrectionSource, CorrectionStatus,
};

#[test]
fn records_immutable_correction_with_evidence() {
    let ledger = CorrectionLedger::open_in_memory().unwrap();
    let record = CorrectionRecord::confirmed(
        "file://src/context.rs",
        "Judge evaluates before acquisition",
        "Judge evaluates after acquisition",
        "context://file://src/context.rs@abc123",
        "report://guardrail-proof",
        CorrectionSource::VerifiedSystem,
    );

    ledger.record(&record).unwrap();

    let active = ledger.active_for(&record.subject).unwrap();
    assert_eq!(active, vec![record.clone()]);
    assert_eq!(active[0].status, CorrectionStatus::Active);
    assert_eq!(active[0].confidence, CorrectionConfidence::Confirmed);
    assert!(active[0].created_at.ends_with('Z'));
    assert_eq!(active[0].supersedes, None);
}

#[test]
fn superseding_correction_preserves_history_and_replaces_active_record() {
    let ledger = CorrectionLedger::open_in_memory().unwrap();
    let original = CorrectionRecord::confirmed(
        "symbol://kept_core::context::Judge",
        "Judge evaluates before acquisition",
        "Judge evaluates after acquisition",
        "context://symbol://kept_core::context::Judge@old",
        "report://old-proof",
        CorrectionSource::User,
    );
    let replacement = CorrectionRecord::confirmed(
        original.subject.clone(),
        "Judge evaluates before tool dispatch",
        original.assertion.clone(),
        "context://symbol://kept_core::context::Judge@new",
        "report://new-proof",
        CorrectionSource::VerifiedSystem,
    );

    ledger.record(&original).unwrap();
    ledger
        .supersede(&original.correction_id, &replacement)
        .unwrap();

    let mut expected_replacement = replacement.clone();
    expected_replacement.supersedes = Some(original.correction_id.clone());
    let mut expected_original = original.clone();
    expected_original.status = CorrectionStatus::Superseded;

    assert_eq!(ledger.active_for(&original.subject).unwrap(), vec![expected_replacement.clone()]);
    assert_eq!(
        ledger.history_for(&original.subject).unwrap(),
        vec![expected_original, expected_replacement]
    );
}

#[test]
fn active_correction_returns_evidence_for_rejected_assertion() {
    let ledger = CorrectionLedger::open_in_memory().unwrap();
    let record = CorrectionRecord::confirmed(
        "file://src/context.rs",
        "Judge evaluates before acquisition",
        "Judge evaluates after acquisition",
        "context://file://src/context.rs@abc123",
        "report://guardrail-proof",
        CorrectionSource::VerifiedSystem,
    );

    ledger.record(&record).unwrap();

    assert_eq!(
        ledger
            .conflict_for(&record.subject, &record.rejected_assertion)
            .unwrap(),
        Some(record)
    );
}

#[test]
fn file_backed_ledger_migrates_and_reopens_active_correction() {
    let path = std::env::temp_dir().join(format!("kept-ledger-{}.sqlite", uuid::Uuid::new_v4()));
    let record = CorrectionRecord::confirmed(
        "file://src/context.rs",
        "Judge evaluates before acquisition",
        "Judge evaluates after acquisition",
        "context://file://src/context.rs@abc123",
        "report://guardrail-proof",
        CorrectionSource::VerifiedSystem,
    );

    CorrectionLedger::open(&path)
        .unwrap()
        .record(&record)
        .unwrap();

    assert_eq!(
        CorrectionLedger::open(&path)
            .unwrap()
            .active_for(&record.subject)
            .unwrap(),
        vec![record]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn migration_preserves_existing_superseded_status() {
    let path = std::env::temp_dir().join(format!("kept-ledger-{}.sqlite", uuid::Uuid::new_v4()));
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE corrections (
                correction_id TEXT PRIMARY KEY,
                subject TEXT NOT NULL,
                assertion TEXT NOT NULL,
                rejected_assertion TEXT NOT NULL,
                evidence_target TEXT NOT NULL,
                outcome_target TEXT NOT NULL,
                source TEXT NOT NULL,
                supersedes TEXT,
                superseded INTEGER NOT NULL DEFAULT 0
            ) STRICT;
            INSERT INTO corrections VALUES (
                'old', 'file://src/context.rs', 'new', 'old', 'context://evidence',
                'report://outcome', 'user', NULL, 1
            );",
        )
        .unwrap();
    drop(connection);

    let history = CorrectionLedger::open(&path)
        .unwrap()
        .history_for("file://src/context.rs")
        .unwrap();

    assert_eq!(history[0].status, CorrectionStatus::Superseded);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn superseding_unknown_correction_fails_without_creating_replacement() {
    let ledger = CorrectionLedger::open_in_memory().unwrap();
    let replacement = CorrectionRecord::confirmed(
        "file://src/context.rs",
        "Judge evaluates before acquisition",
        "Judge evaluates after acquisition",
        "context://file://src/context.rs@abc123",
        "report://guardrail-proof",
        CorrectionSource::VerifiedSystem,
    );

    assert!(ledger.supersede("missing", &replacement).is_err());
    assert!(ledger.history_for(&replacement.subject).unwrap().is_empty());
}
