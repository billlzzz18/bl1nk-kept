# kept-agent Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `kept-agent` crate — agent lifecycle management for external CLI agents (Antigravity/agy, future agents)

**Architecture:** New crate `kept-agent` with `StatusCheck`/`UpdatePreview` types, an `Agent` trait for lifecycle operations, and one concrete implementation (Antigravity). kept-cli depends on kept-agent and exposes `kept agent` subcommand.

**Tech Stack:** Rust, anyhow, serde_json, which, home

**Spec:** User-provided Antigravity integration code + workspace structure analysis

## Global Constraints

- `unwrap_used` / `expect_used` = warn (workspace lints)
- Error handling: `anyhow` for application layers
- Edition 2021, version from workspace
- Follow existing crate patterns (kept-grammar, kept-doc)
- ห้ามสร้าง `mod.rs` — use Rust modern path convention
- Comment style: `// NOTE-001:` for rationale (Thai), `///` for public rustdoc (English)

---

## File Structure

```
crate/kept-agent/
├── Cargo.toml
└── src/
    ├── lib.rs                    ← re-exports, Agent trait
    ├── status.rs                 ← StatusCheck, UpdatePreview types
    ├── detect.rs                 ← agent detection utilities
    └── antigravity.rs            ← agy integration (user's code, cleaned up)

crate/kept-cli/
├── Cargo.toml                    ← add kept-agent dependency
└── src/
    ├── main.rs                   ← add Agent subcommand
    └── commands/
        └── agent.rs              ← handle_agent(), dispatch to kept-agent
```

---

### Task 1: Scaffold kept-agent crate

**Files:**
- Create: `crate/kept-agent/Cargo.toml`
- Create: `crate/kept-agent/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces:**
- Consumes: workspace dependency patterns
- Produces: empty crate that compiles

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "kept-agent"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "Agent lifecycle management for external CLI integrations"

[dependencies]
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create lib.rs**

```rust
//! Agent lifecycle management for external CLI integrations.

mod status;
mod detect;
mod antigravity;

pub use status::*;
pub use detect::*;
pub use antigravity as agy;
```

- [ ] **Step 3: Add to workspace members**

In root `Cargo.toml`, add `"crate/kept-agent"` to `[workspace] members`.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p kept-agent`
Expected: PASS (empty crate, no errors)

- [ ] **Step 5: Commit**

```bash
git add crate/kept-agent/ Cargo.toml
git commit -m "feat(agent): scaffold kept-agent crate

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 2: Define StatusCheck and UpdatePreview types

**Files:**
- Create: `crate/kept-agent/src/status.rs`

**Interfaces:**
- Consumes: none (leaf types)
- Produces: `StatusCheck`, `UpdatePreview` used by all agent implementations

- [ ] **Step 1: Write failing test**

Create `crate/kept-agent/src/status.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_check_variants_exist() {
        let installed = StatusCheck::Installed;
        let not_installed = StatusCheck::NotInstalled;
        let update = StatusCheck::UpdateAvailable;

        // Ensure all variants are constructible
        assert!(matches!(installed, StatusCheck::Installed));
        assert!(matches!(not_installed, StatusCheck::NotInstalled));
        assert!(matches!(update, StatusCheck::UpdateAvailable));
    }

    #[test]
    fn update_preview_holds_display_strings() {
        let preview = UpdatePreview {
            label: "test.json".to_string(),
            installed: "{}\n".to_string(),
            bundled: "{\"a\":1}\n".to_string(),
        };
        assert_eq!(preview.label, "test.json");
        assert!(preview.installed.contains('{}'));
        assert!(preview.bundled.contains("a"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-agent -- status`
Expected: FAIL — module `status` does not exist

- [ ] **Step 3: Write implementation**

```rust
use serde::{Deserialize, Serialize};

/// Status of an agent integration's lifecycle hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusCheck {
    /// Hooks are installed and up to date.
    Installed,
    /// Hooks are not installed.
    NotInstalled,
    /// Hooks exist but a newer version is available.
    UpdateAvailable,
}

/// Preview of what an update would change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreview {
    /// Human-readable label (e.g., file path).
    pub label: String,
    /// Current installed config as string.
    pub installed: String,
    /// Bundled (latest) config as string.
    pub bundled: String,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-agent -- status`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crate/kept-agent/src/status.rs
git commit -m "feat(agent): add StatusCheck and UpdatePreview types

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 3: Create Agent trait and detection utilities

**Files:**
- Create: `crate/kept-agent/src/detect.rs`

**Interfaces:**
- Consumes: `StatusCheck`, `UpdatePreview` from Task 2
- Produces: `AgentSetup` trait, `detect_agents()` function

- [ ] **Step 1: Write failing test**

Append to `crate/kept-agent/src/detect.rs`:

```rust
use crate::status::{StatusCheck, UpdatePreview};
use anyhow::Result;

/// Trait for agent lifecycle management.
///
/// Each external CLI agent (agy, etc.) implements this trait
/// to integrate with kept's agent management.
pub trait AgentSetup {
    /// Human-readable agent name.
    fn name(&self) -> &str;

    /// Detect if this agent is present on the system.
    fn detect(&self) -> Option<&'static str>;

    /// Check if kept's lifecycle hooks are installed for this agent.
    fn check(&self) -> Result<StatusCheck>;

    /// Install lifecycle hooks for this agent.
    fn install(&self) -> Result<String>;

    /// Remove lifecycle hooks for this agent.
    fn uninstall(&self) -> Result<String>;

    /// Preview what an update would change, if any.
    fn update_preview(&self) -> Result<Option<UpdatePreview>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAgent;

    impl AgentSetup for MockAgent {
        fn name(&self) -> &str { "mock-agent" }
        fn detect(&self) -> Option<&'static str> { Some("test override") }
        fn check(&self) -> Result<StatusCheck> { Ok(StatusCheck::Installed) }
        fn install(&self) -> Result<String> { Ok("installed".to_string()) }
        fn uninstall(&self) -> Result<String> { Ok("uninstalled".to_string()) }
        fn update_preview(&self) -> Result<Option<UpdatePreview>> { Ok(None) }
    }

    #[test]
    fn mock_agent_satisfies_trait() {
        let agent = MockAgent;
        assert_eq!(agent.name(), "mock-agent");
        assert!(agent.detect().is_some());
        assert!(matches!(agent.check().unwrap(), StatusCheck::Installed));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-agent -- detect`
Expected: FAIL — module `detect` does not exist in lib.rs yet (it's declared but empty)

- [ ] **Step 3: Write implementation**

```rust
use crate::status::{StatusCheck, UpdatePreview};
use anyhow::Result;

/// Trait for agent lifecycle management.
///
/// Each external CLI agent (agy, etc.) implements this trait
/// to integrate with kept's agent management.
pub trait AgentSetup {
    /// Human-readable agent name.
    fn name(&self) -> &str;

    /// Detect if this agent is present on the system.
    ///
    /// Returns `Some(reason)` if detected, `None` otherwise.
    fn detect(&self) -> Option<&'static str>;

    /// Check if kept's lifecycle hooks are installed for this agent.
    fn check(&self) -> Result<StatusCheck>;

    /// Install lifecycle hooks for this agent.
    fn install(&self) -> Result<String>;

    /// Remove lifecycle hooks for this agent.
    fn uninstall(&self) -> Result<String>;

    /// Preview what an update would change, if any.
    fn update_preview(&self) -> Result<Option<UpdatePreview>>;
}

/// Return all registered agent implementations.
pub fn registered_agents() -> Vec<Box<dyn AgentSetup>> {
    vec![Box::new(crate::antigravity::Antigravity::new())]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAgent;

    impl AgentSetup for MockAgent {
        fn name(&self) -> &str { "mock-agent" }
        fn detect(&self) -> Option<&'static str> { Some("test override") }
        fn check(&self) -> Result<StatusCheck> { Ok(StatusCheck::Installed) }
        fn install(&self) -> Result<String> { Ok("installed".to_string()) }
        fn uninstall(&self) -> Result<String> { Ok("uninstalled".to_string()) }
        fn update_preview(&self) -> Result<Option<UpdatePreview>> { Ok(None) }
    }

    #[test]
    fn mock_agent_satisfies_trait() {
        let agent = MockAgent;
        assert_eq!(agent.name(), "mock-agent");
        assert!(agent.detect().is_some());
        assert!(matches!(agent.check().unwrap(), StatusCheck::Installed));
    }

    #[test]
    fn registered_agents_not_empty() {
        let agents = registered_agents();
        assert!(!agents.is_empty());
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-agent -- detect`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crate/kept-agent/src/detect.rs
git commit -m "feat(agent): add AgentSetup trait and detection utilities

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 4: Implement Antigravity integration

**Files:**
- Create: `crate/kept-agent/src/antigravity.rs`

**Interfaces:**
- Consumes: `AgentSetup` trait, `StatusCheck`, `UpdatePreview` from Tasks 2-3
- Produces: `Antigravity` struct implementing `AgentSetup`

**Dependencies to add to Cargo.toml:**
- `which = "6"` (workspace or direct)
- `home = "0.5"` (workspace or direct)

- [ ] **Step 1: Add dependencies to kept-agent/Cargo.toml**

```toml
[dependencies]
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
which = "6"
home = "0.5"
```

- [ ] **Step 2: Write failing test**

Create `crate/kept-agent/src/antigravity.rs`:

```rust
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

pub struct Antigravity;

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
        detection_reason(
            std::env::var_os("WORKMUX_TEST_AGY_DETECT").is_some(),
            std::env::var_os("WORKMUX_TEST").is_some(),
            which::which("agy").is_ok(),
            gemini_dir().is_some_and(|dir| dir.join("antigravity-cli").is_dir()),
        )
    }

    fn check(&self) -> Result<StatusCheck> {
        let Some(path) = hooks_path() else {
            return Ok(StatusCheck::NotInstalled);
        };
        check_at(&path)
    }

    fn install(&self) -> Result<String> {
        let path = hooks_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        install_at(&path)
    }

    fn uninstall(&self) -> Result<String> {
        let Some(path) = hooks_path() else {
            return Ok("Antigravity config dir not found, nothing to uninstall".to_string());
        };
        uninstall_at(&path)
    }

    fn update_preview(&self) -> Result<Option<UpdatePreview>> {
        let Some(path) = hooks_path().filter(|path| path.exists()) else {
            return Ok(None);
        };
        update_preview_at(&path)
    }
}

// NOTE-001: detection logic แยก test env จาก production —
// test override ต้องมาก่อนเพื่อให้ CI ควบคุมผลลัพธ์ได้
fn detection_reason(
    explicit_test_detection: bool,
    test_environment: bool,
    executable_found: bool,
    config_found: bool,
) -> Option<&'static str> {
    if explicit_test_detection {
        Some("test override")
    } else if !test_environment && executable_found {
        Some("found agy executable")
    } else if config_found {
        Some("found ~/.gemini/antigravity-cli/")
    } else {
        None
    }
}

fn gemini_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join(".gemini"))
}

fn hooks_path() -> Option<PathBuf> {
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
    Ok(format!(
        "Installed Antigravity lifecycle hooks to {}",
        path.display()
    ))
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
    bundled_object.insert(
        WORKMUX_GROUP.to_string(),
        workmux_hooks()[WORKMUX_GROUP].clone(),
    );
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
        fs::remove_file(path)
            .with_context(|| format!("Failed to remove {}", path.display()))?;
    } else {
        write_json(path, &config)?;
    }
    Ok(Some(format!(
        "Removed Workmux hooks from {}",
        path.display()
    )))
}

fn read_json(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
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
    fn detection_skips_executable_in_test_env() {
        assert_eq!(detection_reason(false, true, true, false), None);
        assert_eq!(
            detection_reason(true, true, true, false),
            Some("test override")
        );
        assert_eq!(
            detection_reason(false, true, true, true),
            Some("found ~/.gemini/antigravity-cli/")
        );
        assert_eq!(
            detection_reason(false, false, true, false),
            Some("found agy executable")
        );
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

        assert!(matches!(
            check_at(&path).unwrap(),
            StatusCheck::UpdateAvailable
        ));
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
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test -p kept-agent`
Expected: PASS — all tests green

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p kept-agent -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crate/kept-agent/src/antigravity.rs crate/kept-agent/Cargo.toml
git commit -m "feat(agent): implement Antigravity CLI integration

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 5: Add `kept agent` subcommand to CLI

**Files:**
- Modify: `crate/kept-cli/Cargo.toml` — add `kept-agent` dependency
- Create: `crate/kept-cli/src/commands/agent.rs` — handle_agent()
- Modify: `crate/kept-cli/src/commands.rs` — declare `agent` module
- Modify: `crate/kept-cli/src/main.rs` — add Agent variant + dispatch

**Interfaces:**
- Consumes: `kept_agent::AgentSetup`, `kept_agent::StatusCheck`, `kept_agent::registered_agents()`
- Produces: `kept agent` CLI subcommand

- [ ] **Step 1: Add dependency to kept-cli/Cargo.toml**

```toml
kept-agent = { path = "../kept-agent" }
```

- [ ] **Step 2: Create commands/agent.rs**

```rust
use anyhow::Result;
use kept_agent::StatusCheck;

pub fn handle_agent(action: Option<AgentAction>) -> Result<()> {
    let agents = kept_agent::registered_agents();

    match action {
        None | Some(AgentAction::Status) => {
            for agent in &agents {
                let detected = agent.detect().map(|r| r.to_string()).unwrap_or_else(|| "not found".to_string());
                let status = agent.check()?;
                let status_str = match status {
                    StatusCheck::Installed => "installed",
                    StatusCheck::NotInstalled => "not installed",
                    StatusCheck::UpdateAvailable => "update available",
                };
                println!("{:<20} detected: {:<30} hooks: {}", agent.name(), detected, status_str);
            }
            Ok(())
        }
        Some(AgentAction::Install { name }) => {
            let agent = find_agent(&agents, &name)?;
            println!("{}", agent.install()?);
            Ok(())
        }
        Some(AgentAction::Uninstall { name }) => {
            let agent = find_agent(&agents, &name)?;
            println!("{}", agent.uninstall()?);
            Ok(())
        }
        Some(AgentAction::Check { name }) => {
            let agent = find_agent(&agents, &name)?;
            match agent.check()? {
                StatusCheck::Installed => println!("{}: installed", agent.name()),
                StatusCheck::NotInstalled => println!("{}: not installed", agent.name()),
                StatusCheck::UpdateAvailable => println!("{}: update available", agent.name()),
            }
            Ok(())
        }
    }
}

fn find_agent<'a>(agents: &'a [Box<dyn kept_agent::AgentSetup>], name: &str) -> Result<&'a Box<dyn kept_agent::AgentSetup>> {
    agents
        .iter()
        .find(|a| a.name() == name)
        .ok_or_else(|| anyhow::anyhow!("Unknown agent: {}", name))
}

#[derive(Debug, Clone)]
pub enum AgentAction {
    Status,
    Install { name: String },
    Uninstall { name: String },
    Check { name: String },
}
```

- [ ] **Step 3: Register module in commands.rs**

Add to `crate/kept-cli/src/commands.rs`:

```rust
pub mod agent;
```

- [ ] **Step 4: Add Agent variant to Commands enum in main.rs**

After the `Mcp` variant in `Commands`:

```rust
/// Manage external agent integrations.
Agent {
    #[command(subcommand)]
    action: Option<agent::AgentSubcommand>,
},
```

- [ ] **Step 5: Add dispatch in main()**

In the `match cli.command` block:

```rust
Commands::Agent { action } => agent::handle_agent(action)?,
```

- [ ] **Step 6: Run test**

Run: `cargo test -p kept-cli`
Expected: PASS

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -p kept-cli -- -D warnings`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crate/kept-cli/
git commit -m "feat(cli): add 'kept agent' subcommand for agent lifecycle management

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 6: Full workspace verification

**Files:** none (verification only)

- [ ] **Step 1: Build entire workspace**

Run: `cargo build --workspace`
Expected: PASS

- [ ] **Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: PASS

- [ ] **Step 3: Run clippy on workspace**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

- [ ] **Step 4: Check formatting**

Run: `cargo fmt --all -- --check`
Expected: PASS

- [ ] **Step 5: Final commit if needed**

```bash
git add -A
git commit -m "chore: workspace verification for kept-agent

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```
