//! Antigravity (`agy`) agent ecosystem detection and lifecycle management.
//!
//! รองรับ Antigravity ทั้ง 4 Clients/Runtimes:
//! 1. **Antigravity IDE** (`antigravity-ide`) — GUI Editor / IDE Workspace Environment
//! 2. **Antigravity CLI** (`agy` / `antigravity-cli`) — Command-line interface runtime
//! 3. **Antigravity 2.0** (`agy2` / `antigravity-2.0`) — Next-gen multi-agent runtime
//! 4. **Antigravity ACP** (`agy-acp` / `antigravity-acp`) — Agent Client Protocol / Sidecar server

use crate::detect::AgentSetup;
use crate::status::{StatusCheck, UpdatePreview};
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const WORKMUX_GROUP: &str = "workmux-status";
const REGISTER_COMMAND: &str = "workmux register-agent >/dev/null 2>&1 || true; printf '{}\\n'";
const WORKING_COMMAND: &str =
    "workmux set-window-status working >/dev/null 2>&1 || true; printf '{}\\n'";
const STOP_COMMAND: &str = "workmux set-window-status done >/dev/null 2>&1 || true; printf '{}\\n'";

/// 4 รูปแบบไคลเอนต์ของ Antigravity Ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntigravityClient {
    /// 1. Antigravity IDE (`~/.gemini/antigravity-ide`)
    Ide,
    /// 2. Antigravity CLI (`agy` / `~/.gemini/antigravity-cli`)
    Cli,
    /// 3. Antigravity 2.0 (`agy2` / `~/.gemini/antigravity-2`)
    Agy2,
    /// 4. Antigravity ACP (`agy-acp` / `~/.gemini/acp` / `~/.gemini/antigravity-acp`)
    Acp,
}

impl AntigravityClient {
    pub fn all() -> [AntigravityClient; 4] {
        [
            AntigravityClient::Ide,
            AntigravityClient::Cli,
            AntigravityClient::Agy2,
            AntigravityClient::Acp,
        ]
    }

    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Ide => "antigravity-ide",
            Self::Cli => "antigravity-cli",
            Self::Agy2 => "antigravity-2.0",
            Self::Acp => "antigravity-acp",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Ide => "Antigravity IDE",
            Self::Cli => "Antigravity CLI (agy)",
            Self::Agy2 => "Antigravity 2.0 (agy2)",
            Self::Acp => "Antigravity ACP (Agent Client Protocol)",
        }
    }

    /// ตรวจจับการมีอยู่ของ Client แต่ละตัว
    pub fn detect(&self) -> Option<&'static str> {
        match self {
            Self::Ide => {
                if which::which("antigravity-ide").is_ok()
                    || which::which("antigravity-ide.cmd").is_ok()
                {
                    return Some("found antigravity-ide executable in PATH");
                }
                if gemini_dir().is_some_and(|dir| dir.join("antigravity-ide").is_dir()) {
                    return Some("found ~/.gemini/antigravity-ide/");
                }
                if std::env::var_os("ANTIGRAVITY_IDE").is_some() {
                    return Some("found ANTIGRAVITY_IDE environment");
                }
                None
            }
            Self::Cli => {
                if which::which("agy").is_ok() {
                    return Some("found agy executable in PATH");
                }
                if gemini_dir().is_some_and(|dir| dir.join("antigravity-cli").is_dir()) {
                    return Some("found ~/.gemini/antigravity-cli/");
                }
                None
            }
            Self::Agy2 => {
                if which::which("agy2").is_ok() {
                    return Some("found agy2 executable in PATH");
                }
                if gemini_dir().is_some_and(|dir| {
                    dir.join("antigravity").is_dir()
                        || dir.join("antigravity-2").is_dir()
                        || dir.join("antigravity-2.0").is_dir()
                }) {
                    return Some("found ~/.gemini/antigravity/");
                }
                None
            }
            Self::Acp => {
                if which::which("agy-acp").is_ok() || which::which("antigravity-acp").is_ok() {
                    return Some("found agy-acp executable in PATH");
                }
                if gemini_dir().is_some_and(|dir| {
                    dir.join("antigravity-acp").is_dir() || dir.join("acp").is_dir()
                }) {
                    return Some("found ~/.gemini/antigravity-acp/");
                }
                None
            }
        }
    }

    /// ตำแหน่ง hooks.json ของ Client แต่ละตัว (หรือ fallback เข้า global hooks)
    pub fn hooks_path(&self) -> Option<PathBuf> {
        let dir = gemini_dir()?;
        let specific_path = match self {
            Self::Ide => dir
                .join("antigravity-ide")
                .join("config")
                .join("hooks.json"),
            Self::Cli => dir
                .join("antigravity-cli")
                .join("config")
                .join("hooks.json"),
            Self::Agy2 => dir.join("antigravity-2").join("config").join("hooks.json"),
            Self::Acp => dir.join("acp").join("config").join("hooks.json"),
        };

        if specific_path.exists() {
            Some(specific_path)
        } else {
            // Global Customization Root: ~/.gemini/config/hooks.json
            Some(global_hooks_path()?)
        }
    }
}

/// Unified Antigravity Agent Coordinator (จัดการรวมทั้ง 4 Clients)
pub struct Antigravity {
    target_client: Option<AntigravityClient>,
}

impl Default for Antigravity {
    fn default() -> Self {
        Self::new()
    }
}

impl Antigravity {
    /// Unified agent setup สำหรับทุก Antigravity Clients
    pub fn new() -> Self {
        Self { target_client: None }
    }

    /// เจาะจงไคลเอนต์เฉพาะ (IDE, CLI, Agy2, หรือ ACP)
    pub fn for_client(client: AntigravityClient) -> Self {
        Self { target_client: Some(client) }
    }

    /// ค้นหา Client ทั้งหมดที่ติดตั้งอยู่ในระบบปัจจุบัน
    pub fn detected_clients(&self) -> Vec<AntigravityClient> {
        AntigravityClient::all()
            .into_iter()
            .filter(|c| c.detect().is_some())
            .collect()
    }
}

impl AgentSetup for Antigravity {
    fn name(&self) -> &str {
        match self.target_client {
            Some(client) => client.identifier(),
            None => "antigravity",
        }
    }

    fn detect(&self) -> Option<&'static str> {
        if std::env::var_os("WORKMUX_TEST_AGY_DETECT").is_some() {
            return Some("test override");
        }

        if let Some(client) = self.target_client {
            return client.detect();
        }

        // หากเป็น Unified Agent: ตรวจหา Client ตัวใดตัวหนึ่งใน 4 ตัว
        for client in AntigravityClient::all() {
            if let Some(reason) = client.detect() {
                return Some(reason);
            }
        }

        // Global check: ~/.gemini/config
        if gemini_dir().is_some_and(|dir| dir.join("config").is_dir()) {
            return Some("found ~/.gemini/config/");
        }

        None
    }

    fn check(&self) -> Result<StatusCheck> {
        let path = match self.target_client {
            Some(client) => client.hooks_path(),
            None => global_hooks_path(),
        };

        let Some(path) = path else {
            return Ok(StatusCheck::NotInstalled);
        };

        check_at(&path)
    }

    fn install(&self) -> Result<String> {
        let path = match self.target_client {
            Some(client) => client.hooks_path(),
            None => global_hooks_path(),
        }
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        install_at(&path)?;
        let client_label = self
            .target_client
            .map(|c| c.display_name())
            .unwrap_or("Antigravity Ecosystem (Unified)");

        Ok(format!("Installed {} lifecycle hooks to {}", client_label, path.display()))
    }

    fn uninstall(&self) -> Result<String> {
        let path = match self.target_client {
            Some(client) => client.hooks_path(),
            None => global_hooks_path(),
        };

        let Some(path) = path else {
            return Ok("Antigravity config not found, nothing to uninstall".to_string());
        };

        uninstall_at(&path)
    }

    fn update_preview(&self) -> Result<Option<UpdatePreview>> {
        let path = match self.target_client {
            Some(client) => client.hooks_path(),
            None => global_hooks_path(),
        };

        let Some(path) = path.filter(|p| p.exists()) else {
            return Ok(None);
        };

        update_preview_at(&path)
    }
}

fn gemini_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join(".gemini"))
}

fn global_hooks_path() -> Option<PathBuf> {
    gemini_dir().map(|dir| dir.join("config").join("hooks.json"))
}

fn workmux_hooks() -> Value {
    serde_json::json!({
        WORKMUX_GROUP: {
            "PreInvocation": [{
                "type": "command",
                "command": REGISTER_COMMAND
            }, {
                "type": "command",
                "command": WORKING_COMMAND
            }],
            "PreToolUse": [{
                "matcher": ".*",
                "hooks": [{
                    "type": "command",
                    "command": WORKING_COMMAND
                }]
            }],
            "Stop": [{
                "type": "command",
                "command": STOP_COMMAND
            }]
        }
    })
}

fn check_at(path: &Path) -> Result<StatusCheck> {
    if !path.exists() {
        return Ok(StatusCheck::NotInstalled);
    }
    let config = read_json(path)?;
    if has_workmux_hooks(&config) {
        Ok(StatusCheck::Installed)
    } else if config.get(WORKMUX_GROUP).is_some() {
        Ok(StatusCheck::UpdateAvailable)
    } else {
        Ok(StatusCheck::NotInstalled)
    }
}

fn install_at(path: &Path) -> Result<String> {
    merge_hooks_file(path, &workmux_hooks())?;
    Ok(format!("Installed Antigravity lifecycle hooks to {}", path.display()))
}

fn uninstall_at(path: &Path) -> Result<String> {
    remove_workmux_group_file(path)?
        .map_or_else(|| Ok("No Antigravity workmux hooks found".to_string()), Ok)
}

fn update_preview_at(path: &Path) -> Result<Option<UpdatePreview>> {
    let installed = read_json(path)?;
    let mut bundled = installed.clone();
    let bundled_object = bundled
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("{} root is not an object", path.display()))?;
    bundled_object.insert(WORKMUX_GROUP.to_string(), workmux_hooks()[WORKMUX_GROUP].clone());
    Ok(Some(UpdatePreview {
        label: path.display().to_string(),
        installed: serde_json::to_string_pretty(&installed)? + "\n",
        bundled: serde_json::to_string_pretty(&bundled)? + "\n",
    }))
}

fn has_workmux_hooks(config: &Value) -> bool {
    let Some(group) = config.get(WORKMUX_GROUP) else {
        return false;
    };
    plain_event_has_command(group, "PreInvocation", REGISTER_COMMAND)
        && plain_event_has_command(group, "PreInvocation", WORKING_COMMAND)
        && matcher_event_has_command(group, "PreToolUse", WORKING_COMMAND)
        && plain_event_has_command(group, "Stop", STOP_COMMAND)
}

fn plain_event_has_command(group: &Value, event: &str, command: &str) -> bool {
    group
        .get(event)
        .and_then(Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.get("command").and_then(Value::as_str) == Some(command)
                    && entry.get("type").and_then(Value::as_str) == Some("command")
            })
        })
}

fn matcher_event_has_command(group: &Value, event: &str, command: &str) -> bool {
    group
        .get(event)
        .and_then(Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(Value::as_array)
                    .is_some_and(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command").and_then(Value::as_str) == Some(command)
                                && hook.get("type").and_then(Value::as_str) == Some("command")
                        })
                    })
            })
        })
}

fn merge_hooks_file(path: &Path, hooks_to_add: &Value) -> Result<()> {
    let mut config = if path.exists() {
        read_json(path)?
    } else {
        Value::Object(serde_json::Map::new())
    };
    let config_object = config
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("{} root is not an object", path.display()))?;
    let group = hooks_to_add
        .get(WORKMUX_GROUP)
        .ok_or_else(|| anyhow::anyhow!("embedded Antigravity hooks are invalid"))?;
    config_object.insert(WORKMUX_GROUP.to_string(), group.clone());
    write_json(path, &config)
}

fn remove_workmux_group_file(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut config = read_json(path)?;
    let Some(object) = config.as_object_mut() else {
        anyhow::bail!("{} root is not an object", path.display());
    };
    if object.remove(WORKMUX_GROUP).is_none() {
        return Ok(None);
    }
    if object.is_empty() {
        fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))?;
    } else {
        write_json(path, &config)?;
    }
    Ok(Some(format!("Removed Workmux hooks from {}", path.display())))
}

fn read_json(path: &Path) -> Result<Value> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("{} is not valid JSON", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")
        .with_context(|| format!("Failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn all_four_clients_have_unique_identifiers() {
        let clients = AntigravityClient::all();
        assert_eq!(clients.len(), 4);
        assert_eq!(clients[0].identifier(), "antigravity-ide");
        assert_eq!(clients[1].identifier(), "antigravity-cli");
        assert_eq!(clients[2].identifier(), "antigravity-2.0");
        assert_eq!(clients[3].identifier(), "antigravity-acp");
    }

    #[test]
    fn unified_and_specific_agent_names() {
        assert_eq!(Antigravity::new().name(), "antigravity");
        assert_eq!(Antigravity::for_client(AntigravityClient::Ide).name(), "antigravity-ide");
        assert_eq!(Antigravity::for_client(AntigravityClient::Cli).name(), "antigravity-cli");
        assert_eq!(Antigravity::for_client(AntigravityClient::Agy2).name(), "antigravity-2.0");
        assert_eq!(Antigravity::for_client(AntigravityClient::Acp).name(), "antigravity-acp");
    }

    #[test]
    fn hook_schema_valid() {
        let hooks = workmux_hooks();
        let group = &hooks[WORKMUX_GROUP];

        assert_eq!(group["PreInvocation"][0]["type"], "command");
        assert_eq!(group["PreInvocation"][0]["command"], REGISTER_COMMAND);
        assert_eq!(group["PreToolUse"][0]["hooks"][0]["command"], WORKING_COMMAND);
        assert_eq!(group["Stop"][0]["command"], STOP_COMMAND);
        assert!(group.get("PostToolUse").is_none());
    }

    #[test]
    fn install_writes_hooks_and_preserves_other_groups() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config/hooks.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "user-hooks": {
                    "Stop": [{ "type": "command", "command": "echo done" }]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        install_at(&path).unwrap();
        install_at(&path).unwrap(); // idempotent

        let config = read_json(&path).unwrap();
        assert!(has_workmux_hooks(&config));
        assert!(config.get("user-hooks").is_some());
        assert!(matches!(check_at(&path).unwrap(), StatusCheck::Installed));
    }

    #[test]
    fn check_detects_missing_registration() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("hooks.json");
        write_json(
            &path,
            &json!({
                WORKMUX_GROUP: {
                    "PreInvocation": [{
                        "type": "command",
                        "command": WORKING_COMMAND
                    }],
                    "PreToolUse": [{
                        "matcher": ".*",
                        "hooks": [{ "type": "command", "command": WORKING_COMMAND }]
                    }],
                    "Stop": [{ "type": "command", "command": STOP_COMMAND }]
                }
            }),
        )
        .unwrap();

        assert!(matches!(check_at(&path).unwrap(), StatusCheck::UpdateAvailable));
    }

    #[test]
    fn uninstall_removes_only_workmux_group() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config/hooks.json");
        install_at(&path).unwrap();
        let mut config = read_json(&path).unwrap();
        config["user-hooks"] = json!({
            "Stop": [{ "type": "command", "command": "echo done" }]
        });
        write_json(&path, &config).unwrap();

        uninstall_at(&path).unwrap();

        let config = read_json(&path).unwrap();
        assert!(config.get(WORKMUX_GROUP).is_none());
        assert!(config.get("user-hooks").is_some());
    }

    #[test]
    fn uninstall_removes_empty_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config/hooks.json");
        install_at(&path).unwrap();

        uninstall_at(&path).unwrap();

        assert!(!path.exists());
    }
}
