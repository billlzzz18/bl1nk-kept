use crate::foundation::{
    ClassificationPolicy, CorpusManifest, GlossaryTerm, ProvenanceRecord, RegexRule,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// NOTE-001: สร้าง JSON Schema จาก model registry ปัจจุบันเพื่อให้ artifact ภายนอกไม่ drift จาก Rust source
pub fn export_keyword_registry_schema() -> schemars::schema::RootSchema {
    schemars::schema_for!(KeywordRegistry)
}

// ============= Registry Types =============

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeywordRegistry {
    pub version: String,
    pub metadata: Metadata,
    pub groups: Vec<KeywordGroup>,
    pub validation: ValidationConfig,
    #[serde(default, rename = "synonymSets", skip_serializing_if = "Vec::is_empty")]
    pub synonym_sets: Vec<SynonymSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<RegistryIndex>,
    #[serde(
        default,
        rename = "searchPolicy",
        skip_serializing_if = "Option::is_none"
    )]
    pub search_policy: Option<SearchPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foundation: Option<FoundationProfile>,
}

/// NOTE-001: profile ที่ตรึง normalization/classification/corpus revision เพื่อให้ผลตรวจซ้ำได้
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct FoundationProfile {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub normalization: NormalizationProfile,
    #[serde(
        default,
        rename = "glossaryTerms",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub glossary_terms: Vec<GlossaryTerm>,
    #[serde(
        default,
        rename = "provenanceRecords",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub provenance_records: Vec<ProvenanceRecord>,
    #[serde(default, rename = "regexRules", skip_serializing_if = "Vec::is_empty")]
    pub regex_rules: Vec<RegexRule>,
    #[serde(
        default,
        rename = "classificationPolicy",
        skip_serializing_if = "Option::is_none"
    )]
    pub classification_policy: Option<ClassificationPolicy>,
    #[serde(
        default,
        rename = "corpusManifest",
        skip_serializing_if = "Option::is_none"
    )]
    pub corpus_manifest: Option<CorpusManifest>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// NOTE-001: policy ถูกบันทึกใน registry เพื่อให้ normalize ข้อมูลรอบหลังได้เหมือนรอบเดิม
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NormalizationProfile {
    pub encoding: String,
    #[serde(rename = "unicodeForm")]
    pub unicode_form: String,
    #[serde(rename = "whitespacePolicy")]
    pub whitespace_policy: String,
    #[serde(rename = "casePolicy")]
    pub case_policy: String,
    #[serde(rename = "scriptPolicy")]
    pub script_policy: String,
}

impl Default for NormalizationProfile {
    fn default() -> Self {
        Self {
            encoding: "utf-8".to_string(),
            unicode_form: "nfc".to_string(),
            whitespace_policy: "collapse".to_string(),
            case_policy: "lowercase_latin".to_string(),
            script_policy: "preserve_non_latin".to_string(),
        }
    }
}

/// NOTE-001: ค่าค้นหาเป็น policy ของเจ้าของ registry; default คงผลเดิมที่ผ่าน benchmark แล้วแต่ไม่บังคับทุก dataset
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchPolicy {
    #[serde(rename = "fuzzyMinSimilarity")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub fuzzy_min_similarity: f64,
    #[serde(rename = "fuzzyCandidateLimit")]
    pub fuzzy_candidate_limit: usize,
    #[serde(rename = "fuzzyNgramSize")]
    pub fuzzy_ngram_size: usize,
    #[serde(rename = "maxFuzzyNgramPostings")]
    pub max_fuzzy_ngram_postings: usize,
}

impl Default for SearchPolicy {
    fn default() -> Self {
        Self {
            fuzzy_min_similarity: 0.30,
            fuzzy_candidate_limit: 1_024,
            fuzzy_ngram_size: 2,
            max_fuzzy_ngram_postings: 4_096,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SynonymSet {
    pub term: String,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub homographs: Vec<String>,
    #[serde(default)]
    pub homophones: Vec<String>,
    #[serde(default, rename = "semanticSynonyms")]
    pub semantic_synonyms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegistryIndex {
    #[serde(rename = "lastIndexed")]
    pub last_indexed: String,
    #[serde(default)]
    pub synonyms: HashMap<String, Vec<String>>,
    #[serde(default, rename = "homographs")]
    pub homographs: HashMap<String, Vec<String>>,
    #[serde(default, rename = "homophones")]
    pub homophones: HashMap<String, Vec<String>>,
    #[serde(default, rename = "semanticSynonyms")]
    pub semantic_synonyms: HashMap<String, Vec<String>>,
    #[serde(default, rename = "fuzzyMap")]
    pub fuzzy_map: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Metadata {
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    pub description: String,
    pub owner: String,
    #[serde(default, rename = "totalSize", skip_serializing_if = "Option::is_none")]
    pub total_size: Option<u64>,
    #[serde(
        default,
        rename = "entryCount",
        skip_serializing_if = "Option::is_none"
    )]
    pub entry_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeywordGroup {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: String,
    pub description: String,
    #[serde(rename = "baseFieldsSchema")]
    pub base_fields_schema: HashMap<String, FieldSchema>,
    #[serde(rename = "customFieldAllowed")]
    pub custom_field_allowed: CustomFieldConfig,
    pub entries: Vec<serde_json::Value>,
    #[serde(
        default,
        rename = "groupStats",
        skip_serializing_if = "Option::is_none"
    )]
    pub group_stats: Option<GroupStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupStats {
    #[serde(rename = "totalWeight")]
    pub total_weight: f64,
    #[serde(rename = "extensionCounts")]
    pub extension_counts: HashMap<String, usize>,
    #[serde(rename = "duplicateCount")]
    pub duplicate_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FieldSchema {
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default, rename = "itemType", skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CustomFieldConfig {
    pub enabled: bool,
    #[serde(rename = "maxOne")]
    pub max_one: bool,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<serde_json::Value>>,
    #[serde(
        default,
        rename = "customFieldSchema",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_field_schema: Option<FieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationConfig {
    pub rules: ValidationRules,
    #[serde(rename = "errorMessages")]
    pub error_messages: HashMap<String, String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        // NOTE-001: ใช้ขีดจำกัดเดียวกับ validator เพื่อให้ registry ที่ import แล้วตรวจสอบได้ทันที
        Self {
            rules: ValidationRules {
                alias_min_length: 1,
                alias_max_length: 255,
                description_min_length: 0,
                description_max_length: 10_000,
                custom_field_per_entry: 50,
                required_base_fields: vec!["id".to_string(), "aliases".to_string()],
            },
            error_messages: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationRules {
    #[serde(rename = "aliasMinLength")]
    pub alias_min_length: usize,
    #[serde(rename = "aliasMaxLength")]
    pub alias_max_length: usize,
    #[serde(rename = "descriptionMinLength")]
    pub description_min_length: usize,
    #[serde(rename = "descriptionMaxLength")]
    pub description_max_length: usize,
    #[serde(rename = "customFieldPerEntry")]
    pub custom_field_per_entry: usize,
    #[serde(rename = "requiredBaseFields")]
    pub required_base_fields: Vec<String>,
}

// ============= Search & Metadata Types =============

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SearchResult {
    pub id: String,
    #[serde(rename = "groupId")]
    pub group_id: String,
    pub aliases: Vec<String>,
    pub description: String,
    #[serde(rename = "matchType")]
    pub match_type: String,
    pub score: f64,
    #[serde(rename = "semanticContext", skip_serializing_if = "Option::is_none")]
    pub semantic_context: Option<SemanticMetadata>,
    #[serde(rename = "usageStats", skip_serializing_if = "Option::is_none")]
    pub usage_stats: Option<UsageMetadata>,
    #[serde(rename = "languageScore", skip_serializing_if = "Option::is_none")]
    pub language_score: Option<f64>,
    #[serde(default)]
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct UsageMetadata {
    #[serde(rename = "usageCount")]
    pub usage_count: u64,
    #[serde(rename = "lastUsedAt")]
    pub last_used_at: Option<String>,
    #[serde(rename = "learnedIntents")]
    pub learned_intents: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SemanticMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_th: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<u8>,
    #[serde(rename = "nuanceTags", skip_serializing_if = "Option::is_none")]
    pub nuance_tags: Option<Vec<String>>,
    #[serde(rename = "emotiveTone", skip_serializing_if = "Option::is_none")]
    pub emotive_tone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub count: usize,
}

#[cfg(test)]
mod schema_export_tests {
    use super::{export_keyword_registry_schema, KeywordRegistry, Metadata, ValidationConfig};
    use crate::{load_registry, migrate_registry, Validator};
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn public_schema_export_contains_current_registry_and_foundation_profile() {
        let schema = export_keyword_registry_schema();

        assert!(schema
            .schema
            .object
            .as_ref()
            .is_some_and(|object| object.properties.contains_key("version")));
        assert!(schema.definitions.contains_key("FoundationProfile"));
    }

    #[test]
    fn public_schema_is_meta_valid_and_validates_the_rust_registry_contract() {
        let schema = serde_json::to_value(export_keyword_registry_schema())
            .expect("schema export must serialize to JSON");
        assert!(jsonschema::draft7::meta::is_valid(&schema));

        let validator =
            jsonschema::validator_for(&schema).expect("schema must compile for consumers");
        let valid_registry = serde_json::to_value(KeywordRegistry {
            version: "1.2.0".to_string(),
            metadata: Metadata::default(),
            groups: Vec::new(),
            validation: ValidationConfig::default(),
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        })
        .expect("registry must serialize to JSON");
        assert!(validator.is_valid(&valid_registry));
        assert!(!validator.is_valid(&json!({
            "metadata": {},
            "groups": [],
            "validation": {}
        })));
        let mut invalid_policy = valid_registry.clone();
        invalid_policy["searchPolicy"] = json!({
            "fuzzyMinSimilarity": 1.01,
            "fuzzyCandidateLimit": 1024,
            "fuzzyNgramSize": 2,
            "maxFuzzyNgramPostings": 4096
        });
        assert!(!validator.is_valid(&invalid_policy));
    }

    #[test]
    fn public_schema_and_runtime_accept_the_same_migrated_registry_fixture() {
        let migrated = migrate_registry(KeywordRegistry {
            version: "1.1.0".to_string(),
            metadata: Metadata {
                last_updated: "2026-08-20T00:00:00Z".to_string(),
                ..Metadata::default()
            },
            groups: Vec::new(),
            validation: ValidationConfig::default(),
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        })
        .expect("legacy fixture must migrate");
        assert!(Validator::new(migrated.clone()).validate_registry().is_ok());

        let instance = serde_json::to_value(&migrated).expect("migrated registry must serialize");
        let schema = serde_json::to_value(export_keyword_registry_schema())
            .expect("schema export must serialize to JSON");
        let consumer =
            jsonschema::validator_for(&schema).expect("schema must compile for consumers");
        assert!(consumer.is_valid(&instance));

        let path = std::env::temp_dir().join(format!(
            "kept-schema-runtime-fixture-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos()
        ));
        fs::write(
            &path,
            serde_json::to_vec(&instance).expect("fixture must encode as JSON"),
        )
        .expect("fixture must be written");
        let loaded = load_registry(&path).expect("runtime loader must accept the same fixture");
        fs::remove_file(&path).expect("fixture must be removed");
        assert!(Validator::new(loaded).validate_registry().is_ok());
    }
}
