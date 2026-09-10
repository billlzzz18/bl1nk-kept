//! Claude Code agent detection and lifecycle management.
//!
//! Detects Claude Code via the Claude config directory.
//! Installs hooks by merging into Claude Code settings.json.

use crate::detect::AgentSetup;
use crate::hooks;
use crate::json_config::{self, EmptyJsonRoot, JsonHookInstallSpec, JsonHookUninstallSpec};
use crate::status::{StatusCheck, UpdatePreview};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::PathBuf;

/// Claude Code agent integration.
pub struct ClaudeCode;

impl Default for ClaudeCode {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCode {
    pub fn new() -> Self {
        Self
    }
}

impl AgentSetup for ClaudeCode {
    fn name(&self) -> &str {
        "claude"
    }

    fn detect(&self) -> Option<&'static str> {
        let dir = claude_dir()?;
        if dir.is_dir() {
            Some("found Claude config directory")
        } else {
            None
        }
    }

    fn check(&self) -> Result<StatusCheck> {
        let Some(path) = settings_path() else {
            return Ok(StatusCheck::NotInstalled);
        };
        if !path.exists() {
            return Ok(StatusCheck::NotInstalled);
        }
        let content =
            std::fs::read_to_string(&path).context("Failed to read ~/.claude/settings.json")?;
        let settings: Value =
            serde_json::from_str(&content).context("~/.claude/settings.json is not valid JSON")?;
        Ok(check_settings(&settings))
    }

    fn install(&self) -> Result<String> {
        let path =
            settings_path().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        json_config::json_hook_install_with(
            &path,
            &bundled_hooks()?,
            &JsonHookInstallSpec {
                read_context: "Failed to read ~/.claude/settings.json",
                parse_context: "~/.claude/settings.json is not valid JSON",
                write_context: "Failed to write ~/.claude/settings.json",
                mkdir_context: "Failed to create ~/.claude/ directory",
                empty_root: EmptyJsonRoot::Object,
            },
            hooks::merge_missing_hook_commands,
        )?;
        Ok(format!("Installed hooks to {}", path.display()))
    }

    fn uninstall(&self) -> Result<String> {
        let Some(path) = settings_path() else {
            return Ok("Claude Code config dir not found, nothing to uninstall".to_string());
        };
        uninstall_at(path)
    }

    fn update_preview(&self) -> Result<Option<UpdatePreview>> {
        let Some(path) = settings_path().filter(|p| p.exists()) else {
            return Ok(None);
        };
        let content =
            std::fs::read_to_string(&path).context("Failed to read ~/.claude/settings.json")?;
        let installed: Value =
            serde_json::from_str(&content).context("~/.claude/settings.json is not valid JSON")?;
        let mut bundled = installed.clone();
        hooks::merge_missing_hook_commands(&mut bundled, &bundled_hooks()?)?;
        Ok(Some(UpdatePreview {
            label: path.display().to_string(),
            installed: serde_json::to_string_pretty(&installed)? + "\n",
            bundled: serde_json::to_string_pretty(&bundled)? + "\n",
        }))
    }
}

// NOTE-001: claude_dir รองรับ CLAUDE_CONFIG_DIR env override สำหรับ testing
fn claude_dir_from_config(home: PathBuf, config_dir: Option<std::ffi::OsString>) -> PathBuf {
    config_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"))
}

fn claude_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| claude_dir_from_config(home, std::env::var_os("CLAUDE_CONFIG_DIR")))
}

fn settings_path() -> Option<PathBuf> {
    claude_dir().map(|d| d.join("settings.json"))
}

fn bundled_hooks() -> Result<Value> {
    // NOTE-002: hooks ฝังอยู่ใน binary ตอน compile — ถ้า plugin.json ไม่มี hooks key จะ error ทันที
    hooks::hooks_from_embedded(BUNDLED_HOOKS_JSON, "embedded Claude hooks missing hooks key")
}

// NOTE-003: hooks นี้เป็น template สำหรับ Claude Code agent — ไม่ใช่ workmux-specific
// แต่เป็น hooks ที่ Claude Code ใช้ manage agent lifecycle
const BUNDLED_HOOKS_JSON: &str = r#"{
    "hooks": {
        "SessionStart": [{
            "matcher": "startup|resume|clear|fork",
            "hooks": [{"type": "command", "command": "workmux register-agent"}]
        }],
        "UserPromptSubmit": [{
            "hooks": [{"type": "command", "command": "workmux set-window-status working"}]
        }],
        "Notification": [{
            "hooks": [{"type": "command", "command": "workmux set-window-status working"}]
        }],
        "PostToolUse": [{
            "matcher": ".*",
            "hooks": [{"type": "command", "command": "workmux set-window-status working"}]
        }],
        "Stop": [{
            "hooks": [{"type": "command", "command": "workmux set-window-status done"}]
        }]
    }
}"#;

/// Check settings for hook installation status.
fn check_settings(settings: &Value) -> StatusCheck {
    // Check plugin installation
    if let Some(plugins) = settings.get("enabledPlugins").and_then(|v| v.as_object()) {
        if plugins
            .iter()
            .any(|(key, enabled)| key.starts_with("workmux-status@") && enabled == true)
        {
            return StatusCheck::Installed;
        }
    }

    // Check manual hook installation
    let Ok(required) = bundled_hooks() else {
        return StatusCheck::NotInstalled;
    };
    if hooks::has_required_hook_commands(settings, &required) {
        return StatusCheck::Installed;
    }
    if hooks::has_workmux_hooks(settings) {
        return StatusCheck::UpdateAvailable;
    }

    StatusCheck::NotInstalled
}

fn uninstall_at(path: PathBuf) -> Result<String> {
    json_config::json_hook_uninstall(
        &path,
        &JsonHookUninstallSpec {
            messages: json_config::JsonHookUninstallMessages {
                file_missing: "No Claude Code settings.json found",
                not_found: "No workmux hooks found in Claude Code settings",
                soft_read_error: None,
                soft_parse_error: None,
            },
            delete_if_no_hooks_remain: false,
            remove_plugins: true,
            soft_errors: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn name_is_claude() {
        assert_eq!(ClaudeCode::new().name(), "claude");
    }

    #[test]
    fn claude_dir_respects_env() {
        let path = claude_dir_from_config(
            PathBuf::from("/home/test"),
            Some(std::ffi::OsString::from("/tmp/test-claude-cfg")),
        );
        assert_eq!(path, PathBuf::from("/tmp/test-claude-cfg"));
    }

    #[test]
    fn claude_dir_defaults_to_home() {
        let path = claude_dir_from_config(PathBuf::from("/home/test"), None);
        assert_eq!(path, PathBuf::from("/home/test/.claude"));
    }

    #[test]
    fn check_settings_empty() {
        let settings = json!({});
        assert!(matches!(check_settings(&settings), StatusCheck::NotInstalled));
    }

    #[test]
    fn check_settings_plugin_enabled() {
        let settings = json!({
            "enabledPlugins": {
                "workmux-status@workmux": true
            }
        });
        assert!(matches!(check_settings(&settings), StatusCheck::Installed));
    }

    #[test]
    fn check_settings_plugin_disabled() {
        let settings = json!({
            "enabledPlugins": {
                "workmux-status@workmux": false
            }
        });
        assert!(matches!(check_settings(&settings), StatusCheck::NotInstalled));
    }

    #[test]
    fn check_settings_other_plugins_only() {
        let settings = json!({
            "enabledPlugins": {
                "some-other-plugin@1.0": true
            }
        });
        assert!(matches!(check_settings(&settings), StatusCheck::NotInstalled));
    }

    #[test]
    fn check_settings_incomplete_hooks_need_upgrade() {
        let settings = json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": "workmux register-agent"
                    }]
                }],
                "Stop": [{
                    "hooks": [{
                        "type": "command",
                        "command": "workmux set-window-status done"
                    }]
                }]
            }
        });
        assert!(matches!(check_settings(&settings), StatusCheck::UpdateAvailable));
    }

    #[test]
    fn check_settings_complete_hooks_installed() {
        let required = bundled_hooks().unwrap();
        let mut settings = json!({});
        hooks::merge_missing_hook_commands(&mut settings, &required).unwrap();

        let has_hooks = settings.get("hooks").is_some();
        assert!(has_hooks, "merge should add hooks key");

        let all_present = hooks::has_required_hook_commands(&settings, &required);

        // Check each event individually
        for (event, req_entries) in required.as_object().unwrap() {
            let settings_entries = settings["hooks"].get(event).and_then(|v| v.as_array());
        }

        assert!(all_present, "all required hook commands should be present after merge");
    }

    #[test]
    fn check_settings_both_plugin_and_hooks() {
        let settings = json!({
            "enabledPlugins": {
                "workmux-status@workmux": true
            },
            "hooks": {
                "Stop": [{
                    "hooks": [{
                        "type": "command",
                        "command": "workmux set-window-status done"
                    }]
                }]
            }
        });
        assert!(matches!(check_settings(&settings), StatusCheck::Installed));
    }

    #[test]
    fn uninstall_no_settings_file() {
        let tmp = tempfile::tempdir().unwrap();
        let settings_path = tmp.path().join("settings.json");
        let result = uninstall_at(settings_path).unwrap();
        assert!(result.contains("No Claude Code settings.json"));
    }

    #[test]
    fn uninstall_no_hooks_present() {
        let tmp = tempfile::tempdir().unwrap();
        let settings_path = tmp.path().join("settings.json");
        std::fs::write(&settings_path, r#"{"someSetting": true}"#).unwrap();
        let result = uninstall_at(settings_path).unwrap();
        assert!(result.contains("No workmux hooks found"));
    }

    #[test]
    fn uninstall_removes_hooks_only() {
        let tmp = tempfile::tempdir().unwrap();
        let settings_path = tmp.path().join("settings.json");
        std::fs::write(
            &settings_path,
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"workmux set-window-status done"}]},{"hooks":[{"type":"command","command":"afplay glass.aiff"}]}]}}"#,
        )
        .unwrap();
        let result = uninstall_at(settings_path.clone()).unwrap();
        assert!(result.contains("Removed workmux hooks"), "result: {result}");
        let content = std::fs::read_to_string(&settings_path).unwrap();
        let settings: Value = serde_json::from_str(&content).unwrap();
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 1);
        assert!(stop[0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("glass"));
    }

    #[test]
    fn uninstall_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let settings_path = tmp.path().join("settings.json");
        std::fs::write(
            &settings_path,
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"workmux done"}]}]}}"#,
        )
        .unwrap();
        let result1 = uninstall_at(settings_path.clone()).unwrap();
        assert!(result1.contains("Removed workmux"));
        let result2 = uninstall_at(settings_path).unwrap();
        assert!(result2.contains("No workmux hooks found"));
    }
}
