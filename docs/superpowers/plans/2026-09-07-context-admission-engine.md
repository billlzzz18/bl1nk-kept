# Context Admission & Judge Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement session-scoped Observation model, ContextRegistry, and Judge admission pipeline to eliminate duplicate context token waste and prevent agent tool loops.

**Architecture:** Model resources as typed `Observation` with canonical `Target` URIs. Track seen revisions and view counts in `ContextRegistry`. Run admission treatments (`Pass`, `Reference`, `Delta`, `Compress`, `Warn`, `Block`) via `Judge` engine before streaming context to agents.

**Tech Stack:** Rust (edition 2021), `kept-core`, `kept-mcp`, `pmcp`, `serde`, `serde_json`, `tokio`.

**Spec:** `docs/specs/context_admission_engine.md`

## Global Constraints

- Local-first execution with deterministic serialization.
- Zero external LLM inference dependencies inside core admission logic.
- Reference pointers capped at 13 tokens (`context://<target>@<hash>`).
- Strict warning threshold at `view_count >= 3` for unchanged targets.
- All code formatted via `rustfmt` and passing `cargo clippy --workspace --all-targets -- -D warnings` and `just check`.

---

### Task 1: Observation Data Model & Canonical Target URI Parser

**Files:**
- Create: `crate/kept-core/src/observation/target.rs`
- Create: `crate/kept-core/src/observation/identity.rs`
- Create: `crate/kept-core/src/observation/types.rs`
- Create: `crate/kept-core/src/observation/mod.rs`
- Test: `crate/kept-core/tests/observation_contract.rs`

**Interfaces:**
- Produces:
  - `Target` enum with `parse(uri: &str) -> Result<Target, ParseError>`
  - `Revision` struct with `timestamp: u64` and `token: Option<String>`
  - `ContentIdentity` struct with `from_bytes(data: &[u8]) -> ContentIdentity`
  - `Observation` struct with deterministic serialization

- [ ] **Step 1: Write the failing contract test**

```rust
// crate/kept-core/tests/observation_contract.rs
use kept_core::observation::{ContentIdentity, Observation, Revision, Source, SourceKind, Target};

#[test]
fn test_target_uri_roundtrip_and_identity() {
    let file_target = Target::parse("file://src/main.rs").expect("parse file uri");
    assert_eq!(file_target.to_string(), "file://src/main.rs");

    let bytes = b"hello world";
    let identity = ContentIdentity::from_bytes(bytes);
    assert_eq!(identity.len, 11);
    assert!(!identity.hash.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-core --test observation_contract`
Expected: FAIL (types / modules not resolved)

- [ ] **Step 3: Implement minimal Target and Observation types**

```rust
// crate/kept-core/src/observation/target.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    File(String),
    Symbol(String),
    Search(String),
    Context(String),
}

impl Target {
    pub fn parse(uri: &str) -> Result<Self, String> {
        if let Some(path) = uri.strip_prefix("file://") {
            Ok(Target::File(path.to_string()))
        } else if let Some(sym) = uri.strip_prefix("symbol://") {
            Ok(Target::Symbol(sym.to_string()))
        } else if let Some(q) = uri.strip_prefix("search://") {
            Ok(Target::Search(q.to_string()))
        } else if let Some(c) = uri.strip_prefix("context://") {
            Ok(Target::Context(c.to_string()))
        } else {
            Err(format!("Unknown scheme: {uri}"))
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-core --test observation_contract`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add crate/kept-core/src/observation crate/kept-core/tests/observation_contract.rs
git commit -m "feat(core): implement Target URI parser and Observation data model"
```

---

### Task 2: Context Registry State Tracker

**Files:**
- Create: `crate/kept-core/src/context/registry.rs`
- Create: `crate/kept-core/src/context/mod.rs`
- Test: `crate/kept-core/tests/context_contract.rs`

**Interfaces:**
- Consumes: `Observation`, `Target` from `kept_core::observation`
- Produces: `ContextRegistry`, `SeenEntry`

- [ ] **Step 1: Write the failing unit test**

```rust
// crate/kept-core/tests/context_contract.rs
use kept_core::context::registry::ContextRegistry;
use kept_core::observation::{ContentIdentity, Observation, Revision, Source, SourceKind, Target};

#[test]
fn test_context_registry_records_views() {
    let registry = ContextRegistry::new();
    let target = Target::File("src/lib.rs".to_string());
    let obs = Observation::dummy_for_target(target.clone());

    let entry = registry.register(&obs);
    assert_eq!(entry.view_count, 1);

    let entry2 = registry.register(&obs);
    assert_eq!(entry2.view_count, 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-core --test context_contract`
Expected: FAIL (ContextRegistry not found)

- [ ] **Step 3: Implement ContextRegistry**

```rust
// crate/kept-core/src/context/registry.rs
use std::collections::HashMap;
use std::sync::RwLock;
use crate::observation::Observation;

#[derive(Debug, Clone)]
pub struct SeenEntry {
    pub target: String,
    pub content_hash: String,
    pub view_count: usize,
    pub last_seen_unix: u64,
}

pub struct ContextRegistry {
    entries: RwLock<HashMap<String, SeenEntry>>,
}

impl ContextRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, obs: &Observation) -> SeenEntry {
        let mut map = self.entries.write().unwrap();
        let target_str = obs.target.to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = map.entry(target_str.clone()).or_insert(SeenEntry {
            target: target_str,
            content_hash: obs.source.identity.hash.clone(),
            view_count: 0,
            last_seen_unix: now,
        });

        entry.view_count += 1;
        entry.last_seen_unix = now;
        entry.clone()
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-core --test context_contract`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add crate/kept-core/src/context crate/kept-core/tests/context_contract.rs
git commit -m "feat(core): implement ContextRegistry session view tracking"
```

---

### Task 3: Judge Admission Engine & Decision Logic

**Files:**
- Create: `crate/kept-core/src/context/judge.rs`
- Modify: `crate/kept-core/src/context/mod.rs`
- Test: `crate/kept-core/tests/context_contract.rs`

**Interfaces:**
- Consumes: `ContextRegistry`, `Observation`
- Produces: `Judge`, `AdmissionDecision`

- [ ] **Step 1: Write the failing contract test for Judge decisions**

```rust
// crate/kept-core/tests/context_contract.rs
use kept_core::context::judge::{AdmissionDecision, Judge};
use kept_core::context::registry::ContextRegistry;
use kept_core::observation::Observation;

#[test]
fn test_judge_decisions_pass_reference_warn() {
    let registry = std::sync::Arc::new(ContextRegistry::new());
    let judge = Judge::new(registry);
    let obs = Observation::dummy_file("src/main.rs", b"println!(\"hello\");");

    // 1st visit -> Pass
    let d1 = judge.admit(&obs);
    assert!(matches!(d1, AdmissionDecision::Pass(_)));

    // 2nd visit -> Reference
    let d2 = judge.admit(&obs);
    assert!(matches!(d2, AdmissionDecision::Reference { .. }));

    // 3rd visit -> Reference
    let d3 = judge.admit(&obs);
    assert!(matches!(d3, AdmissionDecision::Reference { .. }));

    // 4th visit (exceeded threshold 3) -> Warn
    let d4 = judge.admit(&obs);
    assert!(matches!(d4, AdmissionDecision::Warn { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-core --test context_contract`
Expected: FAIL (`Judge` or `AdmissionDecision` missing)

- [ ] **Step 3: Implement Judge admission engine**

```rust
// crate/kept-core/src/context/judge.rs
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::context::registry::ContextRegistry;
use crate::observation::Observation;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum AdmissionDecision {
    Pass(Box<Observation>),
    Reference { target: String, hash: String, token_cost: usize },
    Delta { target: String, diff: String, token_cost: usize },
    Compress { target: String, compressed: String, ratio: f64 },
    Drop { target: String, reason: String },
    Warn { target: String, warning: String },
    Block { target: String, reason: String },
}

pub struct Judge {
    registry: Arc<ContextRegistry>,
}

impl Judge {
    pub fn new(registry: Arc<ContextRegistry>) -> Self {
        Self { registry }
    }

    pub fn admit(&self, obs: &Observation) -> AdmissionDecision {
        let entry = self.registry.register(obs);
        if entry.view_count == 1 {
            AdmissionDecision::Pass(Box::new(obs.clone()))
        } else if entry.view_count >= 4 {
            AdmissionDecision::Warn {
                target: obs.target.to_string(),
                warning: format!("Target '{}' read {} times without modification", obs.target, entry.view_count),
            }
        } else {
            AdmissionDecision::Reference {
                target: obs.target.to_string(),
                hash: entry.content_hash,
                token_cost: 13,
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-core --test context_contract`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add crate/kept-core/src/context/judge.rs crate/kept-core/tests/context_contract.rs
git commit -m "feat(core): implement Judge admission engine with Pass, Reference, Warn decisions"
```

---

### Task 4: MCP Filesystem Tools Integration with FFF Manager

**Files:**
- Create: `crate/kept-mcp/src/mcp/tools/filesystem.rs`
- Modify: `crate/kept-mcp/src/mcp/tools/mod.rs`
- Modify: `crate/kept-mcp/src/mcp/server.rs`
- Test: `crate/kept-mcp/tests/filesystem_mcp_contract.rs`

**Interfaces:**
- Consumes: `FffScanner` from `kept_core::scanner::fff`
- Produces: MCP tools (`filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, `filesystem_status`)

- [ ] **Step 1: Write the failing contract test**

```rust
// crate/kept-mcp/tests/filesystem_mcp_contract.rs
use kept_mcp::mcp::server::build;
use kept_mcp::mcp::tools::filesystem::FffManager;

#[tokio::test]
async fn test_filesystem_tools_and_manager_flow() {
    let server = build().expect("server build");
    let manager = FffManager::new();
    let status = manager.status(".").await.expect("query status");
    assert_eq!(status.scanning_state, "ready");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-mcp --test filesystem_mcp_contract`
Expected: FAIL (missing tools/types)

- [ ] **Step 3: Implement FffManager and register tools**

Implement `FffManager` in `crate/kept-mcp/src/mcp/tools/filesystem.rs` and wire into `server.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-mcp --test filesystem_mcp_contract`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add crate/kept-mcp/src/mcp/tools/filesystem.rs crate/kept-mcp/tests/filesystem_mcp_contract.rs
git commit -m "feat(mcp): register FFF filesystem discovery tools and FffManager"
```

---

### Task 5: End-to-End Verification & Quality Gate

**Files:**
- Test: `Justfile`
- Test: `tests/test_public_cli_smoke.py`

- [ ] **Step 1: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: 174/174 tests PASS

- [ ] **Step 2: Run Clippy static analysis**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: 0 warnings

- [ ] **Step 3: Run repository pre-commit gate**

Run: `just check`
Expected: 100% PASS (code formatting, schema export validation, python CLI smoke, markdown links)

- [ ] **Step 4: Final commit**

```bash
git add .
git commit -m "chore: verify context admission and filesystem MCP pipeline with just check"
```
