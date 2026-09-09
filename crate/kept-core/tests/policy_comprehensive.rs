//! Comprehensive unit tests for kept-core::policy
//!
//! Covers: validate_naming_settings edge cases, merge behavior,
//! analyze_naming transforms, resolve_naming_rule edge cases,
//! UserConfig validation, and helper function contracts.

use std::path::Path;

use kept_core::{
    analyze_naming, resolve_naming_rule, NamingIssueKind, NamingSettings, ResolvedNamingRule,
    UserConfig,
};

// ─── validate_naming_settings edge cases ────────────────────────────────────

fn starter_config() -> UserConfig {
    UserConfig::starter()
}

#[test]
fn validate_rejects_unsupported_unicode_value() {
    let mut config = starter_config();
    config.defaults.naming.unicode = Some("nfkd".to_string());
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .to_string()
        .contains("unsupported unicode value nfkd"));
}

#[test]
fn validate_accepts_nfc_unicode() {
    let mut config = starter_config();
    config.defaults.naming.unicode = Some("nfc".to_string());
    assert!(config.validate().is_ok());
}

#[test]
fn validate_rejects_unsupported_case_value() {
    let mut config = starter_config();
    config.defaults.naming.case = Some("camel".to_string());
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .to_string()
        .contains("unsupported case value camel"));
}

#[test]
fn validate_accepts_lower_upper_preserve_case() {
    for case_val in &["lower", "upper", "preserve"] {
        let mut config = starter_config();
        config.defaults.naming.case = Some(case_val.to_string());
        assert!(
            config.validate().is_ok(),
            "case value '{case_val}' should be accepted"
        );
    }
}

#[test]
fn validate_rejects_unsupported_separator_value() {
    let mut config = starter_config();
    config.defaults.naming.separator = Some("dot".to_string());
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .to_string()
        .contains("unsupported separator value dot"));
}

#[test]
fn validate_accepts_kebab_snake_preserve_separator() {
    for sep in &["kebab", "snake", "preserve"] {
        let mut config = starter_config();
        config.defaults.naming.separator = Some(sep.to_string());
        assert!(
            config.validate().is_ok(),
            "separator value '{sep}' should be accepted"
        );
    }
}

#[test]
fn validate_rejects_similarity_threshold_out_of_range() {
    let mut config = starter_config();
    config.defaults.naming.similarity.name = Some(kept_core::SimilarityRule {
        threshold: 1.5,
        case_sensitive: false,
    });
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("similarity threshold"));
}

#[test]
fn validate_rejects_negative_similarity_threshold() {
    let mut config = starter_config();
    config.defaults.naming.similarity.name = Some(kept_core::SimilarityRule {
        threshold: -0.1,
        case_sensitive: false,
    });
    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_length_stem_min_exceeds_max() {
    let mut config = starter_config();
    config.defaults.naming.length = kept_core::LengthSettings {
        stem: Some(kept_core::RangeLimit {
            min: Some(200),
            max: Some(100),
        }),
    };
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("length.stem min cannot exceed max"));
}

#[test]
fn validate_rejects_words_min_exceeds_max() {
    let mut config = starter_config();
    config.defaults.naming.words = kept_core::WordSettings {
        min: Some(20),
        max: Some(5),
    };
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("words min cannot exceed max"));
}

#[test]
fn validate_rejects_invalid_stem_regex() {
    let mut config = starter_config();
    config.defaults.naming.stem_regex = Some("[invalid".to_string());
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("invalid stemRegex"));
}

#[test]
fn validate_accepts_valid_stem_regex() {
    let mut config = starter_config();
    config.defaults.naming.stem_regex = Some(r"^[a-z-]+$".to_string());
    assert!(config.validate().is_ok());
}

#[test]
fn validate_rejects_empty_extension() {
    let mut config = starter_config();
    config.defaults.naming.extensions = vec!["".to_string()];
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("extensions must be non-empty values"));
}

#[test]
fn validate_rejects_extension_with_leading_dot() {
    let mut config = starter_config();
    config.defaults.naming.extensions = vec![".rs".to_string()];
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("extensions must be non-empty values"));
}

#[test]
fn validate_rejects_extension_with_whitespace() {
    let mut config = starter_config();
    config.defaults.naming.extensions = vec!["r s".to_string()];
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("extensions must be non-empty values"));
}

// ─── UserConfig validation ─────────────────────────────────────────────────

#[test]
fn validate_rejects_wrong_version() {
    let mut config = starter_config();
    config.version = 999;
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("unsupported config version 999"));
}

#[test]
fn validate_rejects_scope_with_relative_path() {
    let mut config = starter_config();
    config.scopes.push(kept_core::NamingScope {
        path: "relative/path".into(),
        profile: "generic".into(),
        recursive: true,
        priority: 0,
        exceptions: vec![],
        overrides: kept_core::ScopeOverrides::default(),
    });
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("scope path must be absolute"));
}

#[test]
fn validate_rejects_scope_with_missing_profile() {
    let mut config = starter_config();
    config.scopes.push(kept_core::NamingScope {
        path: "/work".into(),
        profile: "nonexistent".into(),
        recursive: true,
        priority: 0,
        exceptions: vec![],
        overrides: kept_core::ScopeOverrides::default(),
    });
    let err = config.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("scope /work references missing profile nonexistent"));
}

#[test]
fn validate_rejects_scope_with_relative_exception() {
    let mut config = starter_config();
    config.scopes.push(kept_core::NamingScope {
        path: "/work".into(),
        profile: "generic".into(),
        recursive: true,
        priority: 0,
        exceptions: vec!["relative/exception".into()],
        overrides: kept_core::ScopeOverrides::default(),
    });
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("relative exception"));
}

#[test]
fn validate_rejects_profile_with_invalid_naming() {
    let mut config = starter_config();
    config.profiles.insert(
        "bad".to_string(),
        kept_core::NamingProfile {
            description: None,
            naming: NamingSettings {
                case: Some("invalid_case".to_string()),
                ..NamingSettings::default()
            },
        },
    );
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("profile bad"));
}

// ─── resolve_naming_rule edge cases ─────────────────────────────────────────

#[test]
fn resolve_returns_none_when_no_scopes_match() {
    let config = starter_config();
    let result = resolve_naming_rule(&config, Path::new("/unscoped/file.txt")).unwrap();
    assert!(result.is_none());
}

#[test]
fn resolve_rejects_relative_file_path() {
    let config = starter_config();
    let err = resolve_naming_rule(&config, Path::new("relative/file.txt")).unwrap_err();
    assert!(err.to_string().contains("file path must be absolute"));
}

#[test]
fn resolve_selects_deeper_scope_over_higher_priority() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  wide: { naming: {} }
  deep: { naming: { case: lower } }
scopes:
  - path: /project
    profile: wide
    priority: 100
  - path: /project/src
    profile: deep
    priority: 0
"#,
    )
    .unwrap();

    let rule = resolve_naming_rule(&config, Path::new("/project/src/main.rs"))
        .unwrap()
        .unwrap();
    // Deeper scope (/project/src) wins over higher priority (/project)
    assert_eq!(rule.profile_id, "deep");
    assert_eq!(rule.naming.case.as_deref(), Some("lower"));
}

#[test]
fn resolve_filters_by_extension_when_scope_has_extensions() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  rust: { naming: { extensions: [rs] } }
scopes:
  - path: /work
    profile: rust
"#,
    )
    .unwrap();

    // .rs file matches
    let rule = resolve_naming_rule(&config, Path::new("/work/main.rs"))
        .unwrap()
        .unwrap();
    assert_eq!(rule.profile_id, "rust");

    // .txt file doesn't match extension filter
    let result = resolve_naming_rule(&config, Path::new("/work/readme.txt")).unwrap();
    assert!(result.is_none());
}

#[test]
fn resolve_respects_non_recursive_scope() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  reports: { naming: { extensions: [pdf] } }
scopes:
  - path: /work/reports
    profile: reports
    recursive: false
"#,
    )
    .unwrap();

    // Direct child matches
    let rule = resolve_naming_rule(&config, Path::new("/work/reports/q1.pdf"))
        .unwrap()
        .unwrap();
    assert_eq!(rule.profile_id, "reports");

    // Nested file doesn't match non-recursive scope
    let result = resolve_naming_rule(&config, Path::new("/work/reports/old/q1.pdf")).unwrap();
    assert!(result.is_none());
}

#[test]
fn resolve_respects_exceptions() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  docs: { naming: { case: lower } }
scopes:
  - path: /work
    profile: docs
    exceptions:
      - /work/external
"#,
    )
    .unwrap();

    // Inside exception path
    let result = resolve_naming_rule(&config, Path::new("/work/external/file.txt")).unwrap();
    assert!(result.is_none());

    // Outside exception path
    let rule = resolve_naming_rule(&config, Path::new("/work/file.txt"))
        .unwrap()
        .unwrap();
    assert_eq!(rule.profile_id, "docs");
}

// ─── analyze_naming edge cases ──────────────────────────────────────────────

fn simple_rule(profile_id: &str, naming: NamingSettings) -> ResolvedNamingRule {
    ResolvedNamingRule {
        scope_path: "/work".into(),
        profile_id: profile_id.to_string(),
        naming,
    }
}

#[test]
fn analyze_lower_case_rule() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            case: Some("lower".to_string()),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/MyFile.txt"), &rule, &[]).unwrap();
    assert_eq!(finding.proposed_target, Some("myfile.txt".into()));
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Case));
}

#[test]
fn analyze_upper_case_rule() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            case: Some("upper".to_string()),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/myfile.txt"), &rule, &[]).unwrap();
    // Extension is always lowercased in output
    assert_eq!(finding.proposed_target, Some("MYFILE.txt".into()));
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Case));
}

#[test]
fn analyze_kebab_separator_rule() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            separator: Some("kebab".to_string()),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/my_file_name.txt"), &rule, &[]).unwrap();
    assert_eq!(finding.proposed_target, Some("my-file-name.txt".into()));
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Separator));
}

#[test]
fn analyze_snake_separator_rule() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            separator: Some("snake".to_string()),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/my-file-name.txt"), &rule, &[]).unwrap();
    assert_eq!(finding.proposed_target, Some("my_file_name.txt".into()));
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Separator));
}

#[test]
fn analyze_control_characters_detected() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            flag_control_characters: Some(true),
            ..NamingSettings::default()
        },
    );
    // Filename with control character (\x03)
    let finding = analyze_naming(Path::new("/work/file\x03name.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::ControlCharacter));
}

#[test]
fn analyze_whitespace_trim_detected() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            flag_trim_whitespace: Some(true),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/ spaced .txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::TrimWhitespace));
}

#[test]
fn analyze_stem_regex_violation() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            stem_regex: Some(r"^[a-z-]+$".to_string()),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/My File.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::StemRegex));
}

#[test]
fn analyze_stem_length_too_long() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            length: kept_core::LengthSettings {
                stem: Some(kept_core::RangeLimit {
                    min: None,
                    max: Some(5),
                }),
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/longfilename.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Length));
}

#[test]
fn analyze_stem_length_too_short() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            length: kept_core::LengthSettings {
                stem: Some(kept_core::RangeLimit {
                    min: Some(10),
                    max: None,
                }),
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/ab.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Length));
}

#[test]
fn analyze_word_count_violation() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            words: kept_core::WordSettings {
                min: Some(3),
                max: None,
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/two words.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::WordCount));
}

#[test]
fn analyze_numbers_not_allowed() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            numbers: kept_core::NumberSettings {
                allow: Some(false),
                max_digits_per_token: None,
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/file123.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Number));
}

#[test]
fn analyze_max_digits_per_token_exceeded() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            numbers: kept_core::NumberSettings {
                allow: Some(true),
                max_digits_per_token: Some(3),
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/file12345.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Number));
}

#[test]
fn analyze_portability_conflict_detected() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            flag_portability_conflicts: Some(true),
            ..NamingSettings::default()
        },
    );
    // Windows reserved name
    let finding = analyze_naming(Path::new("/work/con.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Portability));
}

#[test]
fn analyze_portability_conflict_special_chars() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            flag_portability_conflicts: Some(true),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/file<name>.txt"), &rule, &[]).unwrap();
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Portability));
}

#[test]
fn analyze_blocked_by_collision() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            case: Some("lower".to_string()),
            ..NamingSettings::default()
        },
    );
    let occupied = vec![PathBuf::from("/work/target.txt")];
    let finding = analyze_naming(Path::new("/work/TARGET.txt"), &rule, &occupied).unwrap();
    assert!(finding.blocked);
    assert!(finding
        .issues
        .iter()
        .any(|i| i.kind == NamingIssueKind::Collision));
}

#[test]
fn analyze_no_issues_when_already_compliant() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            case: Some("lower".to_string()),
            separator: Some("kebab".to_string()),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/already-compliant.txt"), &rule, &[]).unwrap();
    assert!(finding.issues.is_empty());
    assert!(finding.proposed_target.is_none());
    assert!(!finding.blocked);
}

#[test]
fn analyze_empty_extension_not_filtered() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            extensions: vec!["txt".to_string()],
            case: Some("lower".to_string()),
            ..NamingSettings::default()
        },
    );
    // File without extension — extensions list is non-empty so it's filtered out
    let finding = analyze_naming(Path::new("/work/Makefile"), &rule, &[]).unwrap();
    // No extension, extensions list is ["txt"], so no issues and no proposed target
    assert!(finding.issues.is_empty());
    assert!(finding.proposed_target.is_none());
}

use std::path::PathBuf;

#[test]
fn analyze_extension_case_insensitive_match() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            extensions: vec!["TXT".to_string()],
            case: Some("lower".to_string()),
            ..NamingSettings::default()
        },
    );
    // .txt should match extension "TXT" case-insensitively
    let finding = analyze_naming(Path::new("/work/file.TXT"), &rule, &[]).unwrap();
    // It should apply the case transform
    assert_eq!(finding.proposed_target, Some("file.txt".into()));
}

#[test]
fn analyze_replacements_applied() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            replacements: vec![kept_core::TokenReplacement {
                from: "old".to_string(),
                to: "new".to_string(),
                case_sensitive: true,
            }],
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/old-name.txt"), &rule, &[]).unwrap();
    assert_eq!(finding.proposed_target, Some("new-name.txt".into()));
}

#[test]
fn analyze_required_prefix_added() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            prefix: kept_core::PrefixSettings {
                required: Some("proj".to_string()),
                allow: vec![],
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/myfile.txt"), &rule, &[]).unwrap();
    assert_eq!(finding.proposed_target, Some("proj myfile.txt".into()));
}

#[test]
fn analyze_required_prefix_not_doubled() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            prefix: kept_core::PrefixSettings {
                required: Some("proj".to_string()),
                allow: vec![],
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/proj myfile.txt"), &rule, &[]).unwrap();
    // Already has prefix, no change needed
    assert!(finding.proposed_target.is_none());
}

#[test]
fn analyze_combined_lower_kebab_and_prefix() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            case: Some("lower".to_string()),
            separator: Some("kebab".to_string()),
            prefix: kept_core::PrefixSettings {
                required: Some("app".to_string()),
                allow: vec![],
            },
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/My_File_Name.txt"), &rule, &[]).unwrap();
    // Prefix is added first, then kebab separator normalizes spaces to hyphens
    assert_eq!(finding.proposed_target, Some("app-my-file-name.txt".into()));
}

#[test]
fn analyze_preserve_case_and_separator_no_issues() {
    let rule = simple_rule(
        "test",
        NamingSettings {
            case: Some("preserve".to_string()),
            separator: Some("preserve".to_string()),
            ..NamingSettings::default()
        },
    );
    let finding = analyze_naming(Path::new("/work/Any_Name.txt"), &rule, &[]).unwrap();
    // Extension is always lowercased, so no issue with lowercase .txt
    assert!(finding.issues.is_empty());
    assert!(finding.proposed_target.is_none());
}
