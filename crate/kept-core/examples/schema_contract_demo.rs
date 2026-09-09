use kept_core::schema::{
    export_keyword_registry_schema, KeywordRegistry, Metadata, ValidationConfig,
};
use kept_core::{load_registry, migrate_registry, Validator};
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn migrated_fixture() -> Result<KeywordRegistry, Box<dyn Error + Send + Sync>> {
    Ok(migrate_registry(KeywordRegistry {
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
    })?)
}

fn run_demo() -> Result<Value, Box<dyn Error + Send + Sync>> {
    let schema = serde_json::to_value(export_keyword_registry_schema()?)?;
    let draft7_meta_valid = jsonschema::draft7::meta::is_valid(&schema);
    let consumer = jsonschema::validator_for(&schema)?;
    let fixture = migrated_fixture()?;
    let instance = serde_json::to_value(&fixture)?;
    let schema_consumer_accepts_registry = consumer.is_valid(&instance);
    let schema_consumer_rejects_missing_version = !consumer.is_valid(&json!({
        "metadata": {},
        "groups": [],
        "validation": {}
    }));

    let path = std::env::temp_dir().join(format!(
        "kept-schema-contract-demo-{}-{}.json",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::write(&path, serde_json::to_vec(&instance)?)?;
    let loaded = load_registry(&path)?;
    fs::remove_file(&path)?;
    let runtime_loader_accepts_registry =
        Validator::new(loaded.clone()).validate_registry().is_ok();

    Ok(json!({
        "demo": "kept-schema-contract",
        "artifact": "schema/keyword-registry.schema.json",
        "draft7MetaValid": draft7_meta_valid,
        "schemaConsumerAcceptsRegistry": schema_consumer_accepts_registry,
        "schemaConsumerRejectsMissingVersion": schema_consumer_rejects_missing_version,
        "runtimeLoaderAcceptsRegistry": runtime_loader_accepts_registry,
        "runtimeRegistryVersion": loaded.version,
        "artifactTitle": schema["title"],
        "requiredRootFields": schema["required"]
    }))
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("{}", serde_json::to_string_pretty(&run_demo()?)?);
    Ok(())
}

#[cfg(test)]
mod demo_tests {
    use super::run_demo;

    #[test]
    fn demo_reports_schema_consumer_and_runtime_contracts_as_true() {
        let report = run_demo().expect("schema contract demo must run");

        assert_eq!(report["draft7MetaValid"], true);
        assert_eq!(report["schemaConsumerAcceptsRegistry"], true);
        assert_eq!(report["schemaConsumerRejectsMissingVersion"], true);
        assert_eq!(report["runtimeLoaderAcceptsRegistry"], true);
    }
}
