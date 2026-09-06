use std::path::Path;

use kept_core::{analyze_naming, resolve_naming_rule, NamingIssueKind, UserConfig};

#[test]
fn starter_config_is_broad_user_owned_and_has_no_guessed_scope() {
    let config = UserConfig::starter();

    assert_eq!(config.defaults.naming.flag_control_characters, Some(true));
    assert!(config.profiles.contains_key("generic"));
    assert!(config.profiles.contains_key("reports"));
    assert!(config.profiles.contains_key("source-code"));
    assert!(config.profiles.contains_key("images-media"));
    assert!(config.profiles.contains_key("archives"));
    assert!(config.scopes.is_empty());
    assert_eq!(
        resolve_naming_rule(&config, Path::new("/work/report.pdf")).unwrap(),
        None
    );
}

#[test]
fn deepest_absolute_scope_then_priority_selects_one_profile() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  generic:
    naming: {}
  reports:
    naming:
      case: lower
      separator: kebab
      extensions: [pdf]
scopes:
  - path: /work
    profile: generic
    recursive: true
    priority: 100
  - path: /work/reports
    profile: reports
    recursive: true
    priority: 1
"#,
    )
    .expect("user config must parse");

    let resolved = resolve_naming_rule(&config, Path::new("/work/reports/Q1_Report FINAL.pdf"))
        .expect("scope resolution must succeed")
        .expect("an absolute user scope must match");

    assert_eq!(resolved.scope_path, Path::new("/work/reports"));
    assert_eq!(resolved.profile_id, "reports");
    assert_eq!(resolved.naming.case.as_deref(), Some("lower"));
}

#[test]
fn naming_analysis_reports_rule_violations_and_deterministic_target() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  reports:
    naming:
      case: lower
      separator: kebab
      extensions: [pdf]
scopes:
  - path: /work/reports
    profile: reports
    recursive: true
    priority: 100
"#,
    )
    .expect("user config must parse");
    let path = Path::new("/work/reports/Q1_Report FINAL.pdf");
    let rule = resolve_naming_rule(&config, path)
        .expect("scope resolution must succeed")
        .expect("scope must match");

    let finding = analyze_naming(path, &rule, &[]).expect("analysis must succeed");

    assert!(finding
        .issues
        .iter()
        .any(|issue| issue.kind == NamingIssueKind::Case));
    assert!(finding
        .issues
        .iter()
        .any(|issue| issue.kind == NamingIssueKind::Separator));
    assert_eq!(finding.proposed_target, Some("q1-report-final.pdf".into()));
    assert!(!finding.blocked);
}

#[test]
fn equal_scope_and_priority_is_rejected_instead_of_guessing_a_rule() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  first: { naming: {} }
  second: { naming: {} }
scopes:
  - path: /work/reports
    profile: first
    recursive: true
    priority: 100
  - path: /work/reports
    profile: second
    recursive: true
    priority: 100
"#,
    )
    .expect("config syntax must parse");

    let error = resolve_naming_rule(&config, Path::new("/work/reports/file.pdf"))
        .expect_err("ambiguous scopes must be a configuration error");
    assert!(error.to_string().contains("ambiguous"));
}

#[test]
fn index_analysis_uses_only_real_scopes_and_blocks_existing_target_collisions() {
    use kept_core::{analyze_index_naming, FileRecord, ScanIndex};

    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  reports:
    naming:
      case: lower
      separator: kebab
      extensions: [pdf]
scopes:
  - path: /work/reports
    profile: reports
    recursive: true
    priority: 100
"#,
    )
    .expect("user config must parse");
    let index = ScanIndex {
        root: "/work".into(),
        scanned_at_unix: 0,
        total_size: 3,
        files: vec![
            FileRecord {
                path: "reports/Q1_Report.pdf".into(),
                name: "Q1_Report.pdf".into(),
                extension: "pdf".into(),
                size: 1,
                modified_unix: 0,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
            FileRecord {
                path: "reports/q1-report.pdf".into(),
                name: "q1-report.pdf".into(),
                extension: "pdf".into(),
                size: 1,
                modified_unix: 0,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
            FileRecord {
                path: "other/Q1_Report.pdf".into(),
                name: "Q1_Report.pdf".into(),
                extension: "pdf".into(),
                size: 1,
                modified_unix: 0,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
        ],
        issues: Vec::new(),
    };

    let findings = analyze_index_naming(&index, &config).expect("index analysis must succeed");

    assert_eq!(
        findings.len(),
        1,
        "unscoped file must not receive a guessed rule"
    );
    assert_eq!(findings[0].source, Path::new("/work/reports/Q1_Report.pdf"));
    assert_eq!(findings[0].proposed_target, Some("q1-report.pdf".into()));
    assert!(
        findings[0].blocked,
        "existing target must block a future rename plan"
    );
    assert!(findings[0]
        .issues
        .iter()
        .any(|issue| issue.kind == NamingIssueKind::Collision));
}

#[test]
fn validation_rejects_unsupported_naming_values_before_review() {
    let mut config = UserConfig::starter();
    config
        .profiles
        .get_mut("reports")
        .expect("starter reports profile")
        .naming
        .case = Some("camel".to_string());

    let error = config
        .validate()
        .expect_err("unsupported case must not survive config validation");

    assert!(error.to_string().contains("unsupported case value camel"));
}

#[test]
fn shortcuts_expand_filename_tokens_without_becoming_keyboard_shortcuts() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
profiles:
  reports:
    naming:
      case: lower
      separator: kebab
      shortcuts:
        rpt: report
scopes:
  - path: /work
    profile: reports
"#,
    )
    .expect("shortcut config must parse");
    let path = Path::new("/work/rpt-Q1.pdf");
    let rule = resolve_naming_rule(&config, path)
        .expect("scope resolution")
        .expect("scope match");

    let finding = analyze_naming(path, &rule, &[]).expect("analysis");

    assert_eq!(finding.proposed_target.as_deref(), Some("report-q1.pdf"));
}

#[test]
fn transforms_expand_variables_aliases_replacements_reposition_and_prefix() {
    let config: UserConfig = serde_yaml::from_str(
        r#"
version: 1
defaults:
  naming: {}
profiles:
  reports:
    naming:
      case: lower
      separator: kebab
      aliases:
        rpt: report
      variables:
        project: kept
      prefix:
        required: "{{project}}"
      replacements:
        - from: final
          to: draft
      reposition:
        - token: "2026"
          position: back
scopes:
  - path: /work/reports
    profile: reports
    recursive: true
    priority: 100
"#,
    )
    .expect("user config must parse");
    let path = Path::new("/work/reports/RPT 2026 FINAL.pdf");
    let rule = resolve_naming_rule(&config, path)
        .expect("scope resolution")
        .expect("scope match");

    let finding = analyze_naming(path, &rule, &[]).expect("analysis");

    assert_eq!(
        finding.proposed_target,
        Some("kept-report-draft-2026.pdf".into())
    );
}
