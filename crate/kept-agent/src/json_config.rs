//! Shared JSON config file read/write and hook install/uninstall utilities.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

/// What to use as the root when the config file is empty or missing.
#[derive(Debug, Clone, Copy)]
pub enum EmptyJsonRoot {
    /// Use an empty JSON object `{}`.
    Object,
}

/// Messages for hook installation context.
#[derive(Debug, Clone)]
pub struct JsonHookInstallSpec {
    pub read_context: &'static str,
    pub parse_context: &'static str,
    pub write_context: &'static str,
    pub mkdir_context: &'static str,
    pub empty_root: EmptyJsonRoot,
}

/// Messages for hook uninstallation context.
#[derive(Debug, Clone)]
pub struct JsonHookUninstallSpec {
    pub messages: JsonHookUninstallMessages,
    pub delete_if_no_hooks_remain: bool,
    pub remove_plugins: bool,
    pub soft_errors: bool,
}

/// Error messages for uninstall operations.
#[derive(Debug, Clone)]
pub struct JsonHookUninstallMessages {
    pub file_missing: &'static str,
    pub not_found: &'static str,
    pub soft_read_error: Option<&'static str>,
    pub soft_parse_error: Option<&'static str>,
}

/// Install hooks into a JSON config file.
///
/// Reads the file (or creates empty), merges hooks, writes back.
pub fn json_hook_install_with(
    path: &Path,
    hooks: &Value,
    spec: &JsonHookInstallSpec,
    merge_fn: fn(&mut Value, &Value) -> Result<()>,
) -> Result<String> {
    let mut config = if path.exists() {
        read_json_file(path, spec.read_context, spec.parse_context)?
    } else {
        match spec.empty_root {
            EmptyJsonRoot::Object => Value::Object(serde_json::Map::new()),
        }
    };

    merge_fn(&mut config, hooks)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| spec.mkdir_context.to_string())?;
    }

    write_json_file(path, &config, spec.write_context)?;

    Ok(format!("Installed hooks to {}", path.display()))
}

/// Uninstall hooks from a JSON config file.
///
/// Surgically removes only hook entries matching the workmux pattern,
/// preserving user-configured hooks.
pub fn json_hook_uninstall(path: &Path, spec: &JsonHookUninstallSpec) -> Result<String> {
    if !path.exists() {
        return Ok(spec.messages.file_missing.to_string());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let mut config: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            if spec.soft_errors {
                return Ok(spec
                    .messages
                    .soft_parse_error
                    .unwrap_or("parse error")
                    .to_string());
            }
            return Err(e).with_context(|| {
                spec.messages
                    .soft_read_error
                    .unwrap_or("parse error")
                    .to_string()
            });
        },
    };

    let had_hooks = remove_workmux_hooks(&mut config, spec.remove_plugins);

    if !had_hooks {
        return Ok(spec.messages.not_found.to_string());
    }

    // Check if any hooks remain
    let hooks_remain = config
        .get("hooks")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.values()
                .any(|v| v.as_array().is_some_and(|a| !a.is_empty()))
        })
        .unwrap_or(false);

    if spec.delete_if_no_hooks_remain && !hooks_remain {
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to remove {}", path.display()))?;
        return Ok(format!("Removed {} (no hooks remain)", path.display()));
    }

    write_json_file(path, &config, "Failed to write config")?;

    Ok(format!("Removed workmux hooks from {}", path.display()))
}

/// Remove workmux-related hooks from a config value.
///
/// Returns true if any workmux hooks were found and removed.
fn remove_workmux_hooks(config: &mut Value, remove_plugins: bool) -> bool {
    let mut found = false;

    // Remove from hooks object
    if let Some(hooks) = config.get_mut("hooks").and_then(|v| v.as_object_mut()) {
        for (_event, entries) in hooks.iter_mut() {
            if let Some(arr) = entries.as_array_mut() {
                let before = arr.len();
                arr.retain(|entry| !is_workmux_entry(entry));
                if arr.len() < before {
                    found = true;
                }
            }
        }
        // Clean up empty event arrays
        hooks.retain(|_, v| v.as_array().is_some_and(|a| !a.is_empty()));
    }

    // Remove from enabledPlugins
    if remove_plugins {
        if let Some(plugins) = config
            .get_mut("enabledPlugins")
            .and_then(|v| v.as_object_mut())
        {
            let before = plugins.len();
            plugins.retain(|key, _| !key.starts_with("workmux-status@"));
            if plugins.len() < before {
                found = true;
            }
        }
    }

    found
}

/// Check if a hook entry is workmux-related.
fn is_workmux_entry(entry: &Value) -> bool {
    // Check direct command
    if let Some(cmd) = entry.get("command").and_then(|v| v.as_str()) {
        if cmd.contains("workmux") {
            return true;
        }
    }
    // Check nested hooks
    entry["hooks"]
        .as_array()
        .map(|hooks| {
            hooks.iter().any(|h| {
                h.get("command")
                    .and_then(|v| v.as_str())
                    .map(|cmd| cmd.contains("workmux"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn read_json_file(path: &Path, read_ctx: &str, parse_ctx: &str) -> Result<Value> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("{}: {}", read_ctx, path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("{}: {}", parse_ctx, path.display()))
}

fn write_json_file(path: &Path, value: &Value, write_ctx: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)? + "\n")
        .with_context(|| format!("{}: {}", write_ctx, path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn install_creates_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        let hooks = json!({"Stop": [{"command": "test"}]});
        let spec = JsonHookInstallSpec {
            read_context: "read",
            parse_context: "parse",
            write_context: "write",
            mkdir_context: "mkdir",
            empty_root: EmptyJsonRoot::Object,
        };
        json_hook_install_with(&path, &hooks, &spec, crate::hooks::merge_missing_hook_commands)
            .unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("test"));
    }

    #[test]
    fn uninstall_removes_workmux_hooks() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"hooks":{"Stop":[{"hooks":[{"command":"workmux done"}]},{"command":"user hook"}]}}"#,
        )
        .unwrap();
        let spec = JsonHookUninstallSpec {
            messages: JsonHookUninstallMessages {
                file_missing: "missing",
                not_found: "not found",
                soft_read_error: None,
                soft_parse_error: None,
            },
            delete_if_no_hooks_remain: false,
            remove_plugins: false,
            soft_errors: false,
        };
        let result = json_hook_uninstall(&path, &spec).unwrap();
        assert!(result.contains("Removed workmux"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("user hook"));
        assert!(!content.contains("workmux"));
    }

    #[test]
    fn uninstall_returns_not_found_when_no_hooks() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"setting": true}"#).unwrap();
        let spec = JsonHookUninstallSpec {
            messages: JsonHookUninstallMessages {
                file_missing: "missing",
                not_found: "not found",
                soft_read_error: None,
                soft_parse_error: None,
            },
            delete_if_no_hooks_remain: false,
            remove_plugins: false,
            soft_errors: false,
        };
        let result = json_hook_uninstall(&path, &spec).unwrap();
        assert!(result.contains("not found"));
    }

    #[test]
    fn uninstall_returns_file_missing_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let spec = JsonHookUninstallSpec {
            messages: JsonHookUninstallMessages {
                file_missing: "file missing",
                not_found: "not found",
                soft_read_error: None,
                soft_parse_error: None,
            },
            delete_if_no_hooks_remain: false,
            remove_plugins: false,
            soft_errors: false,
        };
        let result = json_hook_uninstall(&path, &spec).unwrap();
        assert!(result.contains("file missing"));
    }
}
