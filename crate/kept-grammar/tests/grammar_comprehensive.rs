//! Comprehensive unit tests for kept-grammar
//!
//! Covers: validate_naming_settings edge cases, config I/O roundtrip,
//! starter config YAML parsing, create_user_config_if_missing behavior.

use kept_grammar::{
    create_user_config_if_missing, load_user_config, save_user_config, starter_config_yaml,
    validate_naming_settings, LengthSettings, NamingSettings, RangeLimit, SimilarityRule,
    SimilaritySettings, UserConfig, WordSettings,
};
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kept-grammar-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir must be created");
    dir
}

// ─── validate_naming_settings ───────────────────────────────────────────────

#[test]
fn validate_accepts_default_settings() {
    let settings = NamingSettings::default();
    assert!(validate_naming_settings(&settings).is_ok());
}

#[test]
fn validate_accepts_nfc_unicode() {
    let settings = NamingSettings {
        unicode: Some("nfc".to_string()),
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_ok());
}

#[test]
fn validate_rejects_unsupported_unicode() {
    let settings = NamingSettings {
        unicode: Some("nfkd".to_string()),
        ..NamingSettings::default()
    };
    let err = validate_naming_settings(&settings).unwrap_err();
    assert!(err.to_string().contains("unsupported unicode value nfkd"));
}

#[test]
fn validate_accepts_all_valid_case_values() {
    for case in &["lower", "upper", "preserve"] {
        let settings = NamingSettings {
            case: Some(case.to_string()),
            ..NamingSettings::default()
        };
        assert!(
            validate_naming_settings(&settings).is_ok(),
            "case '{case}' should be valid"
        );
    }
}

#[test]
fn validate_rejects_invalid_case() {
    let settings = NamingSettings {
        case: Some("camelCase".to_string()),
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_err());
}

#[test]
fn validate_accepts_all_valid_separator_values() {
    for sep in &["kebab", "snake", "preserve"] {
        let settings = NamingSettings {
            separator: Some(sep.to_string()),
            ..NamingSettings::default()
        };
        assert!(
            validate_naming_settings(&settings).is_ok(),
            "separator '{sep}' should be valid"
        );
    }
}

#[test]
fn validate_rejects_invalid_separator() {
    let settings = NamingSettings {
        separator: Some("dot".to_string()),
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_err());
}

#[test]
fn validate_rejects_similarity_threshold_above_one() {
    let settings = NamingSettings {
        similarity: SimilaritySettings {
            name: Some(SimilarityRule {
                threshold: 1.5,
                case_sensitive: false,
            }),
        },
        ..NamingSettings::default()
    };
    let err = validate_naming_settings(&settings).unwrap_err();
    assert!(err.to_string().contains("similarity threshold"));
}

#[test]
fn validate_rejects_negative_similarity_threshold() {
    let settings = NamingSettings {
        similarity: SimilaritySettings {
            name: Some(SimilarityRule {
                threshold: -0.1,
                case_sensitive: false,
            }),
        },
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_err());
}

#[test]
fn validate_accepts_boundary_similarity_thresholds() {
    for threshold in &[0.0, 0.5, 1.0] {
        let settings = NamingSettings {
            similarity: SimilaritySettings {
                name: Some(SimilarityRule {
                    threshold: *threshold,
                    case_sensitive: false,
                }),
            },
            ..NamingSettings::default()
        };
        assert!(
            validate_naming_settings(&settings).is_ok(),
            "threshold {threshold} should be valid"
        );
    }
}

#[test]
fn validate_rejects_length_stem_min_greater_than_max() {
    let settings = NamingSettings {
        length: LengthSettings {
            stem: Some(RangeLimit {
                min: Some(200),
                max: Some(100),
            }),
        },
        ..NamingSettings::default()
    };
    let err = validate_naming_settings(&settings).unwrap_err();
    assert!(err
        .to_string()
        .contains("length.stem min cannot exceed max"));
}

#[test]
fn validate_rejects_words_min_greater_than_max() {
    let settings = NamingSettings {
        words: WordSettings {
            min: Some(20),
            max: Some(5),
        },
        ..NamingSettings::default()
    };
    let err = validate_naming_settings(&settings).unwrap_err();
    assert!(err.to_string().contains("words min cannot exceed max"));
}

#[test]
fn validate_rejects_invalid_stem_regex() {
    let settings = NamingSettings {
        stem_regex: Some("[unclosed".to_string()),
        ..NamingSettings::default()
    };
    let err = validate_naming_settings(&settings).unwrap_err();
    assert!(err.to_string().contains("invalid stemRegex"));
}

#[test]
fn validate_accepts_valid_stem_regex() {
    let settings = NamingSettings {
        stem_regex: Some(r"^[a-z]+-[a-z]+$".to_string()),
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_ok());
}

#[test]
fn validate_rejects_empty_extension() {
    let settings = NamingSettings {
        extensions: vec!["".to_string()],
        ..NamingSettings::default()
    };
    let err = validate_naming_settings(&settings).unwrap_err();
    assert!(err
        .to_string()
        .contains("extensions must be non-empty values"));
}

#[test]
fn validate_rejects_extension_with_dot_prefix() {
    let settings = NamingSettings {
        extensions: vec![".rs".to_string()],
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_err());
}

#[test]
fn validate_rejects_extension_with_whitespace() {
    let settings = NamingSettings {
        extensions: vec!["r s".to_string()],
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_err());
}

#[test]
fn validate_accepts_valid_extensions() {
    let settings = NamingSettings {
        extensions: vec!["rs".to_string(), "ts".to_string(), "py".to_string()],
        ..NamingSettings::default()
    };
    assert!(validate_naming_settings(&settings).is_ok());
}

// ─── starter_config_yaml ────────────────────────────────────────────────────

#[test]
fn starter_config_is_valid_yaml() {
    let yaml = starter_config_yaml();
    let config: UserConfig = serde_yaml::from_str(yaml).expect("starter config must be valid YAML");
    assert_eq!(config.version, 1);
}

#[test]
fn starter_config_has_all_expected_profiles() {
    let yaml = starter_config_yaml();
    let config: UserConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.profiles.contains_key("generic"));
    assert!(config.profiles.contains_key("reports"));
    assert!(config.profiles.contains_key("source-code"));
    assert!(config.profiles.contains_key("images-media"));
    assert!(config.profiles.contains_key("archives"));
}

#[test]
fn starter_config_has_default_naming_settings() {
    let yaml = starter_config_yaml();
    let config: UserConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.defaults.naming.unicode.as_deref(), Some("nfc"));
    assert_eq!(config.defaults.naming.case.as_deref(), Some("preserve"));
    assert_eq!(
        config.defaults.naming.separator.as_deref(),
        Some("preserve")
    );
    assert_eq!(config.defaults.naming.flag_control_characters, Some(true));
    assert_eq!(config.defaults.naming.flag_trim_whitespace, Some(true));
}

// ─── create_user_config_if_missing ──────────────────────────────────────────

#[test]
fn create_config_creates_file_when_missing() {
    let dir = temp_dir("create-missing");
    let config_path = dir.join("config.yaml");

    let created = create_user_config_if_missing(&config_path).unwrap();
    assert!(created);
    assert!(config_path.exists());

    // Cleanup
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn create_config_does_not_overwrite_existing() {
    let dir = temp_dir("create-existing");
    let config_path = dir.join("config.yaml");

    // Write a custom config
    fs::write(&config_path, "version: 1\ndefaults: {}\nprofiles: {}\n").unwrap();

    let created = create_user_config_if_missing(&config_path).unwrap();
    assert!(!created);

    // Content should be unchanged
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("version: 1"));
    assert!(!content.contains("starter"));

    // Cleanup
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn create_config_creates_parent_directories() {
    let dir = temp_dir("create-parents");
    let config_path = dir.join("nested").join("deep").join("config.yaml");

    let created = create_user_config_if_missing(&config_path).unwrap();
    assert!(created);
    assert!(config_path.exists());

    // Cleanup
    let _ = fs::remove_dir_all(dir);
}

// ─── load/save roundtrip ────────────────────────────────────────────────────

#[test]
fn save_and_load_roundtrip() {
    let dir = temp_dir("roundtrip");
    let config_path = dir.join("config.yaml");

    let mut config = minimal_config();
    config.defaults.naming.case = Some("lower".to_string());

    save_user_config(&config_path, &config).unwrap();
    let loaded = load_user_config(&config_path).unwrap();

    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.defaults.naming.case.as_deref(), Some("lower"));

    // Cleanup
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_rejects_invalid_yaml() {
    let dir = temp_dir("invalid-yaml");
    let config_path = dir.join("config.yaml");

    fs::write(&config_path, "not: valid: yaml: [[[[").unwrap();
    let result = load_user_config(&config_path);
    assert!(result.is_err());

    // Cleanup
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn save_produces_loadable_yaml() {
    let dir = temp_dir("save-loadable");
    let config_path = dir.join("config.yaml");

    let config = minimal_config();
    save_user_config(&config_path, &config).unwrap();

    // The saved YAML should be parseable
    let content = fs::read_to_string(&config_path).unwrap();
    let parsed: Result<UserConfig, _> = serde_yaml::from_str(&content);
    assert!(parsed.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(dir);
}

// ─── UserConfig type defaults ───────────────────────────────────────────────

fn minimal_config() -> UserConfig {
    serde_yaml::from_str("version: 1\ndefaults: {}\nprofiles: {}\n").unwrap()
}

#[test]
fn user_config_has_version_one() {
    let config = minimal_config();
    assert_eq!(config.version, 1);
    assert!(config.profiles.is_empty());
    assert!(config.scopes.is_empty());
}

#[test]
fn naming_settings_default_all_none() {
    let settings = NamingSettings::default();
    assert!(settings.unicode.is_none());
    assert!(settings.case.is_none());
    assert!(settings.separator.is_none());
    assert!(settings.extensions.is_empty());
    assert!(settings.stem_regex.is_none());
    assert!(settings.replacements.is_empty());
    assert!(settings.reposition.is_empty());
}
