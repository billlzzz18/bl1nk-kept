use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
