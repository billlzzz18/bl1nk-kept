use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

pub const USER_CONFIG_VERSION: u32 = 1;
pub const USER_CONFIG_FILE_NAME: &str = "config.yaml";

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("invalid user config: {0}")]
    InvalidConfig(String),
    #[error("ambiguous naming rule for {path}: {left} and {right}")]
    AmbiguousScope {
        path: String,
        left: String,
        right: String,
    },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

// NOTE-001: `config.yaml` เป็นการตั้งค่าของผู้ใช้เท่านั้น; ระบบใช้ค่าใน struct เป็น fallback แต่ไม่ rewrite ไฟล์เดิมของผู้ใช้
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserConfig {
    pub version: u32,
    #[serde(default)]
    pub defaults: ConfigDefaults,
    #[serde(default)]
    pub profiles: BTreeMap<String, NamingProfile>,
    #[serde(default)]
    pub scopes: Vec<NamingScope>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDefaults {
    #[serde(default)]
    pub naming: NamingSettings,
    #[serde(default)]
    pub semantic: SemanticSearchSettings,
}

// NOTE-002: ตั้งค่าผ่าน model ID และ endpoint; provider รองรับ "ollama" (default) และ "local" เท่านั้น
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchSettings {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub embedding_model_id: Option<String>,
    #[serde(default)]
    pub rerank_model_id: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingProfile {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub naming: NamingSettings,
}

// NOTE-003: path scope ต้องมาจาก absolute path จริงที่ผู้ใช้เลือกหรือพิมพ์ ไม่เดาจาก root ของการ scan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingScope {
    pub path: PathBuf,
    pub profile: String,
    #[serde(default = "default_recursive")]
    pub recursive: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub exceptions: Vec<PathBuf>,
    #[serde(default)]
    pub overrides: ScopeOverrides,
}

fn default_recursive() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeOverrides {
    #[serde(default)]
    pub naming: NamingSettings,
}

// NOTE-004: field ทั้งหมด optional เพื่อให้ defaults → profile → scope overrides merge ได้โดยไม่ทำให้ profile ที่ต่างกันนิดเดียวชนกัน
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NamingSettings {
    #[serde(default)]
    pub unicode: Option<String>,
    #[serde(default)]
    pub case: Option<String>,
    #[serde(default)]
    pub separator: Option<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub flag_control_characters: Option<bool>,
    #[serde(default)]
    pub flag_trim_whitespace: Option<bool>,
    #[serde(default)]
    pub flag_case_collisions: Option<bool>,
    #[serde(default)]
    pub flag_portability_conflicts: Option<bool>,
    #[serde(default)]
    pub numbers: NumberSettings,
    #[serde(default)]
    pub words: WordSettings,
    #[serde(default)]
    pub length: LengthSettings,
    #[serde(default)]
    pub whitespace: WhitespaceSettings,
    #[serde(default)]
    pub prefix: PrefixSettings,
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub shortcuts: BTreeMap<String, String>,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    #[serde(default)]
    pub similarity: SimilaritySettings,
    #[serde(default)]
    pub replacements: Vec<TokenReplacement>,
    #[serde(default)]
    pub reposition: Vec<TokenReposition>,
    #[serde(default)]
    pub stem_regex: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NumberSettings {
    #[serde(default)]
    pub allow: Option<bool>,
    #[serde(default)]
    pub max_digits_per_token: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WordSettings {
    #[serde(default)]
    pub min: Option<usize>,
    #[serde(default)]
    pub max: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LengthSettings {
    #[serde(default)]
    pub stem: Option<RangeLimit>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RangeLimit {
    #[serde(default)]
    pub min: Option<usize>,
    #[serde(default)]
    pub max: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WhitespaceSettings {
    #[serde(default)]
    pub trim: Option<bool>,
    #[serde(default)]
    pub collapse_internal: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrefixSettings {
    #[serde(default)]
    pub required: Option<String>,
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SimilaritySettings {
    #[serde(default)]
    pub name: Option<SimilarityRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SimilarityRule {
    pub threshold: f64,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenReplacement {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenReposition {
    pub token: String,
    pub position: TokenPosition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenPosition {
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedNamingRule {
    pub scope_path: PathBuf,
    pub profile_id: String,
    pub naming: NamingSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamingIssueKind {
    Unicode,
    Case,
    Separator,
    ControlCharacter,
    TrimWhitespace,
    Prefix,
    StemRegex,
    Length,
    WordCount,
    Number,
    Portability,
    Collision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamingIssue {
    pub kind: NamingIssueKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingFinding {
    pub source: PathBuf,
    pub rule_id: String,
    pub issues: Vec<NamingIssue>,
    pub proposed_target: Option<String>,
    pub blocked: bool,
}

impl UserConfig {
    pub fn starter() -> Self {
        let defaults = NamingSettings {
            unicode: Some("nfc".to_string()),
            case: Some("preserve".to_string()),
            separator: Some("preserve".to_string()),
            flag_control_characters: Some(true),
            flag_trim_whitespace: Some(true),
            flag_case_collisions: Some(true),
            flag_portability_conflicts: Some(true),
            numbers: NumberSettings {
                allow: Some(true),
                max_digits_per_token: Some(8),
            },
            words: WordSettings { min: Some(1), max: Some(12) },
            length: LengthSettings {
                stem: Some(RangeLimit { min: Some(1), max: Some(120) }),
            },
            whitespace: WhitespaceSettings {
                trim: Some(true),
                collapse_internal: Some(false),
            },
            similarity: SimilaritySettings {
                name: Some(SimilarityRule {
                    threshold: 0.90,
                    case_sensitive: false,
                }),
            },
            ..NamingSettings::default()
        };
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "generic".to_string(),
            NamingProfile {
                description: Some("ฐานสำหรับ path scope ที่ผู้ใช้เลือก".to_string()),
                naming: NamingSettings::default(),
            },
        );
        profiles.insert(
            "reports".to_string(),
            NamingProfile {
                description: Some("เอกสารรายงานที่กำหนดรูปแบบชื่อได้".to_string()),
                naming: NamingSettings {
                    extensions: vec!["pdf".to_string(), "docx".to_string(), "xlsx".to_string()],
                    ..NamingSettings::default()
                },
            },
        );
        profiles.insert(
            "source-code".to_string(),
            NamingProfile {
                description: Some("source code ที่รักษา case และ separator เดิม".to_string()),
                naming: NamingSettings {
                    extensions: vec![
                        "rs".to_string(),
                        "ts".to_string(),
                        "tsx".to_string(),
                        "py".to_string(),
                        "go".to_string(),
                        "java".to_string(),
                        "kt".to_string(),
                    ],
                    ..NamingSettings::default()
                },
            },
        );
        profiles.insert(
            "images-media".to_string(),
            NamingProfile {
                description: Some("สื่อที่ตรวจ safe baseline โดยไม่เดาชื่อจาก content".to_string()),
                naming: NamingSettings {
                    extensions: vec![
                        "jpg".to_string(),
                        "jpeg".to_string(),
                        "png".to_string(),
                        "webp".to_string(),
                        "mp4".to_string(),
                        "mov".to_string(),
                        "mp3".to_string(),
                    ],
                    ..NamingSettings::default()
                },
            },
        );
        profiles.insert(
            "archives".to_string(),
            NamingProfile {
                description: Some("archive ที่ตรวจ safe baseline เท่านั้น".to_string()),
                naming: NamingSettings {
                    extensions: vec![
                        "zip".to_string(),
                        "tar".to_string(),
                        "gz".to_string(),
                        "7z".to_string(),
                        "rar".to_string(),
                    ],
                    ..NamingSettings::default()
                },
            },
        );
        Self {
            version: USER_CONFIG_VERSION,
            defaults: ConfigDefaults {
                naming: defaults,
                semantic: SemanticSearchSettings::default(),
            },
            profiles,
            scopes: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.version != USER_CONFIG_VERSION {
            return Err(PolicyError::InvalidConfig(format!(
                "unsupported config version {}",
                self.version
            )));
        }
        validate_naming_settings(&self.defaults.naming)?;
        crate::semantic::validate_semantic_settings(&self.defaults.semantic)?;
        for (profile_id, profile) in &self.profiles {
            validate_naming_settings(&profile.naming).map_err(|error| {
                PolicyError::InvalidConfig(format!("profile {profile_id}: {error}"))
            })?;
        }
        for scope in &self.scopes {
            if !is_absolute_path(&scope.path) {
                return Err(PolicyError::InvalidConfig(format!(
                    "scope path must be absolute: {}",
                    scope.path.display()
                )));
            }
            if !self.profiles.contains_key(&scope.profile) {
                return Err(PolicyError::InvalidConfig(format!(
                    "scope {} references missing profile {}",
                    scope.path.display(),
                    scope.profile
                )));
            }
            if scope.exceptions.iter().any(|path| !is_absolute_path(path)) {
                return Err(PolicyError::InvalidConfig(format!(
                    "scope {} has a relative exception",
                    scope.path.display()
                )));
            }
            validate_naming_settings(&scope.overrides.naming).map_err(|error| {
                PolicyError::InvalidConfig(format!("scope {}: {error}", scope.path.display()))
            })?;
        }
        Ok(())
    }
}

fn validate_naming_settings(naming: &NamingSettings) -> Result<(), PolicyError> {
    if let Some(value) = &naming.unicode {
        if value != "nfc" {
            return Err(PolicyError::InvalidConfig(format!("unsupported unicode value {value}")));
        }
    }
    if let Some(value) = &naming.case {
        if !matches!(value.as_str(), "lower" | "upper" | "preserve") {
            return Err(PolicyError::InvalidConfig(format!("unsupported case value {value}")));
        }
    }
    if let Some(value) = &naming.separator {
        if !matches!(value.as_str(), "kebab" | "snake" | "preserve") {
            return Err(PolicyError::InvalidConfig(format!("unsupported separator value {value}")));
        }
    }
    if let Some(rule) = &naming.similarity.name {
        if !(0.0..=1.0).contains(&rule.threshold) {
            return Err(PolicyError::InvalidConfig(format!(
                "similarity threshold must be between 0.0 and 1.0: {}",
                rule.threshold
            )));
        }
    }
    if let Some(range) = &naming.length.stem {
        if range
            .min
            .is_some_and(|min| range.max.is_some_and(|max| min > max))
        {
            return Err(PolicyError::InvalidConfig(
                "length.stem min cannot exceed max".to_string(),
            ));
        }
    }
    if naming
        .words
        .min
        .is_some_and(|min| naming.words.max.is_some_and(|max| min > max))
    {
        return Err(PolicyError::InvalidConfig("words min cannot exceed max".to_string()));
    }
    if let Some(pattern) = &naming.stem_regex {
        Regex::new(pattern).map_err(|error| {
            PolicyError::InvalidConfig(format!("invalid stemRegex {pattern}: {error}"))
        })?;
    }
    if naming.extensions.iter().any(|extension| {
        extension.is_empty()
            || extension.starts_with('.')
            || extension.chars().any(char::is_whitespace)
    }) {
        return Err(PolicyError::InvalidConfig(
            "extensions must be non-empty values without a leading dot or whitespace".to_string(),
        ));
    }
    Ok(())
}

pub fn default_user_config_path() -> Result<PathBuf, PolicyError> {
    #[cfg(target_os = "windows")]
    {
        let base = env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("USERPROFILE")
                    .map(PathBuf::from)
                    .map(|path| path.join("AppData").join("Roaming"))
            })
            .ok_or_else(|| PolicyError::InvalidConfig("cannot resolve APPDATA".to_string()))?;
        Ok(base.join("kept").join(USER_CONFIG_FILE_NAME))
    }
    #[cfg(target_os = "macos")]
    {
        let home = home_dir()?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("kept")
            .join(USER_CONFIG_FILE_NAME))
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let base = env::var_os("XDG_CONFIG_HOME")
            .filter(|value| Path::new(value).is_absolute())
            .map(PathBuf::from)
            .unwrap_or(home_dir()?.join(".config"));
        Ok(base.join("kept").join(USER_CONFIG_FILE_NAME))
    }
}

#[cfg(not(target_os = "windows"))]
fn home_dir() -> Result<PathBuf, PolicyError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| PolicyError::InvalidConfig("cannot resolve user home directory".to_string()))
}

// NOTE-004: สร้าง starter config เพียงเมื่อยังไม่มีไฟล์ ห้าม rewrite การตั้งค่าที่ผู้ใช้แก้เอง
pub fn create_user_config_if_missing(path: &Path) -> Result<bool, PolicyError> {
    if path.exists() {
        return Ok(false);
    }
    let parent = path.parent().ok_or_else(|| {
        PolicyError::InvalidConfig(format!("config path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    fs::write(path, starter_config_yaml())?;
    Ok(true)
}

pub fn load_user_config(path: &Path) -> Result<UserConfig, PolicyError> {
    let content = fs::read_to_string(path)?;
    let config: UserConfig = serde_yaml::from_str(&content)?;
    config.validate()?;
    Ok(config)
}

// NOTE-005: config ถูกเขียนได้เมื่อผู้ใช้สั่ง task-level mutation เท่านั้น และต้อง validate ก่อนเขียนทุกครั้งเพื่อไม่สร้าง YAML ที่ doctor อ่านไม่ได้
pub fn save_user_config(path: &Path, config: &UserConfig) -> Result<(), PolicyError> {
    config.validate()?;
    let content = serde_yaml::to_string(config)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn starter_config_yaml() -> &'static str {
    r#"# config.yaml is user-owned. kept creates it once and never rewrites it.
version: 1

defaults:
  naming:
    unicode: nfc
    case: preserve
    separator: preserve
    flagControlCharacters: true
    flagTrimWhitespace: true
    flagCaseCollisions: true
    flagPortabilityConflicts: true
    numbers:
      allow: true
      maxDigitsPerToken: 8
    words:
      min: 1
      max: 12
    length:
      stem:
        min: 1
        max: 120
    whitespace:
      trim: true
      collapseInternal: false
    similarity:
      name:
        threshold: 0.9
        caseSensitive: false

profiles:
  generic:
    description: "Base profile for a user-selected absolute path."
    naming: {}
  reports:
    description: "Documents whose naming convention can be made consistent."
    naming:
      extensions: [pdf, docx, xlsx]
  source-code:
    description: "Source code: keep project case and separators unless a scope overrides them."
    naming:
      extensions: [rs, ts, tsx, py, go, java, kt]
  images-media:
    description: "Media: audit baseline only; do not infer a name from file content."
    naming:
      extensions: [jpg, jpeg, png, webp, mp4, mov, mp3]
  archives:
    description: "Archives: audit baseline only."
    naming:
      extensions: [zip, tar, gz, 7z, rar]

# kept adds a scope only after the user selects an absolute folder.
scopes: []
"#
}

fn is_absolute_path(path: &Path) -> bool {
    path.is_absolute() || path.has_root()
}

pub fn resolve_naming_rule(
    config: &UserConfig,
    file_path: &Path,
) -> Result<Option<ResolvedNamingRule>, PolicyError> {
    config.validate()?;
    if !is_absolute_path(file_path) {
        return Err(PolicyError::InvalidConfig(format!(
            "file path must be absolute: {}",
            file_path.display()
        )));
    }

    let extension = file_extension(file_path);
    let mut matches = Vec::new();
    for scope in &config.scopes {
        if !scope_matches(scope, file_path) {
            continue;
        }
        let profile = config.profiles.get(&scope.profile).ok_or_else(|| {
            PolicyError::InvalidConfig(format!("missing profile {}", scope.profile))
        })?;
        let naming = merge_settings(
            &merge_settings(&config.defaults.naming, &profile.naming),
            &scope.overrides.naming,
        );
        if !naming.extensions.is_empty()
            && !naming
                .extensions
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&extension))
        {
            continue;
        }
        matches.push((scope, naming));
    }

    if matches.is_empty() {
        return Ok(None);
    }
    matches.sort_by(|(left, _), (right, _)| {
        scope_depth(&right.path)
            .cmp(&scope_depth(&left.path))
            .then_with(|| right.priority.cmp(&left.priority))
            .then_with(|| left.profile.cmp(&right.profile))
    });
    let (selected_scope, selected_naming) = matches.remove(0);
    if let Some((next_scope, _)) = matches.first() {
        if scope_depth(&selected_scope.path) == scope_depth(&next_scope.path)
            && selected_scope.priority == next_scope.priority
        {
            return Err(PolicyError::AmbiguousScope {
                path: file_path.display().to_string(),
                left: selected_scope.profile.clone(),
                right: next_scope.profile.clone(),
            });
        }
    }

    Ok(Some(ResolvedNamingRule {
        scope_path: selected_scope.path.clone(),
        profile_id: selected_scope.profile.clone(),
        naming: selected_naming,
    }))
}

// NOTE-006: naming analysis อ่าน ScanIndex เดียวกับ scan/review เพื่อไม่ traversal ซ้ำ และไม่เสนอ rule ให้ไฟล์นอก absolute scope ของผู้ใช้
pub fn analyze_index_naming(
    index: &crate::scanner::ScanIndex,
    config: &UserConfig,
) -> Result<Vec<NamingFinding>, PolicyError> {
    let absolute_paths = index
        .files
        .iter()
        .map(|record| {
            let path = Path::new(&record.path);
            if is_absolute_path(path) {
                path.to_path_buf()
            } else {
                Path::new(&index.root).join(path)
            }
        })
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    for source in &absolute_paths {
        let Some(rule) = resolve_naming_rule(config, source)? else {
            continue;
        };
        let finding = analyze_naming(source, &rule, &absolute_paths)?;
        if !finding.issues.is_empty() {
            findings.push(finding);
        }
    }
    findings.sort_by(|left, right| left.source.cmp(&right.source));
    Ok(findings)
}

pub fn analyze_naming(
    source: &Path,
    rule: &ResolvedNamingRule,
    occupied_targets: &[PathBuf],
) -> Result<NamingFinding, PolicyError> {
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            PolicyError::InvalidConfig(format!("invalid file name: {}", source.display()))
        })?;
    let path_extension = file_extension(source);
    if !rule.naming.extensions.is_empty()
        && !rule
            .naming
            .extensions
            .iter()
            .any(|value| value.eq_ignore_ascii_case(&path_extension))
    {
        return Ok(NamingFinding {
            source: source.to_path_buf(),
            rule_id: rule.profile_id.clone(),
            issues: Vec::new(),
            proposed_target: None,
            blocked: false,
        });
    }

    let (stem, extension) = split_stem_and_extension(file_name);
    let mut issues = Vec::new();
    let mut target_stem = stem.to_string();

    if rule.naming.unicode.as_deref() == Some("nfc") {
        let normalized: String = target_stem.nfc().collect();
        if normalized != target_stem {
            issues.push(issue(NamingIssueKind::Unicode, "stem is not Unicode NFC"));
            target_stem = normalized;
        }
    }
    if rule.naming.flag_control_characters.unwrap_or(false)
        && target_stem.chars().any(char::is_control)
    {
        issues.push(issue(NamingIssueKind::ControlCharacter, "stem contains a control character"));
        target_stem.retain(|character| !character.is_control());
    }
    if rule
        .naming
        .whitespace
        .trim
        .or(rule.naming.flag_trim_whitespace)
        == Some(true)
    {
        let trimmed = target_stem.trim().to_string();
        if trimmed != target_stem {
            issues.push(issue(
                NamingIssueKind::TrimWhitespace,
                "stem has leading or trailing whitespace",
            ));
            target_stem = trimmed;
        }
    }

    target_stem = apply_replacements(target_stem, &rule.naming);
    target_stem = apply_shortcuts(target_stem, &rule.naming);
    target_stem = apply_aliases(target_stem, &rule.naming);
    target_stem = apply_reposition(target_stem, &rule.naming);
    target_stem = apply_required_prefix(target_stem, &rule.naming)?;

    match rule.naming.case.as_deref() {
        Some("lower") => {
            let lower = target_stem.to_lowercase();
            if lower != target_stem {
                issues.push(issue(NamingIssueKind::Case, "stem must be lowercase"));
                target_stem = lower;
            }
        },
        Some("upper") => {
            let upper = target_stem.to_uppercase();
            if upper != target_stem {
                issues.push(issue(NamingIssueKind::Case, "stem must be uppercase"));
                target_stem = upper;
            }
        },
        Some("preserve") | None => {},
        Some(value) => {
            return Err(PolicyError::InvalidConfig(format!("unsupported case value {value}")))
        },
    }

    if rule.naming.separator.as_deref() == Some("kebab") {
        let kebab = normalize_separator(&target_stem, '-');
        if kebab != target_stem {
            issues.push(issue(NamingIssueKind::Separator, "stem must use kebab separator"));
            target_stem = kebab;
        }
    } else if rule.naming.separator.as_deref() == Some("snake") {
        let snake = normalize_separator(&target_stem, '_');
        if snake != target_stem {
            issues.push(issue(NamingIssueKind::Separator, "stem must use snake separator"));
            target_stem = snake;
        }
    }

    append_constraint_issues(&target_stem, &rule.naming, &mut issues)?;
    let target_name = if extension.is_empty() {
        target_stem
    } else {
        format!("{target_stem}.{}", extension.to_lowercase())
    };
    let proposed_target = (target_name != file_name).then_some(target_name.clone());
    let target_path = source.with_file_name(&target_name);
    let blocked = proposed_target.is_some()
        && occupied_targets
            .iter()
            .any(|occupied| occupied != source && occupied == &target_path);
    if blocked {
        issues.push(issue(NamingIssueKind::Collision, "proposed target already exists"));
    }

    Ok(NamingFinding {
        source: source.to_path_buf(),
        rule_id: rule.profile_id.clone(),
        issues,
        proposed_target,
        blocked,
    })
}

fn issue(kind: NamingIssueKind, message: &str) -> NamingIssue {
    NamingIssue {
        kind,
        message: message.to_string(),
    }
}

fn normalize_path_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

fn scope_matches(scope: &NamingScope, file_path: &Path) -> bool {
    let norm_file = normalize_path_prefix(file_path);
    let norm_scope = normalize_path_prefix(&scope.path);
    if !norm_file.starts_with(&norm_scope)
        || scope
            .exceptions
            .iter()
            .any(|path| norm_file.starts_with(normalize_path_prefix(path)))
    {
        return false;
    }
    scope.recursive || norm_file.parent() == Some(norm_scope.as_path())
}

fn scope_depth(path: &Path) -> usize {
    normalize_path_prefix(path).components().count()
}

fn merge_settings(base: &NamingSettings, override_value: &NamingSettings) -> NamingSettings {
    NamingSettings {
        unicode: override_value
            .unicode
            .clone()
            .or_else(|| base.unicode.clone()),
        case: override_value.case.clone().or_else(|| base.case.clone()),
        separator: override_value
            .separator
            .clone()
            .or_else(|| base.separator.clone()),
        extensions: if override_value.extensions.is_empty() {
            base.extensions.clone()
        } else {
            override_value.extensions.clone()
        },
        flag_control_characters: override_value
            .flag_control_characters
            .or(base.flag_control_characters),
        flag_trim_whitespace: override_value
            .flag_trim_whitespace
            .or(base.flag_trim_whitespace),
        flag_case_collisions: override_value
            .flag_case_collisions
            .or(base.flag_case_collisions),
        flag_portability_conflicts: override_value
            .flag_portability_conflicts
            .or(base.flag_portability_conflicts),
        numbers: merge_numbers(&base.numbers, &override_value.numbers),
        words: merge_words(&base.words, &override_value.words),
        length: merge_length(&base.length, &override_value.length),
        whitespace: merge_whitespace(&base.whitespace, &override_value.whitespace),
        prefix: if override_value.prefix == PrefixSettings::default() {
            base.prefix.clone()
        } else {
            override_value.prefix.clone()
        },
        aliases: merge_maps(&base.aliases, &override_value.aliases),
        shortcuts: merge_maps(&base.shortcuts, &override_value.shortcuts),
        variables: merge_maps(&base.variables, &override_value.variables),
        similarity: if override_value.similarity == SimilaritySettings::default() {
            base.similarity.clone()
        } else {
            override_value.similarity.clone()
        },
        replacements: if override_value.replacements.is_empty() {
            base.replacements.clone()
        } else {
            override_value.replacements.clone()
        },
        reposition: if override_value.reposition.is_empty() {
            base.reposition.clone()
        } else {
            override_value.reposition.clone()
        },
        stem_regex: override_value
            .stem_regex
            .clone()
            .or_else(|| base.stem_regex.clone()),
    }
}

fn merge_maps(
    base: &BTreeMap<String, String>,
    override_value: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut values = base.clone();
    values.extend(override_value.clone());
    values
}

fn merge_numbers(base: &NumberSettings, override_value: &NumberSettings) -> NumberSettings {
    NumberSettings {
        allow: override_value.allow.or(base.allow),
        max_digits_per_token: override_value
            .max_digits_per_token
            .or(base.max_digits_per_token),
    }
}

fn merge_words(base: &WordSettings, override_value: &WordSettings) -> WordSettings {
    WordSettings {
        min: override_value.min.or(base.min),
        max: override_value.max.or(base.max),
    }
}

fn merge_length(base: &LengthSettings, override_value: &LengthSettings) -> LengthSettings {
    LengthSettings {
        stem: override_value.stem.clone().or_else(|| base.stem.clone()),
    }
}

fn merge_whitespace(
    base: &WhitespaceSettings,
    override_value: &WhitespaceSettings,
) -> WhitespaceSettings {
    WhitespaceSettings {
        trim: override_value.trim.or(base.trim),
        collapse_internal: override_value.collapse_internal.or(base.collapse_internal),
    }
}

fn apply_replacements(mut stem: String, naming: &NamingSettings) -> String {
    for replacement in &naming.replacements {
        let from = expand_variables(&replacement.from, &naming.variables);
        let to = expand_variables(&replacement.to, &naming.variables);
        if replacement.case_sensitive {
            stem = stem.replace(&from, &to);
        } else {
            stem = replace_case_insensitive(&stem, &from, &to);
        }
    }
    stem
}

// NOTE-007: shortcut เป็น keymap ของ token ในชื่อไฟล์ ไม่ใช่ keyboard shortcut; ทำก่อน aliases เพื่อให้ alias ของ profile override ได้ตาม merge order
fn apply_shortcuts(stem: String, naming: &NamingSettings) -> String {
    if naming.shortcuts.is_empty() {
        return stem;
    }
    split_tokens(&stem)
        .into_iter()
        .map(|token| {
            naming
                .shortcuts
                .iter()
                .find(|(shortcut, _)| shortcut.eq_ignore_ascii_case(&token))
                .map(|(_, value)| expand_variables(value, &naming.variables))
                .unwrap_or(token)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn apply_aliases(stem: String, naming: &NamingSettings) -> String {
    if naming.aliases.is_empty() {
        return stem;
    }
    split_tokens(&stem)
        .into_iter()
        .map(|token| {
            naming
                .aliases
                .iter()
                .find(|(alias, _)| alias.eq_ignore_ascii_case(&token))
                .map(|(_, value)| expand_variables(value, &naming.variables))
                .unwrap_or(token)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn apply_reposition(stem: String, naming: &NamingSettings) -> String {
    if naming.reposition.is_empty() {
        return stem;
    }
    let mut tokens = split_tokens(&stem);
    for rule in &naming.reposition {
        let token = expand_variables(&rule.token, &naming.variables);
        if let Some(index) = tokens
            .iter()
            .position(|value| value.eq_ignore_ascii_case(&token))
        {
            let value = tokens.remove(index);
            match rule.position {
                TokenPosition::Front => tokens.insert(0, value),
                TokenPosition::Back => tokens.push(value),
            }
        }
    }
    tokens.join(" ")
}

fn apply_required_prefix(mut stem: String, naming: &NamingSettings) -> Result<String, PolicyError> {
    let Some(required) = &naming.prefix.required else {
        return Ok(stem);
    };
    let prefix = expand_variables(required, &naming.variables);
    if prefix.is_empty() {
        return Err(PolicyError::InvalidConfig(
            "prefix required resolves to empty text".to_string(),
        ));
    }
    if !stem.starts_with(&prefix) {
        stem = format!("{prefix} {stem}");
    }
    Ok(stem)
}

fn append_constraint_issues(
    stem: &str,
    naming: &NamingSettings,
    issues: &mut Vec<NamingIssue>,
) -> Result<(), PolicyError> {
    if let Some(regex) = &naming.stem_regex {
        let compiled = Regex::new(regex).map_err(|error| {
            PolicyError::InvalidConfig(format!("invalid stemRegex {regex}: {error}"))
        })?;
        if !compiled.is_match(stem) {
            issues.push(issue(NamingIssueKind::StemRegex, "stem does not match stemRegex"));
        }
    }
    if let Some(limit) = &naming.length.stem {
        let count = stem.chars().count();
        if limit.min.is_some_and(|minimum| count < minimum)
            || limit.max.is_some_and(|maximum| count > maximum)
        {
            issues.push(issue(NamingIssueKind::Length, "stem length is outside configured range"));
        }
    }
    let words = split_tokens(stem).len();
    if naming.words.min.is_some_and(|minimum| words < minimum)
        || naming.words.max.is_some_and(|maximum| words > maximum)
    {
        issues.push(issue(NamingIssueKind::WordCount, "word count is outside configured range"));
    }
    if naming.numbers.allow == Some(false)
        && stem.chars().any(|character| character.is_ascii_digit())
    {
        issues.push(issue(NamingIssueKind::Number, "numbers are not allowed"));
    }
    if let Some(maximum) = naming.numbers.max_digits_per_token {
        if split_tokens(stem)
            .iter()
            .any(|token| token.chars().filter(char::is_ascii_digit).count() > maximum)
        {
            issues.push(issue(NamingIssueKind::Number, "a number token exceeds maxDigitsPerToken"));
        }
    }
    if naming.flag_portability_conflicts.unwrap_or(false) && contains_portability_conflict(stem) {
        issues.push(issue(
            NamingIssueKind::Portability,
            "stem contains a cross-platform reserved character or name",
        ));
    }
    Ok(())
}

fn contains_portability_conflict(stem: &str) -> bool {
    const RESERVED: &[&str] = &["con", "prn", "aux", "nul", "com1", "lpt1"];
    stem.chars()
        .any(|character| matches!(character, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        || RESERVED
            .iter()
            .any(|value| stem.eq_ignore_ascii_case(value))
}

fn normalize_separator(value: &str, separator: char) -> String {
    let mut output = String::new();
    let mut previous_was_separator = false;
    for character in value.chars() {
        if character.is_whitespace() || character == '_' || character == '-' {
            if !previous_was_separator && !output.is_empty() {
                output.push(separator);
            }
            previous_was_separator = true;
        } else {
            output.push(character);
            previous_was_separator = false;
        }
    }
    output.trim_matches(separator).to_string()
}

fn split_tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| character.is_whitespace() || character == '_' || character == '-')
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn expand_variables(value: &str, variables: &BTreeMap<String, String>) -> String {
    let mut result = value.to_string();
    for (name, replacement) in variables {
        result = result.replace(&format!("{{{{{name}}}}}"), replacement);
    }
    result
}

fn replace_case_insensitive(value: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return value.to_string();
    }
    if let Ok(re) = Regex::new(&format!("(?i){}", regex::escape(from))) {
        re.replace_all(value, to).to_string()
    } else {
        value.replace(from, to)
    }
}

fn file_extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

fn split_stem_and_extension(file_name: &str) -> (&str, &str) {
    match file_name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, extension),
        _ => (file_name, ""),
    }
}
