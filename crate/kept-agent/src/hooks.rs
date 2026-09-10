//! Shared hook checking and merging utilities.
//!
//! Provides functions to check if required hook commands exist in a JSON
//! settings object, and to merge missing hook commands into settings.

use anyhow::Result;
use serde_json::Value;

/// Check if all required hook commands exist in the settings.
///
/// Each required hook is an entry with command inside `hooks` array.
/// Returns true if every command has a matching entry.
pub fn has_required_hook_commands(settings: &Value, required: &Value) -> bool {
    let Some(required_obj) = required.as_object() else {
        return false;
    };
    let Some(settings_hooks) = settings.get("hooks").and_then(|v| v.as_object()) else {
        return false;
    };

    required_obj.iter().all(|(event, required_entries)| {
        let Some(settings_entries) = settings_hooks.get(event).and_then(|v| v.as_array()) else {
            return false;
        };
        required_entries
            .as_array()
            .map(|req_arr| {
                req_arr.iter().all(|req_entry| {
                    let cmd = req_entry.get("command").or_else(|| {
                        req_entry["hooks"]
                            .as_array()
                            .and_then(|h| h.first())
                            .and_then(|h| h.get("command"))
                    });
                    settings_entries
                        .iter()
                        .any(|se| entry_contains_command(se, cmd.unwrap_or(&Value::Null)))
                })
            })
            .unwrap_or(false)
    })
}

/// Check if settings contain any workmux hooks (partial or complete).
pub fn has_workmux_hooks(settings: &Value) -> bool {
    let Some(hooks) = settings.get("hooks").and_then(|v| v.as_object()) else {
        return false;
    };
    hooks.values().any(|entries| {
        entries
            .as_array()
            .map(|arr| {
                arr.iter().any(|entry| {
                    entry["hooks"]
                        .as_array()
                        .map(|hooks| {
                            hooks.iter().any(|hook| {
                                hook["command"]
                                    .as_str()
                                    .map(|cmd| cmd.contains("workmux"))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    })
}

/// Merge missing hook commands from `required` into `settings`.
///
/// For each event in `required`, adds entries that don't already exist
/// in `settings["hooks"][event]`. Preserves existing user hooks.
pub fn merge_missing_hook_commands(settings: &mut Value, required: &Value) -> Result<()> {
    let Some(required_obj) = required.as_object() else {
        return Ok(());
    };

    // Ensure root is object and hooks key exists
    {
        let settings_obj = settings
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("settings root must be an object"))?;
        if !settings_obj.contains_key("hooks") || !settings_obj["hooks"].is_object() {
            settings_obj.insert("hooks".to_string(), Value::Object(serde_json::Map::new()));
        }
    }

    // Collect events that need new entries
    let mut to_add: Vec<(String, Vec<Value>)> = Vec::new();

    for (event, required_entries) in required_obj {
        let Some(req_arr) = required_entries.as_array() else {
            continue;
        };

        let existing = settings["hooks"]
            .get(event)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut new_entries = Vec::new();
        for req_entry in req_arr {
            let req_cmd = req_entry
                .get("command")
                .or_else(|| {
                    req_entry["hooks"]
                        .as_array()
                        .and_then(|h| h.first())
                        .and_then(|h| h.get("command"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let is_dup = existing
                .iter()
                .any(|se| entry_contains_command(se, &Value::String(req_cmd.to_string())));
            if !req_cmd.is_empty() && !is_dup {
                new_entries.push(req_entry.clone());
            }
        }
        if !new_entries.is_empty() {
            to_add.push((event.clone(), new_entries));
        }
    }

    // Apply collected entries
    for (event, entries) in to_add {
        let settings_obj = settings
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("settings root must be an object"))?;
        let hooks_val = settings_obj
            .get_mut("hooks")
            .ok_or_else(|| anyhow::anyhow!("hooks key missing"))?;
        let hooks_obj = hooks_val
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("hooks must be an object"))?;
        let arr = hooks_obj
            .entry(event.clone())
            .or_insert_with(|| Value::Array(vec![]))
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("hooks entry must be array"))?;
        arr.extend(entries);
    }

    Ok(())
}

/// Check if an entry contains a specific command.
fn entry_contains_command(entry: &Value, command: &Value) -> bool {
    // Direct command on entry
    if entry.get("command") == Some(command) {
        return true;
    }
    // Nested hooks array
    entry["hooks"]
        .as_array()
        .map(|hooks| hooks.iter().any(|h| h.get("command") == Some(command)))
        .unwrap_or(false)
}

/// Load hooks from an embedded JSON string.
pub fn hooks_from_embedded(json_str: &str, error_msg: &str) -> Result<Value> {
    let value: Value =
        serde_json::from_str(json_str).map_err(|e| anyhow::anyhow!("{}: {}", error_msg, e))?;
    value
        .get("hooks")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{}", error_msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn has_required_empty_on_empty_settings() {
        let settings = json!({});
        let required = json!({
            "Stop": [{"command": "workmux set-window-status done"}]
        });
        assert!(!has_required_hook_commands(&settings, &required));
    }

    #[test]
    fn has_required_matches_direct_command() {
        let settings = json!({
            "hooks": {
                "Stop": [{"command": "workmux set-window-status done"}]
            }
        });
        let required = json!({
            "Stop": [{"command": "workmux set-window-status done"}]
        });
        assert!(has_required_hook_commands(&settings, &required));
    }

    #[test]
    fn has_required_matches_nested_hook() {
        let settings = json!({
            "hooks": {
                "Stop": [{"hooks": [{"command": "workmux set-window-status done"}]}]
            }
        });
        let required = json!({
            "Stop": [{"command": "workmux set-window-status done"}]
        });
        assert!(has_required_hook_commands(&settings, &required));
    }

    #[test]
    fn has_workmux_hooks_detects_workmux_command() {
        let settings = json!({
            "hooks": {
                "Stop": [{"hooks": [{"command": "workmux set-window-status done"}]}]
            }
        });
        assert!(has_workmux_hooks(&settings));
    }

    #[test]
    fn has_workmux_hooks_returns_false_when_empty() {
        let settings = json!({});
        assert!(!has_workmux_hooks(&settings));
    }

    #[test]
    fn merge_adds_missing_commands() {
        let mut settings = json!({
            "hooks": {
                "Stop": [{"command": "afplay glass.aiff"}]
            }
        });
        let required = json!({
            "Stop": [{"command": "workmux set-window-status done"}]
        });
        merge_missing_hook_commands(&mut settings, &required).unwrap();
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2);
    }

    #[test]
    fn merge_does_not_duplicate_existing() {
        let mut settings = json!({
            "hooks": {
                "Stop": [{"command": "workmux set-window-status done"}]
            }
        });
        let required = json!({
            "Stop": [{"command": "workmux set-window-status done"}]
        });
        merge_missing_hook_commands(&mut settings, &required).unwrap();
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 1);
    }

    #[test]
    fn hooks_from_embedded_parses_valid_json() {
        let json_str = r#"{"hooks": {"Stop": [{"command": "test"}]}}"#;
        let hooks = hooks_from_embedded(json_str, "failed").unwrap();
        assert!(hooks.get("Stop").is_some());
    }

    #[test]
    fn hooks_from_embedded_fails_on_missing_hooks_key() {
        let json_str = r#"{"noHooks": true}"#;
        assert!(hooks_from_embedded(json_str, "missing hooks").is_err());
    }
}
