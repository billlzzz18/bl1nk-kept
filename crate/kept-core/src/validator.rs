use crate::error::ValidationError;
use crate::foundation::validate_regex_rule;
use crate::migration::CURRENT_REGISTRY_VERSION;
use crate::schema::KeywordRegistry;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub struct Validator {
    registry: KeywordRegistry,
}

impl Validator {
    pub fn new(registry: KeywordRegistry) -> Self {
        Self { registry }
    }

    /// NOTE-001: ตรวจ alias ซ้ำข้ามกลุ่ม โดยข้าม entry ที่กำลังแก้ไขได้
    pub fn check_duplicate_aliases(
        &self,
        group_id: &str,
        editing_entry_id: Option<&str>,
        new_aliases: &[String],
    ) -> Vec<ValidationError> {
        let mut aliases: HashMap<String, (String, String)> = HashMap::new();
        for group in &self.registry.groups {
            for entry in &group.entries {
                let Some(entry_id) = entry.get("id").and_then(Value::as_str) else {
                    continue;
                };
                if editing_entry_id == Some(entry_id) && group.group_id == group_id {
                    continue;
                }
                if let Some(values) = entry.get("aliases").and_then(Value::as_array) {
                    for alias in values.iter().filter_map(Value::as_str) {
                        aliases.insert(
                            normalize(alias),
                            (group.group_id.clone(), entry_id.to_string()),
                        );
                    }
                }
            }
        }
        new_aliases
            .iter()
            .filter_map(|alias| {
                aliases
                    .get(&normalize(alias))
                    .map(|(existing_group, existing_entry)| ValidationError {
                        code: "DUPLICATE_ALIAS".into(),
                        message: format!(
                            "Alias '{}' already exists in group '{}' entry '{}'",
                            alias, existing_group, existing_entry
                        ),
                        field: Some("aliases".into()),
                    })
            })
            .collect()
    }

    pub fn validate_entry(
        &self,
        group_id: &str,
        entry: &Value,
    ) -> Result<(), Vec<ValidationError>> {
        let Some(group) = self
            .registry
            .groups
            .iter()
            .find(|group| group.group_id == group_id)
        else {
            return Err(vec![ValidationError {
                code: "GROUP_NOT_FOUND".into(),
                message: format!("Group '{}' not found", group_id),
                field: None,
            }]);
        };
        let mut errors = Vec::new();
        for (field_name, field_schema) in &group.base_fields_schema {
            if field_schema.required.unwrap_or(false) && entry.get(field_name).is_none() {
                errors.push(ValidationError {
                    code: "MISSING_REQUIRED_FIELD".into(),
                    message: format!("Missing required field '{}'", field_name),
                    field: Some(field_name.clone()),
                });
                continue;
            }
            let Some(value) = entry.get(field_name) else {
                continue;
            };
            validate_field(field_name, value, field_schema, &self.registry, &mut errors);
        }

        // NOTE-001: ส่วนที่ไม่ใช่ base field ถือเป็น custom field และตรวจจำนวน/ชนิดตาม group policy
        if group.custom_field_allowed.enabled {
            let custom_fields: Vec<(&String, &Value)> = entry
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .filter(|(name, _)| !group.base_fields_schema.contains_key(*name))
                        .collect()
                })
                .unwrap_or_default();
            let limit = if group.custom_field_allowed.max_one {
                1
            } else {
                self.registry.validation.rules.custom_field_per_entry
            };
            if custom_fields.len() > limit {
                errors.push(ValidationError {
                    code: "TOO_MANY_CUSTOM_FIELDS".into(),
                    message: format!(
                        "Only {} custom field(s) allowed for group '{}'",
                        limit, group_id
                    ),
                    field: None,
                });
            }
            if let Some(custom_schema) = &group.custom_field_allowed.custom_field_schema {
                for (name, value) in custom_fields {
                    validate_field(name, value, custom_schema, &self.registry, &mut errors);
                }
            }
        } else if let Some(object) = entry.as_object() {
            let unknown = object
                .keys()
                .filter(|name| !group.base_fields_schema.contains_key(*name))
                .cloned()
                .collect::<Vec<_>>();
            if !unknown.is_empty() {
                errors.push(ValidationError {
                    code: "CUSTOM_FIELDS_DISABLED".into(),
                    message: format!(
                        "Custom fields are disabled; unknown fields: {}",
                        unknown.join(", ")
                    ),
                    field: None,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn validate_registry(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        if self.registry.version != CURRENT_REGISTRY_VERSION {
            errors.push(ValidationError {
                code: "INCOMPATIBLE_VERSION".into(),
                message: format!(
                    "Registry version '{}' is not supported. Supported version is '{}'",
                    self.registry.version, CURRENT_REGISTRY_VERSION
                ),
                field: None,
            });
            return Err(errors);
        }
        let Some(foundation) = &self.registry.foundation else {
            errors.push(ValidationError {
                code: "MISSING_FOUNDATION_PROFILE".into(),
                message: "Current registry schema requires a foundation profile".into(),
                field: Some("foundation".into()),
            });
            return Err(errors);
        };
        if !valid_normalization_profile(&foundation.normalization) {
            errors.push(ValidationError {
                code: "INVALID_NORMALIZATION_PROFILE".into(),
                message: "Normalization profile must use utf-8, nfc, collapse, lowercase_latin, and preserve_non_latin".into(),
                field: Some("foundation.normalization".into()),
            });
            return Err(errors);
        }
        if self
            .registry
            .search_policy
            .as_ref()
            .is_some_and(|policy| !valid_search_policy(policy))
        {
            errors.push(ValidationError {
                code: "INVALID_SEARCH_POLICY".into(),
                message: "Search policy requires fuzzyMinSimilarity from 0.0 to 1.0 and fuzzyNgramSize above zero".into(),
                field: Some("searchPolicy".into()),
            });
            return Err(errors);
        }
        for rule in &foundation.regex_rules {
            if let Err(error) = validate_regex_rule(rule) {
                errors.push(ValidationError {
                    code: "INVALID_REGEX_RULE".into(),
                    message: error.to_string(),
                    field: Some(format!("foundation.regexRules.{}", rule.id)),
                });
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut all_ids = HashSet::new();
        for group in &self.registry.groups {
            let mut group_ids = HashSet::new();
            for entry in &group.entries {
                if let Some(id) = entry.get("id").and_then(Value::as_str) {
                    if !group_ids.insert(id.to_string()) {
                        errors.push(ValidationError {
                            code: "DUPLICATE_ID".into(),
                            message: format!(
                                "ID '{}' is duplicated within group namespace '{}'",
                                id, group.group_id
                            ),
                            field: Some("id".into()),
                        });
                    }
                    all_ids.insert(id.to_string());
                }
            }
        }
        for group in &self.registry.groups {
            for entry in &group.entries {
                if let Err(mut entry_errors) = self.validate_entry(&group.group_id, entry) {
                    errors.append(&mut entry_errors);
                }
                if let Some(related) = entry.get("relatedIds").and_then(Value::as_array) {
                    for value in related.iter().filter_map(Value::as_str) {
                        if !all_ids.contains(value) {
                            errors.push(ValidationError {
                                code: "BROKEN_RELATIONSHIP".into(),
                                message: format!("Referenced ID '{}' does not exist", value),
                                field: Some("relatedIds".into()),
                            });
                        }
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn registry(&self) -> &KeywordRegistry {
        &self.registry
    }
}

fn valid_normalization_profile(profile: &crate::schema::NormalizationProfile) -> bool {
    profile.encoding == "utf-8"
        && profile.unicode_form == "nfc"
        && profile.whitespace_policy == "collapse"
        && profile.case_policy == "lowercase_latin"
        && profile.script_policy == "preserve_non_latin"
}

fn valid_search_policy(policy: &crate::schema::SearchPolicy) -> bool {
    policy.fuzzy_min_similarity.is_finite()
        && (0.0..=1.0).contains(&policy.fuzzy_min_similarity)
        && policy.fuzzy_ngram_size > 0
}

fn validate_field(
    name: &str,
    value: &Value,
    schema: &crate::schema::FieldSchema,
    registry: &KeywordRegistry,
    errors: &mut Vec<ValidationError>,
) {
    let valid_type = match schema.field_type.as_str() {
        "string" | "enum" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        _ => true,
    };
    if !valid_type {
        errors.push(ValidationError {
            code: "INVALID_TYPE".into(),
            message: format!(
                "Field '{}' has invalid type for schema '{}': {:?}",
                name, schema.field_type, value
            ),
            field: Some(name.into()),
        });
        return;
    }
    if let Some(text) = value.as_str() {
        if let Some(pattern) = &schema.pattern {
            match Regex::new(pattern) {
                Ok(regex) if !regex.is_match(text) => errors.push(ValidationError {
                    code: "INVALID_PATTERN".into(),
                    message: format!("Field '{}' does not match pattern '{}'", name, pattern),
                    field: Some(name.into()),
                }),
                Err(_) => errors.push(ValidationError {
                    code: "INVALID_SCHEMA_PATTERN".into(),
                    message: format!("Invalid regex pattern for field '{}'", name),
                    field: Some(name.into()),
                }),
                _ => {}
            }
        }
        if let Some(values) = &schema.values {
            if !values.iter().any(|allowed| allowed == text) {
                errors.push(ValidationError {
                    code: "INVALID_ENUM".into(),
                    message: format!("Field '{}' must be one of: {}", name, values.join(", ")),
                    field: Some(name.into()),
                });
            }
        }
        if let Some(max) = schema.max_length.or_else(|| {
            (name == "description").then_some(registry.validation.rules.description_max_length)
        }) {
            if text.chars().count() > max {
                errors.push(ValidationError {
                    code: "DESCRIPTION_TOO_LONG".into(),
                    message: format!("Field '{}' exceeds {} characters", name, max),
                    field: Some(name.into()),
                });
            }
        }
        if name == "description"
            && text.chars().count() < registry.validation.rules.description_min_length
        {
            errors.push(ValidationError {
                code: "DESCRIPTION_TOO_SHORT".into(),
                message: format!(
                    "Field '{}' is shorter than {} characters",
                    name, registry.validation.rules.description_min_length
                ),
                field: Some(name.into()),
            });
        }
    }
    if let Some(array) = value.as_array() {
        if let Some(item_type) = &schema.item_type {
            for (index, item) in array.iter().enumerate() {
                let valid = match item_type.as_str() {
                    "string" => item.is_string(),
                    "number" => item.is_number(),
                    "boolean" => item.is_boolean(),
                    _ => true,
                };
                if !valid {
                    errors.push(ValidationError {
                        code: "INVALID_TYPE".into(),
                        message: format!("Item {} in field '{}' has invalid type", index, name),
                        field: Some(name.into()),
                    });
                }
            }
        }
        if name == "aliases" {
            for alias in array.iter().filter_map(Value::as_str) {
                let length = alias.chars().count();
                if length < registry.validation.rules.alias_min_length {
                    errors.push(ValidationError {
                        code: "ALIAS_TOO_SHORT".into(),
                        message: format!("Alias '{}' is too short", alias),
                        field: Some(name.into()),
                    });
                }
                if length > registry.validation.rules.alias_max_length {
                    errors.push(ValidationError {
                        code: "ALIAS_TOO_LONG".into(),
                        message: format!("Alias '{}' is too long", alias),
                        field: Some(name.into()),
                    });
                }
            }
        }
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        CustomFieldConfig, FieldSchema, KeywordGroup, Metadata, ValidationConfig, ValidationRules,
    };
    use serde_json::json;
    use std::collections::HashMap;

    fn registry() -> KeywordRegistry {
        let mut base = HashMap::new();
        base.insert(
            "id".into(),
            FieldSchema {
                field_type: "string".into(),
                item_type: None,
                pattern: None,
                values: None,
                required: Some(true),
                max_length: None,
                description: String::new(),
            },
        );
        base.insert(
            "aliases".into(),
            FieldSchema {
                field_type: "array".into(),
                item_type: Some("string".into()),
                pattern: None,
                values: None,
                required: Some(true),
                max_length: None,
                description: String::new(),
            },
        );
        base.insert(
            "description".into(),
            FieldSchema {
                field_type: "string".into(),
                item_type: None,
                pattern: None,
                values: None,
                required: Some(true),
                max_length: Some(30),
                description: String::new(),
            },
        );
        KeywordRegistry {
            version: "1.1.0".into(),
            metadata: Metadata {
                last_updated: "2026-01-01".into(),
                description: "test".into(),
                owner: "test".into(),
                total_size: None,
                entry_count: None,
                stats: None,
            },
            groups: vec![KeywordGroup {
                group_id: "g".into(),
                group_name: "G".into(),
                description: "test".into(),
                base_fields_schema: base,
                custom_field_allowed: CustomFieldConfig {
                    enabled: false,
                    max_one: false,
                    description: String::new(),
                    examples: None,
                    custom_field_schema: None,
                },
                entries: Vec::new(),
                group_stats: None,
            }],
            validation: ValidationConfig {
                rules: ValidationRules {
                    alias_min_length: 2,
                    alias_max_length: 30,
                    description_min_length: 3,
                    description_max_length: 30,
                    custom_field_per_entry: 1,
                    required_base_fields: Vec::new(),
                },
                error_messages: HashMap::new(),
            },
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        }
    }

    #[test]
    fn validates_legacy_valid_entry() {
        assert!(Validator::new(registry())
            .validate_entry(
                "g",
                &json!({"id":"x","aliases":["xx"],"description":"valid"})
            )
            .is_ok());
    }

    #[test]
    fn detects_short_description() {
        let result = Validator::new(registry())
            .validate_entry("g", &json!({"id":"x","aliases":["xx"],"description":"x"}));
        assert!(result
            .unwrap_err()
            .iter()
            .any(|error| error.code == "DESCRIPTION_TOO_SHORT"));
    }
}

#[cfg(test)]
mod foundation_profile_tests {
    use super::*;
    use crate::migrate_registry;
    use crate::schema::{KeywordRegistry, Metadata, SearchPolicy, ValidationConfig};

    #[test]
    fn validator_rejects_search_policy_outside_similarity_range() {
        let legacy = KeywordRegistry {
            version: "1.1.0".to_string(),
            metadata: Metadata {
                last_updated: "2026-08-19T00:00:00Z".to_string(),
                ..Metadata::default()
            },
            groups: Vec::new(),
            validation: ValidationConfig::default(),
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        };
        let mut registry = migrate_registry(legacy).expect("legacy registry must migrate");
        registry.search_policy = Some(SearchPolicy {
            fuzzy_min_similarity: 1.01,
            ..SearchPolicy::default()
        });

        let errors = Validator::new(registry)
            .validate_registry()
            .expect_err("out-of-range user search policy must be rejected");

        assert!(errors
            .iter()
            .any(|error| error.code == "INVALID_SEARCH_POLICY"));
    }

    #[test]
    fn validator_rejects_an_invalid_normalization_profile() {
        let legacy = KeywordRegistry {
            version: "1.1.0".to_string(),
            metadata: Metadata {
                last_updated: "2026-08-19T00:00:00Z".to_string(),
                ..Metadata::default()
            },
            groups: Vec::new(),
            validation: ValidationConfig::default(),
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        };
        let mut registry = migrate_registry(legacy).expect("legacy registry must migrate");
        registry
            .foundation
            .as_mut()
            .expect("migration must create foundation")
            .normalization
            .encoding = "latin-1".to_string();

        let errors = Validator::new(registry)
            .validate_registry()
            .expect_err("invalid normalization profile must be rejected");

        assert!(errors
            .iter()
            .any(|error| error.code == "INVALID_NORMALIZATION_PROFILE"));
    }
}

#[cfg(test)]
mod foundation_regex_catalog_tests {
    use super::*;
    use crate::foundation::{RegexRule, RegexScope, RegexTestVector, RuleSeverity};
    use crate::migrate_registry;
    use crate::schema::{KeywordRegistry, Metadata, ValidationConfig};

    #[test]
    fn validator_rejects_an_invalid_regex_rule_in_foundation_profile() {
        let legacy = KeywordRegistry {
            version: "1.1.0".to_string(),
            metadata: Metadata {
                last_updated: "2026-08-19T00:00:00Z".to_string(),
                ..Metadata::default()
            },
            groups: Vec::new(),
            validation: ValidationConfig::default(),
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        };
        let mut registry = migrate_registry(legacy).expect("legacy registry must migrate");
        registry
            .foundation
            .as_mut()
            .expect("migration must create foundation")
            .regex_rules
            .push(RegexRule {
                id: "bad-rule".to_string(),
                scope: RegexScope::Alias,
                pattern: "(".to_string(),
                severity: RuleSeverity::Error,
                description: "invalid fixture".to_string(),
                test_vectors: vec![
                    RegexTestVector {
                        value: "ok".to_string(),
                        accepted: true,
                    },
                    RegexTestVector {
                        value: "bad".to_string(),
                        accepted: false,
                    },
                ],
            });

        let errors = Validator::new(registry)
            .validate_registry()
            .expect_err("invalid regex catalog entry must be rejected");

        assert!(errors
            .iter()
            .any(|error| error.code == "INVALID_REGEX_RULE"));
    }
}
