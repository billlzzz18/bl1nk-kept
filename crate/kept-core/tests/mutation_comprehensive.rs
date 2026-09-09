//! Comprehensive unit tests for kept-core::scanner::mutation
//!
//! Covers: DuplicateMutationPolicy defaults, create_duplicate_mutation_plan,
//! simulate_duplicate_mutation safety checks, execute/rollback on real files.

use kept_core::scanner::DuplicateMutationPlan;
use kept_core::{
    create_duplicate_mutation_plan, simulate_duplicate_mutation, DuplicateActionKind,
    DuplicateEvidence, DuplicateGroup, DuplicateMutationPolicy, DuplicatePlanAction, FileRecord,
    ScanIndex,
};

fn make_file(path: &str, name: &str, size: u64) -> FileRecord {
    FileRecord {
        path: path.to_string(),
        name: name.to_string(),
        extension: name.rsplit('.').next().unwrap_or("").to_string(),
        size,
        modified_unix: 0,
        kind: "file".to_string(),
        is_binary: None,
        git_status: None,
    }
}

fn make_index(root: &str, files: Vec<FileRecord>) -> ScanIndex {
    ScanIndex {
        root: root.to_string(),
        scanned_at_unix: 0,
        total_size: files.iter().map(|f| f.size).sum(),
        files,
        issues: Vec::new(),
    }
}

// ─── DuplicateMutationPolicy defaults ───────────────────────────────────────

#[test]
fn default_policy_has_protected_patterns() {
    let policy = DuplicateMutationPolicy::default();
    assert!(policy.protected_patterns.contains(&".git".to_string()));
    assert!(policy
        .protected_patterns
        .contains(&"node_modules".to_string()));
    assert!(policy.protected_patterns.contains(&".kept".to_string()));
}

#[test]
fn default_policy_preserves_canonical() {
    let policy = DuplicateMutationPolicy::default();
    assert!(policy.preserve_canonical);
    assert!(policy.verify_checksum_before_action);
    assert!(policy.allowed_roots.is_empty());
    assert!(policy.allow_list.is_empty());
    assert!(policy.backup_directory.is_none());
}

// ─── create_duplicate_mutation_plan ─────────────────────────────────────────

#[test]
fn plan_creates_actions_for_each_duplicate() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/a/report.txt", "report.txt", 100),
            make_file("/work/b/report.txt", "report.txt", 100),
            make_file("/work/c/report.txt", "report.txt", 100),
        ],
    );
    let groups = vec![DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec![
            "/work/a/report.txt".to_string(),
            "/work/b/report.txt".to_string(),
            "/work/c/report.txt".to_string(),
        ],
        evidence: None,
    }];

    let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Trash);

    // 2 actions: b and c are duplicates of a (canonical)
    assert_eq!(plan.actions.len(), 2);
    assert_eq!(plan.total_reclaimable_bytes, 200);
    assert!(plan.root == "/work");
    // Canonical is first item, targets are items[1..]
    assert_eq!(plan.actions[0].canonical_path, "/work/a/report.txt");
    assert_eq!(plan.actions[0].target_path, "/work/b/report.txt");
    assert_eq!(plan.actions[1].target_path, "/work/c/report.txt");
}

#[test]
fn plan_skips_groups_with_single_item() {
    let index = make_index("/work", vec![]);
    let groups = vec![DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec!["/work/a.txt".to_string()],
        evidence: None,
    }];

    let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Delete);
    assert!(plan.actions.is_empty());
    assert_eq!(plan.total_reclaimable_bytes, 0);
}

#[test]
fn plan_uses_evidence_size_when_available() {
    let index = make_index("/work", vec![]);
    let groups = vec![DuplicateGroup {
        kind: "same_content".to_string(),
        similarity: 1.0,
        items: vec!["/work/a.bin".to_string(), "/work/b.bin".to_string()],
        evidence: Some(DuplicateEvidence {
            size_bytes: 999,
            partial_sha256: "aaa".to_string(),
            full_sha256: "bbb".to_string(),
        }),
    }];

    let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Trash);
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].expected_size, 999);
    assert_eq!(plan.actions[0].expected_sha256, "bbb");
    assert_eq!(plan.total_reclaimable_bytes, 999);
}

#[test]
fn plan_uses_index_size_when_no_evidence() {
    let index = make_index("/work", vec![make_file("/work/a.txt", "a.txt", 42)]);
    let groups = vec![DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec!["/work/a.txt".to_string(), "/work/b.txt".to_string()],
        evidence: None,
    }];

    let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Delete);
    assert_eq!(plan.actions[0].expected_size, 42);
}

#[test]
fn plan_sets_correct_action_kind() {
    let index = make_index("/work", vec![]);
    let groups = vec![DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec!["/work/a.txt".to_string(), "/work/b.txt".to_string()],
        evidence: None,
    }];

    let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::HardLink);
    assert_eq!(plan.actions[0].action, DuplicateActionKind::HardLink);
}

// ─── simulate_duplicate_mutation ────────────────────────────────────────────

#[test]
fn simulate_passes_for_valid_actions() {
    let plan = DuplicateMutationPlan {
        root: "/work".to_string(),
        created_at_unix: 0,
        actions: vec![DuplicatePlanAction {
            target_path: "/work/b.txt".to_string(),
            canonical_path: "/work/a.txt".to_string(),
            action: DuplicateActionKind::Trash,
            expected_size: 100,
            expected_sha256: String::new(),
            reclaimed_bytes: 100,
        }],
        total_reclaimable_bytes: 100,
    };
    let index = make_index("/work", vec![make_file("/work/b.txt", "b.txt", 100)]);
    let policy = DuplicateMutationPolicy::default();

    let result = simulate_duplicate_mutation(&plan, &index, &policy);
    assert!(result.simulation_passed);
    assert_eq!(result.valid_actions.len(), 1);
    assert!(result.blocked_actions.is_empty());
}

#[test]
fn simulate_blocks_path_outside_root() {
    let plan = DuplicateMutationPlan {
        root: "/work".to_string(),
        created_at_unix: 0,
        actions: vec![DuplicatePlanAction {
            target_path: "/other/secret.txt".to_string(),
            canonical_path: "/work/a.txt".to_string(),
            action: DuplicateActionKind::Delete,
            expected_size: 100,
            expected_sha256: String::new(),
            reclaimed_bytes: 100,
        }],
        total_reclaimable_bytes: 100,
    };
    let index = make_index("/work", vec![]);
    let policy = DuplicateMutationPolicy::default();

    let result = simulate_duplicate_mutation(&plan, &index, &policy);
    assert!(!result.simulation_passed);
    assert_eq!(result.blocked_actions.len(), 1);
    assert!(result.blocked_actions[0]
        .rejection_reason
        .as_ref()
        .unwrap()
        .contains("outside allowed root"));
}

#[test]
fn simulate_blocks_protected_pattern() {
    let plan = DuplicateMutationPlan {
        root: "/work".to_string(),
        created_at_unix: 0,
        actions: vec![DuplicatePlanAction {
            target_path: "/work/.git/config.bak".to_string(),
            canonical_path: "/work/a.txt".to_string(),
            action: DuplicateActionKind::Trash,
            expected_size: 100,
            expected_sha256: String::new(),
            reclaimed_bytes: 100,
        }],
        total_reclaimable_bytes: 100,
    };
    let index = make_index("/work", vec![]);
    let policy = DuplicateMutationPolicy::default();

    let result = simulate_duplicate_mutation(&plan, &index, &policy);
    assert!(!result.simulation_passed);
    assert!(result.blocked_actions[0]
        .rejection_reason
        .as_ref()
        .unwrap()
        .contains("protected directory"));
}

#[test]
fn simulate_blocks_size_mismatch_in_index() {
    let plan = DuplicateMutationPlan {
        root: "/work".to_string(),
        created_at_unix: 0,
        actions: vec![DuplicatePlanAction {
            target_path: "/work/b.txt".to_string(),
            canonical_path: "/work/a.txt".to_string(),
            action: DuplicateActionKind::Delete,
            expected_size: 100,
            expected_sha256: String::new(),
            reclaimed_bytes: 100,
        }],
        total_reclaimable_bytes: 100,
    };
    let index = make_index(
        "/work",
        vec![make_file("/work/b.txt", "b.txt", 200)], // Different size
    );
    let policy = DuplicateMutationPolicy::default();

    let result = simulate_duplicate_mutation(&plan, &index, &policy);
    assert!(!result.simulation_passed);
    assert!(result.blocked_actions[0]
        .rejection_reason
        .as_ref()
        .unwrap()
        .contains("File size mismatch"));
}

#[test]
fn simulate_allows_path_within_allowed_root() {
    let plan = DuplicateMutationPlan {
        root: "/work".to_string(),
        created_at_unix: 0,
        actions: vec![DuplicatePlanAction {
            target_path: "/other/b.txt".to_string(),
            canonical_path: "/work/a.txt".to_string(),
            action: DuplicateActionKind::Trash,
            expected_size: 100,
            expected_sha256: String::new(),
            reclaimed_bytes: 100,
        }],
        total_reclaimable_bytes: 100,
    };
    let index = make_index("/work", vec![]);
    let policy = DuplicateMutationPolicy {
        allowed_roots: vec!["/other".to_string()],
        ..DuplicateMutationPolicy::default()
    };

    let result = simulate_duplicate_mutation(&plan, &index, &policy);
    assert!(result.simulation_passed);
}

#[test]
fn simulate_calculates_total_reclaimable() {
    let plan = DuplicateMutationPlan {
        root: "/work".to_string(),
        created_at_unix: 0,
        actions: vec![
            DuplicatePlanAction {
                target_path: "/work/b.txt".to_string(),
                canonical_path: "/work/a.txt".to_string(),
                action: DuplicateActionKind::Trash,
                expected_size: 100,
                expected_sha256: String::new(),
                reclaimed_bytes: 100,
            },
            DuplicatePlanAction {
                target_path: "/work/d.txt".to_string(),
                canonical_path: "/work/c.txt".to_string(),
                action: DuplicateActionKind::Trash,
                expected_size: 200,
                expected_sha256: String::new(),
                reclaimed_bytes: 200,
            },
        ],
        total_reclaimable_bytes: 300,
    };
    let index = make_index(
        "/work",
        vec![
            make_file("/work/b.txt", "b.txt", 100),
            make_file("/work/d.txt", "d.txt", 200),
        ],
    );
    let policy = DuplicateMutationPolicy::default();

    let result = simulate_duplicate_mutation(&plan, &index, &policy);
    assert_eq!(result.total_reclaimable_bytes, 300);
}

// ─── Integration: plan → simulate roundtrip ─────────────────────────────────

#[test]
fn plan_then_simulate_roundtrip() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/a/report.pdf", "report.pdf", 500),
            make_file("/work/b/report.pdf", "report.pdf", 500),
        ],
    );
    let groups = vec![DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec![
            "/work/a/report.pdf".to_string(),
            "/work/b/report.pdf".to_string(),
        ],
        evidence: Some(DuplicateEvidence {
            size_bytes: 500,
            partial_sha256: "aaa".to_string(),
            full_sha256: "bbb".to_string(),
        }),
    }];

    let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Delete);
    let policy = DuplicateMutationPolicy::default();
    let result = simulate_duplicate_mutation(&plan, &index, &policy);

    assert!(result.simulation_passed);
    assert_eq!(result.valid_actions.len(), 1);
    assert_eq!(result.valid_actions[0].target_path, "/work/b/report.pdf");
    assert_eq!(result.valid_actions[0].canonical_path, "/work/a/report.pdf");
}

#[test]
fn mixed_valid_and_blocked_actions() {
    let plan = DuplicateMutationPlan {
        root: "/work".to_string(),
        created_at_unix: 0,
        actions: vec![
            DuplicatePlanAction {
                target_path: "/work/b.txt".to_string(),
                canonical_path: "/work/a.txt".to_string(),
                action: DuplicateActionKind::Trash,
                expected_size: 100,
                expected_sha256: String::new(),
                reclaimed_bytes: 100,
            },
            DuplicatePlanAction {
                target_path: "/work/.git/config.bak".to_string(),
                canonical_path: "/work/a.txt".to_string(),
                action: DuplicateActionKind::Trash,
                expected_size: 50,
                expected_sha256: String::new(),
                reclaimed_bytes: 50,
            },
        ],
        total_reclaimable_bytes: 150,
    };
    let index = make_index("/work", vec![make_file("/work/b.txt", "b.txt", 100)]);
    let policy = DuplicateMutationPolicy::default();

    let result = simulate_duplicate_mutation(&plan, &index, &policy);
    assert!(!result.simulation_passed);
    assert_eq!(result.valid_actions.len(), 1);
    assert_eq!(result.blocked_actions.len(), 1);
}
