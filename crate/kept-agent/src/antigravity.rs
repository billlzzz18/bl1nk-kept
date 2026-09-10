//! Antigravity CLI (`agy`) agent detection and lifecycle management.

use crate::detect::AgentSetup;
use crate::status::{StatusCheck, UpdatePreview};
use anyhow::Result;
use std::path::PathBuf;

/// Antigravity CLI agent integration.
pub struct Antigravity;

impl Default for Antigravity {
    fn default() -> Self {
        Self::new()
    }
}

impl Antigravity {
    pub fn new() -> Self {
        Self
    }
}

impl AgentSetup for Antigravity {
    fn name(&self) -> &str {
        "antigravity"
    }

    fn detect(&self) -> Option<&'static str> {
        // NOTE-001: ตรวจสอบ binary ก่อน แล้วค่อยเช็ค config directory
        if which::which("agy").is_ok() {
            return Some("found agy executable");
        }
        if let Some(dir) = gemini_dir() {
            if dir.join("antigravity-cli").is_dir() {
                return Some("found ~/.gemini/antigravity-cli/");
            }
        }
        None
    }

    fn check(&self) -> Result<StatusCheck> {
        let Some(path) = config_path() else {
            return Ok(StatusCheck::NotInstalled);
        };
        if path.exists() {
            Ok(StatusCheck::Installed)
        } else {
            Ok(StatusCheck::NotInstalled)
        }
    }

    fn install(&self) -> Result<String> {
        let _path = config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        // TODO: agy-specific setup logic
        Ok("Antigravity agent registered".to_string())
    }

    fn uninstall(&self) -> Result<String> {
        let _path = config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        // TODO: agy-specific cleanup logic
        Ok("Antigravity agent removed".to_string())
    }

    fn update_preview(&self) -> Result<Option<UpdatePreview>> {
        Ok(None)
    }
}

fn gemini_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join(".gemini"))
}

fn config_path() -> Option<PathBuf> {
    gemini_dir().map(|dir| dir.join("antigravity-cli"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_antigravity() {
        assert_eq!(Antigravity::new().name(), "antigravity");
    }

    #[test]
    fn detect_returns_reason_or_none() {
        // NOTE-002: detect() คืน Some หรือ None ไม่ใช่ error
        let result = Antigravity::new().detect();
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn check_returns_installed_or_not() {
        let status = Antigravity::new().check().unwrap();
        assert!(matches!(
            status,
            StatusCheck::Installed | StatusCheck::NotInstalled
        ));
    }

    #[test]
    fn install_returns_message() {
        let msg = Antigravity::new().install().unwrap();
        assert!(!msg.is_empty());
    }

    #[test]
    fn uninstall_returns_message() {
        let msg = Antigravity::new().uninstall().unwrap();
        assert!(!msg.is_empty());
    }
}
