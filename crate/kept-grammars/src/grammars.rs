//! Language grammar definitions and configurations.
//!
//! Provides LanguageConfig, grammar registry, and query definitions
//! for supported syntax and tree-sitter languages in bl1nk-kept.

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use thiserror::Error;

// NOTE-001: GrammarError รวม error ทั้งหมดที่เกิดจากการ parse และค้นหา grammar configuration
#[derive(Debug, Error)]
pub enum GrammarError {
    #[error("failed to parse language configuration: {0}")]
    ConfigParse(String),
    #[error("grammar not found for language: {0}")]
    NotFound(String),
}

/// Language configuration representing grammar metadata, extensions, and comment styles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LanguageConfig {
    pub name: String,
    pub grammar: String,
    #[serde(default)]
    pub path_suffixes: Vec<String>,
    #[serde(default)]
    pub line_comments: Vec<String>,
    #[serde(default)]
    pub block_comment: Option<(String, String)>,
    #[serde(default)]
    pub tab_size: Option<u32>,
    #[serde(default)]
    pub hard_tabs: Option<bool>,
}

impl LanguageConfig {
    // NOTE-002: constructor พื้นฐานสำหรับสร้าง LanguageConfig แบบ programmatic
    pub fn new(name: impl Into<String>, grammar: impl Into<String>, suffixes: &[&str]) -> Self {
        Self {
            name: name.into(),
            grammar: grammar.into(),
            path_suffixes: suffixes.iter().map(|s| s.to_string()).collect(),
            line_comments: Vec::new(),
            block_comment: None,
            tab_size: Some(4),
            hard_tabs: Some(false),
        }
    }

    pub fn with_line_comment(mut self, comment: impl Into<String>) -> Self {
        self.line_comments.push(comment.into());
        self
    }

    pub fn with_block_comment(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.block_comment = Some((start.into(), end.into()));
        self
    }

    /// Load and parse LanguageConfig from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self, GrammarError> {
        // NOTE-003: parse config จาก TOML format เพื่อรองรับ Zed extension grammar spec
        toml::from_str(content).map_err(|e| GrammarError::ConfigParse(e.to_string()))
    }

    /// Check if this language matches a given file extension.
    pub fn matches_extension(&self, ext: &str) -> bool {
        let clean_ext = ext.trim_start_matches('.');
        self.path_suffixes
            .iter()
            .any(|suffix| suffix.eq_ignore_ascii_case(clean_ext))
    }
}

// NOTE-004: รายการภาษาในระบบที่รองรับโดย bl1nk-kept ฝังเป็น static lazy table
static BUILTIN_LANGUAGES: LazyLock<Vec<LanguageConfig>> = LazyLock::new(|| {
    vec![
        LanguageConfig::new("rust", "rust", &["rs"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/"),
        LanguageConfig::new("python", "python", &["py", "pyi"]).with_line_comment("#"),
        LanguageConfig::new("javascript", "javascript", &["js", "jsx", "mjs", "cjs"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/"),
        LanguageConfig::new("typescript", "typescript", &["ts", "tsx", "mts", "cts"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/"),
        LanguageConfig::new("go", "go", &["go"])
            .with_line_comment("//")
            .with_block_comment("/*", "*/"),
        LanguageConfig::new("html", "html", &["html", "htm"]).with_block_comment("<!--", "-->"),
        LanguageConfig::new("markdown", "markdown", &["md", "mdx", "markdown"])
            .with_block_comment("<!--", "-->"),
        LanguageConfig::new("json", "json", &["json", "jsonc"]),
        LanguageConfig::new("yaml", "yaml", &["yaml", "yml"]).with_line_comment("#"),
        LanguageConfig::new("bash", "bash", &["sh", "bash"]).with_line_comment("#"),
        LanguageConfig::new("toml", "toml", &["toml"]).with_line_comment("#"),
    ]
});

/// Built-in language definitions for languages supported by bl1nk-kept.
pub fn builtin_languages() -> &'static [LanguageConfig] {
    &BUILTIN_LANGUAGES
}

/// Find a language configuration by language name or grammar name.
pub fn find_language(name: &str) -> Option<&'static LanguageConfig> {
    builtin_languages().iter().find(|lang| {
        lang.name.eq_ignore_ascii_case(name) || lang.grammar.eq_ignore_ascii_case(name)
    })
}

/// Load a language config by name, returning GrammarError if not found.
pub fn load_config(name: &str) -> Result<LanguageConfig, GrammarError> {
    find_language(name)
        .cloned()
        .ok_or_else(|| GrammarError::NotFound(name.to_string()))
}

/// Load config stripping grammar fields when grammars are not loaded.
pub fn load_config_for_feature(
    name: &str,
    grammars_loaded: bool,
) -> Result<LanguageConfig, GrammarError> {
    let mut config = load_config(name)?;
    if !grammars_loaded {
        config.path_suffixes.clear();
    }
    Ok(config)
}

/// Find a language configuration by file extension (with or without leading dot).
pub fn find_language_by_extension(ext: &str) -> Option<&'static LanguageConfig> {
    let clean_ext = ext.trim_start_matches('.');
    builtin_languages()
        .iter()
        .find(|lang| lang.matches_extension(clean_ext))
}

/// Get all supported grammar names.
pub fn supported_grammar_names() -> Vec<&'static str> {
    builtin_languages()
        .iter()
        .map(|l| l.grammar.as_str())
        .collect()
}
