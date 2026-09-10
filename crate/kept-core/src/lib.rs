pub mod analyzer;
pub mod content_router;
pub mod context;
pub mod error;
pub mod factory;
pub mod foundation;
pub mod judgment;
pub mod keyboard;
pub mod migration;
pub mod observation;
pub mod operation;
pub mod policy;
pub mod regret_tracker;
pub mod scanner;
pub mod schema;
pub mod search;
pub mod semantic;
pub mod source_graph;
pub mod token_counter;
pub mod validator;

pub use analyzer::RegistryAnalyzer;
pub use error::ValidationError;
pub use factory::{FactoryError, OperationFactory, UserIntent};
pub use foundation::{
    classify_evidence, decode_utf8, import_anonymized_jsonl, normalize_text,
    select_default_candidate, summarize_measurements, validate_corpus_manifest,
    validate_glossary_term, validate_regex_rule, verify_corpus_bytes, AnonymizedEvidence,
    ClassificationDecision, ClassificationPolicy, ClassificationResult, CorpusManifest,
    CorpusManifestEntry, DefaultCandidate, DimensionScores, DistributionSummary, EvidenceKind,
    EvidenceRecord, FoundationError, GlossaryTerm, ImportRejection, ImportReport, ProvenanceRecord,
    RegexRule, RegexScope, RegexTestVector, RuleSeverity, RunMeasurement, SelectionConstraints,
};
pub use judgment::{AgentJudgment, JudgmentError, JudgmentResult};
pub use migration::{migrate_registry, MigrationError, CURRENT_REGISTRY_VERSION};
pub use operation::{
    ExpectedEffect, MoveChange, NamingIssueItem, ObservedState, OperationContract, RenameChange,
    ValidationError as OperationValidationError,
};
pub use policy::{
    analyze_index_naming, analyze_naming, create_user_config_if_missing, default_user_config_path,
    load_user_config, resolve_naming_rule, save_user_config, ConfigDefaults, LengthSettings,
    NamingFinding, NamingIssue, NamingIssueKind, NamingProfile, NamingScope, NamingSettings,
    NumberSettings, PolicyError, PrefixSettings, RangeLimit, ResolvedNamingRule, ScopeOverrides,
    SemanticSearchSettings, SimilarityRule, SimilaritySettings, TokenPosition, TokenReplacement,
    TokenReposition, UserConfig, WhitespaceSettings, WordSettings, USER_CONFIG_FILE_NAME,
    USER_CONFIG_VERSION,
};
pub use scanner::{
    apply_refresh_plan, build_treemap, check_file_extension_integrity, compile_query,
    create_duplicate_mutation_plan, create_persistent_snapshot, execute_duplicate_mutation,
    filter_allowed_duplicates, filter_index, find_content_duplicates, find_duplicates,
    find_duplicates_with_stats, migrate_snapshot, parse_size_to_bytes, plan_incremental_refresh,
    rollback_duplicate_mutation, scan_directory, scan_index_integrity, simulate_duplicate_mutation,
    ActionSimulation, BadExtensionIssue, ContentDuplicateOptions, ContentDuplicateStats,
    CustomFilter, CustomOperator, DuplicateActionKind, DuplicateAllowRule, DuplicateEvidence,
    DuplicateGroup, DuplicateMutationPlan, DuplicateMutationPolicy, DuplicateOptions,
    DuplicatePlanAction, DuplicateSearchStats, DuplicateSimulationResult, FileFilter, FileRecord,
    FilterSet, IntegrityStatus, PersistentScanSnapshot, QueryClause, QueryPlan, RefreshPlan,
    RollbackEntry, RollbackJournal, ScanIndex, ScanIssue, ScanIssueKind, ScanOptions, TreemapNode,
    CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION,
};
pub use search::KeywordSearch;
pub use source_graph::{
    coalesce_file_events, load_index, save_index, DefinitionKind, FileEvent, FileEventKind,
    GraphBuilder, GraphEvent, GraphIndex, GraphManager, ImplementationRecord, ImportRecord,
    IndexBuilder, IndexBuilderConfig, IndexManager, IndexManagerConfig, KeptGraphIndexConfig,
    ReferenceKind, ScopeGraphIndex, SourceFileEvent, SourceGraph, SourceGraphStats,
    SymbolDefinition, SymbolReference,
};
pub use validator::Validator;

use crate::schema::KeywordRegistry;
use std::fs;
use std::path::Path;

// NOTE-001: โหลด registry โดยรองรับทั้ง JSON และ YAML อัตโนมัติจากนามสกุลไฟล์
pub fn load_registry<P: AsRef<Path>>(
    path: P,
) -> Result<KeywordRegistry, Box<dyn std::error::Error + Send + Sync>> {
    let path = path.as_ref();

    // ป้องกัน Path Traversal เบื้องต้น
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Access denied: Illegal path components".into());
    }

    let content = fs::read_to_string(path)?;

    let registry: KeywordRegistry = if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
            serde_yaml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        }
    } else {
        serde_json::from_str(&content)?
    };

    let migrated = migrate_registry(registry)?;
    let validator = Validator::new(migrated.clone());
    validator.validate_search_policy().map_err(|errors| {
        let msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        msg
    })?;

    Ok(migrated)
}

// NOTE-002: บันทึก registry โดยรองรับทั้ง JSON และ YAML
pub fn save_registry<P: AsRef<Path>>(
    path: P,
    registry: &KeywordRegistry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = path.as_ref();

    if registry.version != CURRENT_REGISTRY_VERSION || registry.foundation.is_none() {
        return Err(format!(
            "Registry must use current schema version '{CURRENT_REGISTRY_VERSION}' with a foundation profile before saving"
        )
        .into());
    }

    // ป้องกัน Path Traversal เบื้องต้น
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Access denied: Illegal path components".into());
    }

    let is_yaml = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
        .unwrap_or(false);

    let content = if is_yaml {
        serde_yaml::to_string(registry)?
    } else {
        serde_json::to_string_pretty(registry)?
    };

    fs::write(path, content)?;
    Ok(())
}

// NOTE-003: นำเข้าข้อมูลจาก CSV เพื่อสร้างกลุ่มและรายการอัตโนมัติ
pub fn import_csv<P: AsRef<Path>>(
    path: P,
    group_id: &str,
    group_name: &str,
) -> Result<KeywordRegistry, Box<dyn std::error::Error + Send + Sync>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut entries = Vec::new();

    for result in rdr.deserialize() {
        let mut fields: std::collections::HashMap<String, String> = result?;
        let id = fields
            .remove("id")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "CSV row is missing required id",
                )
            })?;
        let aliases = fields
            .remove("aliases")
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "CSV row is missing required aliases",
                )
            })?
            .split('|')
            .map(str::trim)
            .filter(|alias| !alias.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if aliases.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "CSV row has no usable aliases",
            )
            .into());
        }

        // NOTE-004: custom CSV columns ต้องยังอยู่ใน entry แต่ aliases ต้องเป็น array ตาม schema contract ไม่ใช่ string ดิบจาก CSV
        let mut entry = serde_json::Map::new();
        entry.insert("id".to_string(), serde_json::Value::String(id));
        entry.insert(
            "aliases".to_string(),
            serde_json::Value::Array(aliases.into_iter().map(serde_json::Value::String).collect()),
        );
        for (field, value) in fields {
            entry.insert(field, serde_json::Value::String(value));
        }
        entries.push(serde_json::Value::Object(entry));
    }

    let mut base_fields = std::collections::HashMap::new();
    base_fields.insert(
        "id".to_string(),
        schema::FieldSchema {
            field_type: "string".to_string(),
            required: Some(true),
            description: "Unique ID".to_string(),
            ..Default::default()
        },
    );
    base_fields.insert(
        "aliases".to_string(),
        schema::FieldSchema {
            field_type: "array".to_string(),
            item_type: Some("string".to_string()),
            required: Some(true),
            description: "Search aliases".to_string(),
            ..Default::default()
        },
    );

    let group = schema::KeywordGroup {
        group_id: group_id.to_string(),
        group_name: group_name.to_string(),
        description: format!("Imported from CSV: {}", group_name),
        base_fields_schema: base_fields,
        custom_field_allowed: schema::CustomFieldConfig {
            enabled: true,
            ..Default::default()
        },
        entries,
        group_stats: None,
    };

    Ok(KeywordRegistry {
        version: "1.1.0".to_string(),
        metadata: schema::Metadata {
            last_updated: chrono::Utc::now().to_rfc3339(),
            description: "Imported Registry".to_string(),
            owner: "system".to_string(),
            ..Default::default()
        },
        groups: vec![group],
        validation: schema::ValidationConfig::default(),
        synonym_sets: Vec::new(),
        index: None,
        search_policy: None,
        foundation: None,
    })
}

// NOTE-005: สร้างเอกสาร Markdown จาก registry
pub fn generate_markdown(registry: &KeywordRegistry) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", registry.metadata.description));
    md.push_str(&format!("**Owner:** {}\n", registry.metadata.owner));
    md.push_str(&format!("**Last Updated:** {}\n\n", registry.metadata.last_updated));

    for group in &registry.groups {
        md.push_str(&format!("## Group: {} ({})\n", group.group_name, group.group_id));
        md.push_str(&format!("{}\n\n", group.description));

        md.push_str("| ID | Description | Aliases |\n");
        md.push_str("|---|---|---|\n");

        for entry in &group.entries {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let desc = entry
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let aliases = entry
                .get("aliases")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "-".to_string());

            md.push_str(&format!("| `{}` | {} | {} |\n", id, desc, aliases));
        }
        md.push('\n');
    }

    md.push_str("---\n*Generated by bl1nk-kept*\n");
    md
}

#[cfg(test)]
mod foundation_migration_tests {
    use super::*;

    #[test]
    fn csv_import_materializes_required_id_and_aliases_fields() {
        let directory =
            std::env::temp_dir().join(format!("kept-csv-import-{}", std::process::id()));
        let csv_path = directory.join("keywords.csv");
        fs::remove_dir_all(&directory).ok();
        fs::create_dir_all(&directory).expect("temporary import fixture must be created");
        fs::write(&csv_path, "id,aliases\nreport,report|รายงาน\n")
            .expect("CSV fixture must be written");

        let registry =
            import_csv(&csv_path, "fixtures", "Fixtures").expect("CSV import must succeed");
        let entry = &registry.groups[0].entries[0];

        assert_eq!(entry["id"], "report");
        assert_eq!(entry["aliases"], serde_json::json!(["report", "รายงาน"]));
        fs::remove_dir_all(directory).expect("temporary import fixture must be removed");
    }

    #[test]
    fn load_registry_migrates_legacy_schema_before_returning_it() {
        let directory = std::env::temp_dir().join(format!(
            "bl1nk-kept-foundation-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory must be created");
        let csv_path = directory.join("legacy.csv");
        let registry_path = directory.join("legacy-registry.json");
        fs::write(&csv_path, "id,aliases\nlegacy-keyword,legacy alias\n")
            .expect("legacy CSV fixture must be written");
        let legacy = import_csv(&csv_path, "legacy", "Legacy")
            .expect("legacy registry fixture must be imported");
        assert_eq!(legacy.version, "1.1.0");
        fs::write(
            &registry_path,
            serde_json::to_string_pretty(&legacy).expect("legacy fixture must serialize"),
        )
        .expect("legacy registry fixture must be written");

        let migrated = load_registry(&registry_path).expect("legacy registry must load");

        assert_eq!(migrated.version, "1.2.0");
        let foundation = migrated
            .foundation
            .as_ref()
            .expect("migration must add foundation profile");
        assert_eq!(foundation.normalization.encoding, "utf-8");
        assert_eq!(foundation.normalization.unicode_form, "nfc");
        assert_eq!(foundation.normalization.whitespace_policy, "collapse");
        assert!(foundation.glossary_terms.is_empty());
        assert!(foundation.provenance_records.is_empty());
        assert!(foundation.regex_rules.is_empty());
        assert!(foundation.classification_policy.is_none());
        assert!(foundation.corpus_manifest.is_none());
        fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(test)]
mod foundation_save_boundary_tests {
    use super::*;

    #[test]
    fn save_registry_rejects_legacy_version_without_foundation_profile() {
        let directory = std::env::temp_dir().join(format!(
            "bl1nk-kept-foundation-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory must be created");
        let csv_path = directory.join("legacy.csv");
        let output_path = directory.join("registry.json");
        fs::write(&csv_path, "id,aliases\nlegacy-keyword,legacy alias\n")
            .expect("legacy CSV fixture must be written");
        let legacy = import_csv(&csv_path, "legacy", "Legacy")
            .expect("legacy registry fixture must be imported");

        let error = save_registry(&output_path, &legacy)
            .expect_err("saving an unmigrated legacy registry must fail");

        assert!(error.to_string().contains("current schema version"));
        assert!(!output_path.exists());
        fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(test)]
mod foundation_normalization_tests {
    use crate::foundation::normalize_text;

    #[test]
    fn normalization_uses_nfc_collapses_whitespace_and_lowercases_latin_only() {
        let normalized =
            normalize_text("  CAFÉ\tไทย\nA\u{0301}  ").expect("valid UTF-8 text must normalize");

        assert_eq!(normalized, "café ไทย á");
    }
}

#[cfg(test)]
mod foundation_release_version_tests {
    #[test]
    fn foundation_release_uses_workspace_version() {
        // NOTE: test verifies version sync between Cargo.toml and compile-time env
        let expected = env!("CARGO_PKG_VERSION");
        assert!(expected.starts_with("0.4"), "version should be in 0.4.x series; got {}", expected);
    }
}
