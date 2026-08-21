use crate::schema::{FoundationProfile, KeywordRegistry, NormalizationProfile};
use thiserror::Error;

pub const CURRENT_REGISTRY_VERSION: &str = "1.2.0";
pub const LEGACY_REGISTRY_VERSION: &str = "1.1.0";
const MIGRATION_EPOCH: &str = "1970-01-01T00:00:00Z";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MigrationError {
    #[error("Registry version '{0}' is not supported for migration")]
    UnsupportedVersion(String),
    #[error("Registry version '{0}' is newer than supported version '{CURRENT_REGISTRY_VERSION}'")]
    UnsupportedFutureVersion(String),
    #[error("Registry version '{0}' is missing its required foundation profile")]
    MissingFoundationProfile(String),
}

/// NOTE-001: migration ทำงานก่อน validation และใช้ timestamp เดิมเพื่อให้ rerun ให้ผลเหมือนเดิม
pub fn migrate_registry(mut registry: KeywordRegistry) -> Result<KeywordRegistry, MigrationError> {
    match registry.version.as_str() {
        LEGACY_REGISTRY_VERSION => {
            let timestamp = non_empty_timestamp(&registry.metadata.last_updated);
            registry.version = CURRENT_REGISTRY_VERSION.to_string();
            registry.foundation = Some(FoundationProfile {
                schema_version: CURRENT_REGISTRY_VERSION.to_string(),
                normalization: NormalizationProfile::default(),
                glossary_terms: Vec::new(),
                provenance_records: Vec::new(),
                regex_rules: Vec::new(),
                classification_policy: None,
                corpus_manifest: None,
                created_at: timestamp.clone(),
                updated_at: timestamp,
            });
            Ok(registry)
        }
        CURRENT_REGISTRY_VERSION => {
            if registry.foundation.is_none() {
                return Err(MigrationError::MissingFoundationProfile(
                    CURRENT_REGISTRY_VERSION.to_string(),
                ));
            }
            Ok(registry)
        }
        version if version_is_future(version) => Err(MigrationError::UnsupportedFutureVersion(
            version.to_string(),
        )),
        version => Err(MigrationError::UnsupportedVersion(version.to_string())),
    }
}

fn non_empty_timestamp(value: &str) -> String {
    if value.trim().is_empty() {
        MIGRATION_EPOCH.to_string()
    } else {
        value.to_string()
    }
}

fn version_is_future(version: &str) -> bool {
    version
        .split_once('.')
        .and_then(|(major, _)| major.parse::<u64>().ok())
        .is_some_and(|major| major > 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Metadata, ValidationConfig};

    pub(super) fn legacy_registry() -> KeywordRegistry {
        KeywordRegistry {
            version: LEGACY_REGISTRY_VERSION.to_string(),
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
        }
    }

    #[test]
    fn migration_preserves_source_timestamp() {
        let migrated = migrate_registry(legacy_registry()).expect("legacy migration must succeed");
        assert_eq!(migrated.version, CURRENT_REGISTRY_VERSION);
        assert_eq!(
            migrated
                .foundation
                .as_ref()
                .expect("migration must add foundation")
                .created_at,
            "2026-08-19T00:00:00Z"
        );
    }
}

#[cfg(test)]
mod validator_compatibility_tests {
    use super::{migrate_registry, tests::legacy_registry};
    use crate::Validator;

    #[test]
    fn validator_accepts_a_registry_after_legacy_migration() {
        let migrated = migrate_registry(legacy_registry()).expect("migration must succeed");
        assert!(Validator::new(migrated).validate_registry().is_ok());
    }
}
