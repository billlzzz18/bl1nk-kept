//! Scanner module split into cohesive submodules:
//! - `types`: Core structs (`ScanIndex`, `FileRecord`, `PersistentScanSnapshot`, `ScanIssue`, etc.)
//! - `scan`: Directory traversal and metadata extraction
//! - `filter`: Filter criteria and treemap generation
//! - `duplicate`: Exact/near name, content duplicates, and allow-lists
//! - `mutation`: Mutation policy, simulation, execution, and rollback
//! - `integrity`: Magic byte detection and bad extension inspection

pub mod duplicate;
pub mod fff;
pub mod filter;
pub mod integrity;
pub mod mutation;
pub mod query;
pub mod scan;
pub mod types;

pub use duplicate::{
    filter_allowed_duplicates, find_content_duplicates, find_duplicates,
    find_duplicates_with_stats, normalized_similarity, ContentDuplicateOptions,
    ContentDuplicateStats, DuplicateAllowRule, DuplicateEvidence, DuplicateGroup, DuplicateOptions,
    DuplicateSearchStats,
};
pub use fff::{FffAcquisitionMode, FffAdapterError, FffScanner};
pub use filter::{
    build_treemap, filter_index, CustomFilter, CustomOperator, FileFilter, FilterSet, TreemapNode,
};
pub use integrity::{
    check_file_extension_integrity, scan_index_integrity, BadExtensionIssue, IntegrityStatus,
};
pub use mutation::{
    create_duplicate_mutation_plan, execute_duplicate_mutation, rollback_duplicate_mutation,
    simulate_duplicate_mutation, ActionSimulation, DuplicateActionKind, DuplicateMutationPlan,
    DuplicateMutationPolicy, DuplicatePlanAction, DuplicateSimulationResult, RollbackEntry,
    RollbackJournal,
};
pub use query::{compile_query, parse_size_to_bytes, QueryClause, QueryPlan};
pub use scan::scan_directory;
pub use types::{
    apply_refresh_plan, create_persistent_snapshot, migrate_snapshot, plan_incremental_refresh,
    FileRecord, PersistentScanSnapshot, RefreshPlan, ScanIndex, ScanIssue, ScanIssueKind,
    ScanOptions, CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn scan_index_persists_structured_scan_issues() {
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: 0,
            files: Vec::new(),
            issues: vec![ScanIssue::new("locked", "read_dir", "permission denied")],
        };

        let value = serde_json::to_value(index).expect("index must serialize");
        assert_eq!(value["issues"][0]["path"], "locked");
        assert_eq!(value["issues"][0]["operation"], "read_dir");
    }

    #[test]
    fn similarity_is_adjustable_and_normalized() {
        assert_eq!(normalized_similarity("report.txt", "report.txt"), 1.0);
        assert!(normalized_similarity("report", "reports") > 0.8);
    }

    fn file(path: &str, name: &str, size: u64) -> FileRecord {
        FileRecord {
            path: path.to_string(),
            name: name.to_string(),
            extension: Path::new(name)
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_string(),
            size,
            modified_unix: 0,
            kind: "file".to_string(),
            is_binary: None,
            git_status: None,
        }
    }

    #[test]
    fn staged_detection_preserves_exact_and_near_name_matches() {
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: 12_000,
            files: vec![
                file("one/report.md", "report.md", 1),
                file("two/report.txt", "report.txt", 10_000),
                file("three/report-2024.md", "report-2024.md", 512),
                file("four/report-2025.md", "report-2025.md", 513),
            ],
            issues: Vec::new(),
        };
        let options = DuplicateOptions {
            name_threshold: 0.90,
            ..DuplicateOptions::default()
        };

        let (groups, stats) = find_duplicates_with_stats(&index, &options);
        assert!(groups.iter().any(|group| {
            group.kind == "exact_name"
                && group.items == vec!["one/report.md".to_string(), "two/report.txt".to_string()]
        }));
        assert!(groups.iter().any(|group| {
            group.kind == "near_name"
                && group.items
                    == vec!["four/report-2025.md".to_string(), "three/report-2024.md".to_string()]
        }));
        assert!(stats.compared_pairs < stats.total_possible_pairs);
    }

    #[test]
    fn staged_detection_avoids_quadratic_comparisons_on_large_input() {
        let mut files = Vec::new();
        for index in 0..1_000 {
            files.push(file(
                &format!("data/item-{index:04}.txt"),
                &format!("item-{index:04}.txt"),
                32_768,
            ));
        }
        files.push(file("data/report-2024.md", "report-2024.md", 32_768));
        files.push(file("data/report-2025.md", "report-2025.md", 32_768));
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: files.iter().map(|file| file.size).sum(),
            files,
            issues: Vec::new(),
        };
        let options = DuplicateOptions {
            name_ngram_size: 2,
            max_ngram_postings: 16,
            ..DuplicateOptions::default()
        };

        let (groups, stats) = find_duplicates_with_stats(&index, &options);
        assert!(groups.iter().any(|group| {
            group.kind == "near_name"
                && group.items
                    == vec!["data/report-2024.md".to_string(), "data/report-2025.md".to_string()]
        }));
        assert!(stats.compared_pairs < stats.total_possible_pairs / 100);
        assert!(stats.skipped_high_frequency_postings > 0);
    }

    #[test]
    fn filters_are_composable() {
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: 10,
            files: vec![FileRecord {
                path: "docs/readme.md".into(),
                name: "readme.md".into(),
                extension: "md".into(),
                size: 10,
                modified_unix: 100,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            }],
            issues: Vec::new(),
        };
        let filters = FilterSet {
            all: vec![FileFilter::Extension("md".into()), FileFilter::MinSize(10)],
            any: vec![],
            custom: vec![CustomFilter {
                field: "path".into(),
                operator: CustomOperator::Contains,
                value: "docs".into(),
            }],
        };
        assert_eq!(filter_index(&index, &filters).len(), 1);
    }
}

#[cfg(test)]
mod measured_duplicate_default_tests {
    use super::*;

    #[test]
    fn duplicate_defaults_match_the_repeated_experiment_selection() {
        let defaults = DuplicateOptions::default();

        assert_eq!(defaults.size_bucket_bytes, 4 * 1024);
        assert_eq!(defaults.name_ngram_size, 3);
        assert_eq!(defaults.max_ngram_postings, 256);
        assert!((defaults.name_threshold - 0.90).abs() < f64::EPSILON);
        assert!(!defaults.include_extension);
    }
}

#[cfg(test)]
mod content_duplicate_tests {
    use super::*;

    fn temporary_directory(label: &str) -> std::path::PathBuf {
        let unique = format!(
            "bl1nk-kept-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        directory
    }

    #[test]
    fn content_scan_groups_equal_bytes_and_rejects_equal_size_different_bytes() {
        let directory = temporary_directory("content-scan");
        std::fs::write(directory.join("report-a.txt"), b"same payload for verified duplicate")
            .expect("first fixture must be written");
        std::fs::write(directory.join("report-b.txt"), b"same payload for verified duplicate")
            .expect("second fixture must be written");
        std::fs::write(
            directory.join("same-size-other.txt"),
            vec![b'x'; b"same payload for verified duplicate".len()],
        )
        .expect("different-content fixture must be written");

        let index = scan_directory(&directory, &ScanOptions::default())
            .expect("fixture directory must be indexed");
        let (groups, stats) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content scan must complete");

        assert!(groups.iter().any(|group| {
            group.kind == "same_content"
                && group.items == vec!["report-a.txt".to_string(), "report-b.txt".to_string()]
                && group.similarity == 1.0
        }));
        assert!(!groups.iter().any(|group| {
            group.kind == "same_content" && group.items.contains(&"same-size-other.txt".to_string())
        }));
        assert_eq!(stats.full_hash_files, 2);

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn partial_hash_collision_is_rejected_by_full_hash_verification() {
        let directory = temporary_directory("partial-hash-collision");
        let size = 256 * 1024;
        let mut data_a = vec![0xaa_u8; size];
        let mut data_b = vec![0xaa_u8; size];
        data_a[100 * 1024] = 0x11;
        data_b[100 * 1024] = 0x22;

        std::fs::write(directory.join("collision-a.bin"), &data_a)
            .expect("collision-a fixture must be written");
        std::fs::write(directory.join("collision-b.bin"), &data_b)
            .expect("collision-b fixture must be written");

        let index = scan_directory(&directory, &ScanOptions::default())
            .expect("fixture directory must be indexed");
        let (groups, stats) = find_content_duplicates(
            &index,
            &ContentDuplicateOptions { partial_hash_bytes: 64 * 1024 },
        )
        .expect("content scan must complete");

        assert_eq!(groups.len(), 0);
        assert_eq!(stats.partial_hash_files, 2);
        assert_eq!(stats.full_hash_files, 2);

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn unreadable_candidate_files_are_tracked_without_aborting_scan() {
        let directory = temporary_directory("unreadable-candidates");
        std::fs::write(directory.join("valid-a.txt"), b"duplicate content")
            .expect("valid-a fixture must be written");
        std::fs::write(directory.join("valid-b.txt"), b"duplicate content")
            .expect("valid-b fixture must be written");

        let mut index = scan_directory(&directory, &ScanOptions::default())
            .expect("fixture directory must be indexed");
        index.files.push(FileRecord {
            path: "non-existent-candidate.txt".to_string(),
            name: "non-existent-candidate.txt".to_string(),
            extension: "txt".to_string(),
            size: b"duplicate content".len() as u64,
            modified_unix: 0,
            kind: "file".to_string(),
            is_binary: None,
            git_status: None,
        });

        let (groups, stats) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content scan must complete despite unreadable file");

        assert_eq!(groups.len(), 1);
        assert_eq!(stats.unreadable_files, 1);
        assert_eq!(groups[0].items.len(), 2);

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn duplicate_mutation_simulation_blocks_unsafe_paths_and_executes_safe_trash_with_rollback() {
        let directory = temporary_directory("mutation-lifecycle");
        let primary_path = directory.join("a-primary.txt");
        let duplicate_path = directory.join("b-duplicate.txt");
        let git_protected = directory.join("repo").join(".git").join("c-git-dup.txt");

        std::fs::create_dir_all(directory.join("repo").join(".git"))
            .expect("git dir must be created");
        std::fs::write(&primary_path, b"identical data").expect("primary must be written");
        std::fs::write(&duplicate_path, b"identical data").expect("duplicate must be written");
        std::fs::write(&git_protected, b"identical data").expect("git file must be written");

        let index = scan_directory(
            &directory,
            &ScanOptions {
                include_hidden: true,
                max_depth: None,
            },
        )
        .expect("fixture directory must be indexed");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content duplicates must complete");

        let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Trash);
        assert!(!plan.actions.is_empty());

        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            protected_patterns: vec![".git".to_string()],
            allow_list: Vec::new(),
            backup_directory: Some(directory.join("trash-bin").display().to_string()),
            preserve_canonical: true,
            verify_checksum_before_action: true,
        };

        // 1. Simulate dry-run
        let simulation = simulate_duplicate_mutation(&plan, &index, &policy);
        assert!(!simulation.valid_actions.is_empty());
        assert!(simulation
            .blocked_actions
            .iter()
            .any(|b| b.action.target_path.contains(".git")));
        assert!(primary_path.exists());
        assert!(duplicate_path.exists());

        // 2. Filter plan to only valid actions and execute
        let safe_plan = DuplicateMutationPlan {
            root: plan.root.clone(),
            created_at_unix: plan.created_at_unix,
            actions: simulation.valid_actions.clone(),
            total_reclaimable_bytes: simulation.total_reclaimable_bytes,
        };

        let mut journal = execute_duplicate_mutation(&safe_plan, &index, &policy)
            .expect("safe mutation plan must execute successfully");

        assert!(primary_path.exists());
        assert!(!duplicate_path.exists());

        // 3. Rollback
        let restored = rollback_duplicate_mutation(&mut journal).expect("rollback must succeed");
        assert_eq!(restored, 1);
        assert!(primary_path.exists());
        assert!(duplicate_path.exists());
        assert_eq!(std::fs::read(&duplicate_path).unwrap(), b"identical data");

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn file_integrity_detects_bad_extension_mismatches() {
        let directory = temporary_directory("file-integrity");
        let png_magic = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0];
        let pdf_magic = b"%PDF-1.7 sample content";

        // Write valid PNG with .png extension
        std::fs::write(directory.join("valid.png"), png_magic).expect("valid png must be written");
        // Write PDF with mismatched .txt extension
        std::fs::write(directory.join("fake.txt"), pdf_magic).expect("fake txt must be written");

        let index = scan_directory(&directory, &ScanOptions::default())
            .expect("fixture directory must be indexed");
        let (bad_extensions, _) = scan_index_integrity(&index);

        assert_eq!(bad_extensions.len(), 1);
        assert_eq!(bad_extensions[0].path, "fake.txt");
        assert_eq!(bad_extensions[0].detected_extension, "pdf");
        assert_eq!(bad_extensions[0].actual_extension, "txt");

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn duplicate_allow_list_filters_known_benign_duplicates() {
        let groups = vec![
            DuplicateGroup {
                kind: "same_content".to_string(),
                similarity: 1.0,
                items: vec!["a/photo.jpg".to_string(), "b/photo-copy.jpg".to_string()],
                evidence: Some(DuplicateEvidence {
                    size_bytes: 100,
                    partial_sha256: "abc".to_string(),
                    full_sha256: "deadbeef".to_string(),
                }),
            },
            DuplicateGroup {
                kind: "same_content".to_string(),
                similarity: 1.0,
                items: vec!["c/doc.pdf".to_string(), "d/doc-copy.pdf".to_string()],
                evidence: Some(DuplicateEvidence {
                    size_bytes: 200,
                    partial_sha256: "def".to_string(),
                    full_sha256: "cafebabe".to_string(),
                }),
            },
        ];

        let allow_list = vec![DuplicateAllowRule {
            path_a: None,
            path_b: None,
            glob_pattern: None,
            sha256: Some("deadbeef".to_string()),
            reason: Some("approved mirror".to_string()),
        }];

        let filtered = filter_allowed_duplicates(groups, &allow_list);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].items[0], "c/doc.pdf");
    }

    // --- Mutation safety tests (DUP-001) ---

    #[test]
    fn mutation_blocks_delete_for_protected_path() {
        let directory = temporary_directory("mutation-delete-protected");
        let primary = directory.join("keep-this.txt");
        let dup = directory.join("repo").join(".git").join("dup.txt");

        std::fs::create_dir_all(dup.parent().unwrap()).expect("git dir must be created");
        std::fs::write(&primary, b"protected data").expect("primary must be written");
        std::fs::write(&dup, b"protected data").expect("dup must be written");

        let index = scan_directory(
            &directory,
            &ScanOptions {
                include_hidden: true,
                max_depth: None,
            },
        )
        .expect("index must be created");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content duplicates must complete");

        let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Delete);
        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            protected_patterns: vec![".git".to_string()],
            ..Default::default()
        };

        let simulation = simulate_duplicate_mutation(&plan, &index, &policy);
        assert!(simulation
            .blocked_actions
            .iter()
            .any(|b| b.action.target_path.contains(".git")
                && b.rejection_reason
                    .as_deref()
                    .is_some_and(|r| r.contains("protected"))));

        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }

    #[test]
    fn mutation_blocks_out_of_root_boundary() {
        let directory = temporary_directory("mutation-out-of-root");
        let primary = directory.join("canonical.txt");
        let outside_dir =
            std::env::temp_dir().join(format!("bl1nk-kept-oob-{}", std::process::id()));
        std::fs::create_dir_all(&outside_dir).expect("outside dir must be created");
        let dup = outside_dir.join("duplicate.txt");

        std::fs::write(&primary, b"root data").expect("primary must be written");
        std::fs::write(&dup, b"root data").expect("dup must be written");

        let index =
            scan_directory(&directory, &ScanOptions::default()).expect("index must be created");
        // Manually add the outside file to simulate cross-root duplicate
        let mut index_with_oob = index.clone();
        index_with_oob.files.push(FileRecord {
            path: dup.display().to_string(),
            name: "duplicate.txt".to_string(),
            extension: "txt".to_string(),
            size: 9,
            modified_unix: 0,
            kind: "file".to_string(),
            is_binary: None,
            git_status: None,
        });
        let groups = vec![DuplicateGroup {
            kind: "same_content".to_string(),
            similarity: 1.0,
            items: vec![
                primary
                    .strip_prefix(&directory)
                    .unwrap()
                    .display()
                    .to_string(),
                dup.display().to_string(),
            ],
            evidence: Some(DuplicateEvidence {
                size_bytes: 9,
                partial_sha256: "abc".to_string(),
                full_sha256: "def".to_string(),
            }),
        }];

        let plan =
            create_duplicate_mutation_plan(&index_with_oob, &groups, DuplicateActionKind::Trash);
        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            protected_patterns: vec![".git".to_string()],
            ..Default::default()
        };

        let simulation = simulate_duplicate_mutation(&plan, &index_with_oob, &policy);
        assert!(simulation.blocked_actions.iter().any(|b| b
            .rejection_reason
            .as_deref()
            .is_some_and(|r| r.contains("outside"))));

        let _ = std::fs::remove_dir_all(&outside_dir);
        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }

    #[test]
    fn mutation_blocks_on_disk_size_mismatch() {
        let directory = temporary_directory("mutation-size-mismatch");
        let primary = directory.join("a.txt");
        let dup = directory.join("b.txt");

        std::fs::write(&primary, b"original").expect("primary must be written");
        std::fs::write(&dup, b"original").expect("dup must be written");

        let index =
            scan_directory(&directory, &ScanOptions::default()).expect("index must be created");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content duplicates must complete");

        let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Trash);

        // Mutate the file on disk after plan creation
        std::fs::write(&dup, b"modified-after-plan").expect("dup must be modified");

        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            ..Default::default()
        };

        let simulation = simulate_duplicate_mutation(&plan, &index, &policy);
        assert!(!simulation.blocked_actions.is_empty());
        assert!(simulation.blocked_actions.iter().any(|b| b
            .rejection_reason
            .as_deref()
            .is_some_and(|r| r.contains("size"))));

        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }

    #[test]
    fn mutation_blocks_stale_index_size_mismatch() {
        let directory = temporary_directory("mutation-stale-index");
        let primary = directory.join("a.txt");
        let dup = directory.join("b.txt");

        std::fs::write(&primary, b"data123").expect("primary must be written");
        std::fs::write(&dup, b"data123").expect("dup must be written");

        let mut index =
            scan_directory(&directory, &ScanOptions::default()).expect("index must be created");

        // Tamper index: change recorded size of dup to wrong value
        for file in &mut index.files {
            if file.path == "b.txt" {
                file.size = 999;
            }
        }

        let groups_input = [DuplicateGroup {
            kind: "same_content".to_string(),
            similarity: 1.0,
            items: vec!["a.txt".to_string(), "b.txt".to_string()],
            evidence: None,
        }];
        let plan =
            create_duplicate_mutation_plan(&index, &groups_input, DuplicateActionKind::Trash);

        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            ..Default::default()
        };

        let simulation = simulate_duplicate_mutation(&plan, &index, &policy);
        assert!(!simulation.blocked_actions.is_empty());
        assert!(simulation.blocked_actions.iter().any(|b| b
            .rejection_reason
            .as_deref()
            .is_some_and(|r| r.contains("size mismatch"))));

        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }

    #[test]
    fn mutation_blocks_on_checksum_fail_when_file_modified() {
        let directory = temporary_directory("mutation-checksum-fail");
        let primary = directory.join("a.txt");
        let dup = directory.join("b.txt");

        std::fs::write(&primary, b"checksum data").expect("primary must be written");
        std::fs::write(&dup, b"checksum data").expect("dup must be written");

        let index =
            scan_directory(&directory, &ScanOptions::default()).expect("index must be created");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content duplicates must complete");

        let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Trash);

        // Modify file after plan — on-disk size check should catch this
        std::fs::write(&dup, b"CHANGED").expect("dup must be modified");

        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            verify_checksum_before_action: true,
            ..Default::default()
        };

        let result = execute_duplicate_mutation(&plan, &index, &policy);
        assert!(result.is_err());

        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }

    #[test]
    fn mutation_hardlink_creates_rollback_entry_with_backup() {
        let directory = temporary_directory("mutation-hardlink-rollback");
        let primary = directory.join("canonical.txt");
        let dup = directory.join("duplicate.txt");

        std::fs::write(&primary, b"hardlink content").expect("primary must be written");
        std::fs::write(&dup, b"hardlink content").expect("dup must be written");

        let index =
            scan_directory(&directory, &ScanOptions::default()).expect("index must be created");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content duplicates must complete");

        let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::HardLink);
        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            backup_directory: Some(directory.join("backups").display().to_string()),
            ..Default::default()
        };

        let mut journal =
            execute_duplicate_mutation(&plan, &index, &policy).expect("hardlink must execute");

        assert_eq!(journal.entries.len(), 1);
        assert!(journal.entries[0].backup_path.is_some());
        assert!(primary.exists());
        assert!(dup.exists());
        // After hardlink, both should point to same inode
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let meta_a = std::fs::metadata(&primary).unwrap();
            let meta_b = std::fs::metadata(&dup).unwrap();
            assert_eq!(meta_a.ino(), meta_b.ino());
        }

        // Rollback should restore original file content
        let restored = rollback_duplicate_mutation(&mut journal).expect("rollback must succeed");
        assert_eq!(restored, 1);
        assert!(primary.exists());
        assert!(dup.exists());

        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }

    #[test]
    fn mutation_delete_execution_and_rollback() {
        let directory = temporary_directory("mutation-delete-exec");
        let primary = directory.join("a.txt");
        let dup = directory.join("b.txt");

        std::fs::write(&primary, b"delete me").expect("primary must be written");
        std::fs::write(&dup, b"delete me").expect("dup must be written");

        let index =
            scan_directory(&directory, &ScanOptions::default()).expect("index must be created");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content duplicates must complete");

        let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Delete);
        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            backup_directory: Some(directory.join("backup").display().to_string()),
            ..Default::default()
        };

        let mut journal =
            execute_duplicate_mutation(&plan, &index, &policy).expect("delete must execute");

        assert!(primary.exists());
        assert!(!dup.exists());
        assert_eq!(journal.entries.len(), 1);
        assert!(journal.entries[0].backup_path.is_some());

        // Rollback restores the deleted file
        let restored = rollback_duplicate_mutation(&mut journal).expect("rollback must succeed");
        assert_eq!(restored, 1);
        assert!(dup.exists());
        assert_eq!(std::fs::read(&dup).unwrap(), b"delete me");

        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }

    #[test]
    fn mutation_allow_list_skips_allowed_groups() {
        let directory = temporary_directory("mutation-allow-list");
        let primary = directory.join("a.txt");
        let dup = directory.join("b.txt");

        std::fs::write(&primary, b"allowed data").expect("primary must be written");
        std::fs::write(&dup, b"allowed data").expect("dup must be written");

        let index =
            scan_directory(&directory, &ScanOptions::default()).expect("index must be created");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content duplicates must complete");

        let plan = create_duplicate_mutation_plan(&index, &groups, DuplicateActionKind::Trash);
        let policy = DuplicateMutationPolicy {
            allowed_roots: vec![directory.display().to_string()],
            allow_list: vec![DuplicateAllowRule {
                path_a: None,
                path_b: None,
                glob_pattern: Some("a.txt".to_string()),
                sha256: None,
                reason: Some("approved".to_string()),
            }],
            ..Default::default()
        };

        let simulation = simulate_duplicate_mutation(&plan, &index, &policy);
        // All actions should be skipped by allow_list
        assert!(simulation.valid_actions.is_empty());
        assert!(simulation.blocked_actions.is_empty());

        std::fs::remove_dir_all(directory).expect("cleanup must succeed");
    }
}

#[cfg(all(test, unix))]
mod mixed_content_duplicate_tests {
    use super::*;

    #[test]
    fn content_scan_reports_hard_link_subgroup_when_copies_share_the_same_hash() {
        let directory = std::env::temp_dir().join(format!(
            "bl1nk-kept-mixed-content-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        std::fs::write(directory.join("source.txt"), b"same verified payload")
            .expect("source fixture must be written");
        std::fs::hard_link(directory.join("source.txt"), directory.join("source-link.txt"))
            .expect("hard link fixture must be created");
        std::fs::copy(directory.join("source.txt"), directory.join("copy.txt"))
            .expect("copy fixture must be created");

        let index = scan_directory(&directory, &ScanOptions::default())
            .expect("fixture directory must be indexed");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content scan must complete");

        assert!(groups.iter().any(|group| {
            group.kind == "same_content"
                && group.items
                    == vec![
                        "copy.txt".to_string(),
                        "source-link.txt".to_string(),
                        "source.txt".to_string(),
                    ]
        }));
        assert!(groups.iter().any(|group| {
            group.kind == "hard_link"
                && group.items == vec!["source-link.txt".to_string(), "source.txt".to_string()]
                && group
                    .evidence
                    .as_ref()
                    .is_some_and(|evidence| !evidence.full_sha256.is_empty())
        }));

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(test)]
mod persistent_index_tests {
    use super::{plan_incremental_refresh, FileRecord, ScanIndex};

    fn record(path: &str, size: u64, modified_unix: u64) -> FileRecord {
        FileRecord {
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            extension: "txt".to_string(),
            size,
            modified_unix,
            kind: "file".to_string(),
            is_binary: None,
            git_status: None,
        }
    }

    #[test]
    fn incremental_refresh_classifies_added_modified_removed_and_unchanged_files() {
        let previous = ScanIndex {
            root: "/fixture".to_string(),
            scanned_at_unix: 10,
            total_size: 3,
            files: vec![record("a.txt", 1, 1), record("gone.txt", 1, 1), record("same.txt", 1, 1)],
            issues: Vec::new(),
        };
        let current = ScanIndex {
            root: "/fixture".to_string(),
            scanned_at_unix: 20,
            total_size: 4,
            files: vec![record("a.txt", 2, 2), record("new.txt", 1, 1), record("same.txt", 1, 1)],
            issues: Vec::new(),
        };

        let plan = plan_incremental_refresh(&previous, &current)
            .expect("matching roots must produce a refresh plan");

        assert_eq!(plan.added, vec!["new.txt"]);
        assert_eq!(plan.modified, vec!["a.txt"]);
        assert_eq!(plan.removed, vec!["gone.txt"]);
        assert_eq!(plan.unchanged, vec!["same.txt"]);
    }
}

#[cfg(test)]
mod persistent_snapshot_tests {
    use super::{create_persistent_snapshot, FileRecord, ScanIndex, ScanOptions};

    #[test]
    fn persistent_snapshot_binds_index_to_root_and_scan_options() {
        let index = ScanIndex {
            root: "/fixture/root".to_string(),
            scanned_at_unix: 100,
            total_size: 10,
            files: vec![FileRecord {
                path: "file.txt".to_string(),
                name: "file.txt".to_string(),
                extension: "txt".to_string(),
                size: 10,
                modified_unix: 100,
                kind: "file".to_string(),
                is_binary: None,
                git_status: None,
            }],
            issues: Vec::new(),
        };
        let options = ScanOptions {
            include_hidden: true,
            max_depth: Some(3),
        };

        let snapshot = create_persistent_snapshot(index, &options);

        assert_eq!(snapshot.schema_version, "1.2.0");
        assert_eq!(snapshot.root, "/fixture/root");
        assert!(snapshot.include_hidden);
        assert_eq!(snapshot.max_depth, Some(3));
        assert!(!snapshot.root_fingerprint.is_empty());
    }
}
