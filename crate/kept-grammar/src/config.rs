use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::types::*;

/// Default user config path for the current platform.
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

/// Create starter config if missing. Never overwrites existing user config.
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
    // NOTE: validation stays in kept-core (depends on semantic module)
    Ok(config)
}

pub fn save_user_config(path: &Path, config: &UserConfig) -> Result<(), PolicyError> {
    // NOTE: validation stays in kept-core (depends on semantic module)
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
    description: "Base profile for user-selected path scopes"
  reports:
    description: "Report documents with defined name format"
    naming:
      extensions: [pdf, docx, xlsx]
  source-code:
    description: "Source code preserving case and separator"
    naming:
      extensions: [rs, ts, tsx, py, go, java, kt]
  images-media:
    description: "Media with safe baseline only"
    naming:
      extensions: [jpg, jpeg, png, webp, mp4, mov, mp3]
  archives:
    description: "Archives with safe baseline only"
    naming:
      extensions: [zip, tar, gz, 7z, rar]
"#
}

/// Validate naming settings (grammar rules).
pub fn validate_naming_settings(naming: &NamingSettings) -> Result<(), PolicyError> {
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
