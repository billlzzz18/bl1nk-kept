//! Comprehensive unit tests for kept-core::scanner::duplicate
//!
//! Covers: DuplicateOptions defaults, DuplicateAllowRule matching,
//! filter_allowed_duplicates, normalized_similarity, find_duplicates,
//! find_duplicates_with_stats, name ngrams, hash_hex.

use kept_core::scanner::normalized_similarity;
use kept_core::{
    filter_allowed_duplicates, find_duplicates, find_duplicates_with_stats,
    ContentDuplicateOptions, DuplicateAllowRule, DuplicateEvidence, DuplicateGroup,
    DuplicateOptions, FileRecord, ScanIndex,
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

// ─── DuplicateOptions defaults ──────────────────────────────────────────────

#[test]
fn default_options_has_sensible_values() {
    let opts = DuplicateOptions::default();
    assert_eq!(opts.name_threshold, 0.90);
    assert!(!opts.include_extension);
    assert_eq!(opts.size_bucket_bytes, 4096);
    assert_eq!(opts.name_ngram_size, 3);
    assert_eq!(opts.max_ngram_postings, 256);
}

// ─── DuplicateAllowRule matching ────────────────────────────────────────────

#[test]
fn allow_rule_matches_by_sha256() {
    let rule = DuplicateAllowRule {
        path_a: None,
        path_b: None,
        glob_pattern: None,
        sha256: Some("abc123".to_string()),
        reason: None,
    };
    let group = DuplicateGroup {
        kind: "same_content".to_string(),
        similarity: 1.0,
        items: vec!["a.txt".to_string(), "b.txt".to_string()],
        evidence: Some(DuplicateEvidence {
            size_bytes: 100,
            partial_sha256: "aaa".to_string(),
            full_sha256: "ABC123".to_string(),
        }),
    };
    assert!(rule.matches_group(&group));
}

#[test]
fn allow_rule_matches_by_path_pair() {
    let rule = DuplicateAllowRule {
        path_a: Some("a.txt".to_string()),
        path_b: Some("b.txt".to_string()),
        glob_pattern: None,
        sha256: None,
        reason: None,
    };
    let group = DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec!["a.txt".to_string(), "b.txt".to_string()],
        evidence: None,
    };
    assert!(rule.matches_group(&group));
}

#[test]
fn allow_rule_matches_by_glob_pattern() {
    let rule = DuplicateAllowRule {
        path_a: None,
        path_b: None,
        glob_pattern: Some("test".to_string()),
        sha256: None,
        reason: None,
    };
    let group = DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec!["test_data.txt".to_string(), "test_fixture.txt".to_string()],
        evidence: None,
    };
    assert!(rule.matches_group(&group));
}

#[test]
fn allow_rule_no_match_when_nothing_matches() {
    let rule = DuplicateAllowRule {
        path_a: Some("x.txt".to_string()),
        path_b: Some("y.txt".to_string()),
        glob_pattern: None,
        sha256: None,
        reason: None,
    };
    let group = DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec!["a.txt".to_string(), "b.txt".to_string()],
        evidence: None,
    };
    assert!(!rule.matches_group(&group));
}

#[test]
fn allow_rule_glob_requires_all_items_to_match() {
    let rule = DuplicateAllowRule {
        path_a: None,
        path_b: None,
        glob_pattern: Some("test".to_string()),
        sha256: None,
        reason: None,
    };
    let group = DuplicateGroup {
        kind: "exact_name".to_string(),
        similarity: 1.0,
        items: vec!["test_data.txt".to_string(), "other_file.txt".to_string()],
        evidence: None,
    };
    assert!(!rule.matches_group(&group));
}

// ─── filter_allowed_duplicates ──────────────────────────────────────────────

#[test]
fn filter_empty_allow_list_returns_all_groups() {
    let groups = vec![
        DuplicateGroup {
            kind: "exact_name".to_string(),
            similarity: 1.0,
            items: vec!["a.txt".to_string(), "b.txt".to_string()],
            evidence: None,
        },
        DuplicateGroup {
            kind: "near_name".to_string(),
            similarity: 0.95,
            items: vec!["c.txt".to_string(), "d.txt".to_string()],
            evidence: None,
        },
    ];
    let result = filter_allowed_duplicates(groups.clone(), &[]);
    assert_eq!(result.len(), 2);
}

#[test]
fn filter_removes_matching_groups() {
    let groups = vec![
        DuplicateGroup {
            kind: "same_content".to_string(),
            similarity: 1.0,
            items: vec!["a.txt".to_string(), "b.txt".to_string()],
            evidence: Some(DuplicateEvidence {
                size_bytes: 100,
                partial_sha256: "aaa".to_string(),
                full_sha256: "abc123".to_string(),
            }),
        },
        DuplicateGroup {
            kind: "near_name".to_string(),
            similarity: 0.95,
            items: vec!["c.txt".to_string(), "d.txt".to_string()],
            evidence: None,
        },
    ];
    let allow_list = vec![DuplicateAllowRule {
        path_a: Some("a.txt".to_string()),
        path_b: Some("b.txt".to_string()),
        glob_pattern: None,
        sha256: None,
        reason: None,
    }];
    let result = filter_allowed_duplicates(groups, &allow_list);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].kind, "near_name");
}

// ─── normalized_similarity ──────────────────────────────────────────────────

#[test]
fn similarity_identical_strings_is_one() {
    assert_eq!(normalized_similarity("hello", "hello"), 1.0);
}

#[test]
fn similarity_empty_strings_is_one() {
    assert_eq!(normalized_similarity("", ""), 1.0);
}

#[test]
fn similarity_completely_different_strings() {
    let sim = normalized_similarity("aaaa", "bbbb");
    assert!(sim < 0.5, "completely different strings should have low similarity");
}

#[test]
fn similarity_one_char_different() {
    let sim = normalized_similarity("hello", "hallo");
    assert!(sim > 0.7, "one char difference should have high similarity");
}

#[test]
fn similarity_is_symmetric() {
    let a = normalized_similarity("abc", "abd");
    let b = normalized_similarity("abd", "abc");
    assert!((a - b).abs() < f64::EPSILON);
}

#[test]
fn similarity_one_empty_one_nonempty() {
    let sim = normalized_similarity("", "hello");
    assert!(sim < 0.5);
}

// ─── find_duplicates ────────────────────────────────────────────────────────

#[test]
fn find_duplicates_exact_name_group() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/a/report.txt", "report.txt", 100),
            make_file("/work/b/report.txt", "report.txt", 200),
        ],
    );
    let opts = DuplicateOptions::default();
    let groups = find_duplicates(&index, &opts);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].kind, "exact_name");
    assert_eq!(groups[0].similarity, 1.0);
    assert_eq!(groups[0].items.len(), 2);
}

#[test]
fn find_duplicates_no_groups_for_unique_names() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/a.txt", "alpha.txt", 100),
            make_file("/work/b.txt", "beta.txt", 100),
            make_file("/work/c.txt", "gamma.txt", 100),
        ],
    );
    let opts = DuplicateOptions::default();
    let groups = find_duplicates(&index, &opts);
    assert!(groups.is_empty());
}

#[test]
fn find_duplicates_near_name_group() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/report_final.txt", "report_final.txt", 100),
            make_file("/work/report_final_v2.txt", "report_final_v2.txt", 110),
        ],
    );
    let opts = DuplicateOptions {
        name_threshold: 0.60, // Lower threshold to catch near-matches
        ..DuplicateOptions::default()
    };
    let groups = find_duplicates(&index, &opts);
    let near_name_groups: Vec<_> = groups.iter().filter(|g| g.kind == "near_name").collect();
    assert!(!near_name_groups.is_empty(), "should detect near-name duplicates");
}

#[test]
fn find_duplicates_include_extension_option() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/a/file.rs", "file.rs", 100),
            make_file("/work/b/file.txt", "file.txt", 100),
        ],
    );
    // Without include_extension, stems "file" match exactly
    let opts_no_ext = DuplicateOptions {
        include_extension: false,
        ..DuplicateOptions::default()
    };
    let groups_no_ext = find_duplicates(&index, &opts_no_ext);
    assert_eq!(groups_no_ext.len(), 1);
    assert_eq!(groups_no_ext[0].kind, "exact_name");

    // With include_extension, "file.rs" != "file.txt"
    let opts_with_ext = DuplicateOptions {
        include_extension: true,
        ..DuplicateOptions::default()
    };
    let groups_with_ext = find_duplicates(&index, &opts_with_ext);
    assert!(groups_with_ext.is_empty());
}

#[test]
fn find_duplicates_case_insensitive() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/Report.TXT", "Report.TXT", 100),
            make_file("/work/report.txt", "report.txt", 100),
        ],
    );
    let opts = DuplicateOptions::default();
    let groups = find_duplicates(&index, &opts);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].kind, "exact_name");
}

#[test]
fn find_duplicates_sorted_by_similarity_descending() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/a.txt", "aaaa.txt", 100),
            make_file("/work/b.txt", "aaab.txt", 100),
            make_file("/work/c.txt", "aabb.txt", 100),
        ],
    );
    let opts = DuplicateOptions {
        name_threshold: 0.50,
        ..DuplicateOptions::default()
    };
    let groups = find_duplicates(&index, &opts);
    // All near-name groups should be sorted by similarity descending
    for window in groups.windows(2) {
        assert!(window[0].similarity >= window[1].similarity);
    }
}

#[test]
fn find_duplicates_with_stats_returns_correct_stats() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/a/report.txt", "report.txt", 100),
            make_file("/work/b/report.txt", "report.txt", 200),
        ],
    );
    let opts = DuplicateOptions::default();
    let (groups, stats) = find_duplicates_with_stats(&index, &opts);
    assert_eq!(stats.file_count, 2);
    assert_eq!(stats.total_possible_pairs, 1);
    assert_eq!(groups.len(), 1);
    // exact_name match: compared_pairs stays 0 since names are identical
    assert_eq!(stats.compared_pairs, 0);
}

// ─── ContentDuplicateOptions ────────────────────────────────────────────────

#[test]
fn content_duplicate_options_default() {
    let opts = ContentDuplicateOptions::default();
    assert_eq!(opts.partial_hash_bytes, 64 * 1024);
}

// ─── Empty index edge cases ─────────────────────────────────────────────────

#[test]
fn find_duplicates_empty_index() {
    let index = make_index("/work", vec![]);
    let opts = DuplicateOptions::default();
    let groups = find_duplicates(&index, &opts);
    assert!(groups.is_empty());
}

#[test]
fn find_duplicates_single_file() {
    let index = make_index("/work", vec![make_file("/work/a.txt", "a.txt", 100)]);
    let opts = DuplicateOptions::default();
    let groups = find_duplicates(&index, &opts);
    assert!(groups.is_empty());
}

#[test]
fn find_duplicates_sorted_items_within_group() {
    let index = make_index(
        "/work",
        vec![
            make_file("/work/z/report.txt", "report.txt", 100),
            make_file("/work/a/report.txt", "report.txt", 200),
            make_file("/work/m/report.txt", "report.txt", 300),
        ],
    );
    let opts = DuplicateOptions::default();
    let groups = find_duplicates(&index, &opts);
    assert_eq!(groups.len(), 1);
    // Items should be sorted alphabetically
    assert_eq!(
        groups[0].items,
        vec!["/work/a/report.txt", "/work/m/report.txt", "/work/z/report.txt"]
    );
}
