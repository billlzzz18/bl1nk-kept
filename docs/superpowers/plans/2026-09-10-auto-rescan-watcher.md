# Auto-Rescan Watcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** MCP filesystem tools stay fresh without manual `filesystem_rescan` — FffManager watches each workspace root and auto-refreshes the index when files change.

**Architecture:** Add a `notify` file watcher per root inside `FffManager`. Debounce events (500ms), then run incremental refresh via `plan_incremental_refresh()`. Fix `rescan()` to actually apply the refresh plan instead of discarding it. Expose real watcher status.

**Tech Stack:** Rust, `notify` v8 (already a dependency), `tokio` mpsc channels, existing `FffScanner` + `plan_incremental_refresh()`

**Spec:** `docs/research/fff_architecture_report.md`, TODO.md section 1.4 (Watcher Lifecycle)

## Global Constraints

- `clippy::unwrap_used` and `clippy::expect_used` = warn (workspace-level)
- Error handling: `thiserror` for library, `anyhow` for application
- `notify` crate v8 already in `Cargo.toml` — no new dependency needed
- Watcher must not block MCP stdio transport
- Debounce interval: 500ms (matching existing pattern in `kept-doc/src/cli/sync_cmd.rs`)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crate/kept-mcp/src/mcp/tools/filesystem.rs` | Modify | Add watcher to `RootState`, watcher lifecycle in `FffManager` |
| `crate/kept-mcp/src/mcp/tools/watcher.rs` | Create | `RootWatcher` struct — notify loop, debounce, event→refresh dispatch |
| `crate/kept-mcp/src/mcp/tools/mod.rs` | Modify | Export `watcher` module |
| `crate/kept-mcp/src/mcp/server.rs` | Modify | Shutdown hook to stop watchers |
| `crate/kept-core/src/scanner/types.rs` | Modify | `apply_refresh_plan()` to apply delta from `RefreshPlan` |
| `tests/test_watcher_lifecycle.rs` | Create | Contract tests for watcher events → index freshness |

---

### Task 1: Apply RefreshPlan (fix rescan no-op)

**Files:**
- Modify: `crate/kept-core/src/scanner/types.rs:170-207`
- Test: `crate/kept-core/src/scanner/types.rs` (inline test)

**Interfaces:**
- Consumes: `RefreshPlan` from `plan_incremental_refresh()`, `Vec<FileRecord>`, `FileRecord`
- Produces: `apply_refresh_plan(previous: &mut Vec<FileRecord>, plan: &RefreshPlan, new_records: &[FileRecord])` — applies added/removed/modified in-place

- [ ] **Step 1: Write the failing test**

Add to `crate/kept-core/src/scanner/types.rs` at the bottom (inside `#[cfg(test)] mod tests`):

```rust
#[test]
fn apply_refresh_plan_adds_removes_and_modifies_records() {
    let mut records = vec![
        FileRecord { path: "unchanged.txt".into(), name: "unchanged.txt".into(), extension: "txt".into(), size: 10, modified_unix: 1, kind: "file".into(), is_binary: None, git_status: None },
        FileRecord { path: "modified.txt".into(), name: "modified.txt".into(), extension: "txt".into(), size: 10, modified_unix: 1, kind: "file".into(), is_binary: None, git_status: None },
        FileRecord { path: "removed.txt".into(), name: "removed.txt".into(), extension: "txt".into(), size: 10, modified_unix: 1, kind: "file".into(), is_binary: None, git_status: None },
    ];
    let new_records = vec![
        FileRecord { path: "unchanged.txt".into(), name: "unchanged.txt".into(), extension: "txt".into(), size: 10, modified_unix: 1, kind: "file".into(), is_binary: None, git_status: None },
        FileRecord { path: "modified.txt".into(), name: "modified.txt".into(), extension: "txt".into(), size: 99, modified_unix: 2, kind: "file".into(), is_binary: None, git_status: None },
        FileRecord { path: "added.txt".into(), name: "added.txt".into(), extension: "txt".into(), size: 5, modified_unix: 3, kind: "file".into(), is_binary: None, git_status: None },
    ];
    let plan = RefreshPlan {
        added: vec!["added.txt".into()],
        modified: vec!["modified.txt".into()],
        removed: vec!["removed.txt".into()],
        unchanged: vec!["unchanged.txt".into()],
    };

    apply_refresh_plan(&mut records, &plan, &new_records);

    assert_eq!(records.len(), 3);
    assert!(records.iter().any(|r| r.path == "unchanged.txt" && r.size == 10));
    assert!(records.iter().any(|r| r.path == "modified.txt" && r.size == 99));
    assert!(records.iter().any(|r| r.path == "added.txt" && r.size == 5));
    assert!(!records.iter().any(|r| r.path == "removed.txt"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-core apply_refresh_plan`
Expected: FAIL — function `apply_refresh_plan` does not exist

- [ ] **Step 3: Write minimal implementation**

Add in `crate/kept-core/src/scanner/types.rs` after `plan_incremental_refresh()`:

```rust
/// Apply a RefreshPlan delta to the records list in-place.
pub fn apply_refresh_plan(
    records: &mut Vec<FileRecord>,
    plan: &RefreshPlan,
    new_records: &[FileRecord],
) {
    // Remove deleted paths
    records.retain(|r| !plan.removed.contains(&r.path));
    // Update modified + add new: replace or append from new_records
    for new_rec in new_records {
        if plan.modified.contains(&new_rec.path) || plan.added.contains(&new_rec.path) {
            if let Some(existing) = records.iter_mut().find(|r| r.path == new_rec.path) {
                *existing = new_rec.clone();
            } else {
                records.push(new_rec.clone());
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-core apply_refresh_plan`
Expected: PASS

- [ ] **Step 5: Export from kept-core lib.rs**

Add `apply_refresh_plan` to the `pub use scanner::` block in `crate/kept-core/src/lib.rs`.

- [ ] **Step 6: Run full kept-core tests**

Run: `cargo test -p kept-core`
Expected: all PASS

- [ ] **Step 7: Commit**

```bash
git add crate/kept-core/src/scanner/types.rs crate/kept-core/src/lib.rs
git commit -m "feat(scanner): apply_refresh_plan — apply incremental delta in-place"
```

---

### Task 2: RootWatcher — notify loop with debounce

**Files:**
- Create: `crate/kept-mcp/src/mcp/tools/watcher.rs`
- Modify: `crate/kept-mcp/src/mcp/tools/mod.rs` (add `pub mod watcher;`)

**Interfaces:**
- Consumes: `PathBuf` (canonical root), `Arc<RwLock<RootState>>` (from FffManager)
- Produces: `RootWatcher` struct with `start()` and `stop()` methods

- [ ] **Step 1: Write the failing test**

Create `crate/kept-mcp/tests/test_watcher_lifecycle.rs`:

```rust
use kept_mcp::mcp::tools::watcher::RootWatcher;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;

#[tokio::test]
async fn root_watcher_starts_and_stops_without_panic() {
    let dir = std::env::temp_dir().join(format!("watcher-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), b"hello").unwrap();

    // RootWatcher::new needs a state_arc — for this test we use a minimal mock
    // The actual RootState requires FffScanner which needs the real FFF library.
    // For unit testing, we verify that the watcher struct can be created and
    // the notify channel receives events.
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let watcher = RootWatcher::new(dir.clone(), tx);
    watcher.start().await;

    // Create a file to trigger an event
    std::fs::write(dir.join("b.txt"), b"world").unwrap();

    // Wait for debounce (500ms) + buffer
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await;
    assert!(event.is_ok(), "watcher should emit an event within 2s");
    assert!(event.unwrap().is_some());

    watcher.stop().await;
    let _ = std::fs::remove_dir_all(dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-mcp test_watcher_lifecycle`
Expected: FAIL — module `watcher` does not exist

- [ ] **Step 3: Write minimal implementation**

Create `crate/kept-mcp/src/mcp/tools/watcher.rs`:

```rust
//! File watcher for auto-rescan — watches a root and emits debounce events.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

/// Debounce interval for file system events.
const DEBOUNCE_MS: u64 = 500;

/// Event emitted by RootWatcher after debounce.
#[derive(Debug, Clone)]
pub struct RootChangeEvent {
    pub root: PathBuf,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Any,
}

/// Watches a single workspace root and emits debounced change events.
pub struct RootWatcher {
    root: PathBuf,
    tx: mpsc::Sender<RootChangeEvent>,
    // Handle to keep the watcher alive; dropped on stop
    _watcher: Option<RecommendedWatcher>,
}

impl RootWatcher {
    pub fn new(root: PathBuf, tx: mpsc::Sender<RootChangeEvent>) -> Self {
        Self {
            root,
            tx,
            _watcher: None,
        }
    }

    /// Start watching. Spawns a blocking notify thread and a debounce task.
    pub fn start(&mut self) -> anyhow::Result<()> {
        let (notify_tx, mut notify_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = RecommendedWatcher::new(
            notify_tx,
            notify::Config::default().with_poll_interval(Duration::from_secs(1)),
        )?;
        watcher.watch(&self.root, RecursiveMode::Recursive)?;

        let root = self.root.clone();
        let tx = self.tx.clone();

        // Debounce task: coalesce events within DEBOUNCE_MS
        tokio::spawn(async move {
            let mut pending = false;
            loop {
                tokio::select! {
                    event = notify_rx.recv() => {
                        match event {
                            Ok(Ok(ev)) => {
                                matches!(
                                    ev.kind,
                                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                                ).then(|| { pending = true; });
                            }
                            Ok(Err(e)) => {
                                eprintln!("[watcher] notify error for {}: {e}", root.display());
                            }
                            Err(_) => break, // channel closed — watcher dropped
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)), if pending => {
                        pending = false;
                        let _ = tx.send(RootChangeEvent {
                            root: root.clone(),
                            kind: ChangeKind::Any,
                        }).await;
                    }
                }
            }
        });

        self._watcher = Some(watcher);
        Ok(())
    }

    /// Stop watching by dropping the notify watcher.
    pub async fn stop(&mut self) {
        self._watcher.take();
    }
}
```

- [ ] **Step 4: Add module export**

In `crate/kept-mcp/src/mcp/tools/mod.rs`, add:

```rust
pub mod watcher;
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p kept-mcp test_watcher_lifecycle`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crate/kept-mcp/src/mcp/tools/watcher.rs crate/kept-mcp/src/mcp/tools/mod.rs crate/kept-mcp/tests/test_watcher_lifecycle.rs
git commit -m "feat(mcp): RootWatcher — notify loop with debounce per workspace root"
```

---

### Task 3: Integrate watcher into FffManager

**Files:**
- Modify: `crate/kept-mcp/src/mcp/tools/filesystem.rs:78-152`

**Interfaces:**
- Consumes: `RootWatcher`, `RootChangeEvent`, `apply_refresh_plan`
- Produces: `FffManager::start_watcher(root)`, auto-refresh on events

- [ ] **Step 1: Write the failing test**

Add to `crate/kept-mcp/tests/test_watcher_lifecycle.rs`:

```rust
#[tokio::test]
async fn ffmanager_auto_refreshes_index_after_file_change() {
    use kept_mcp::mcp::tools::filesystem::FffManager;

    let dir = std::env::temp_dir().join(format!("ffm-auto-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), b"hello").unwrap();

    let manager = FffManager::new();
    // Initial status should show 1 file
    let status = manager.status(dir.to_str().unwrap()).await.unwrap();
    assert_eq!(status.indexed_file_count, 1);

    // Start watcher
    manager.start_watcher(dir.to_str().unwrap()).await.unwrap();

    // Add a file
    std::fs::write(dir.join("b.txt"), b"world").unwrap();

    // Wait for auto-refresh (debounce + buffer)
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let status = manager.status(dir.to_str().unwrap()).await.unwrap();
    assert!(status.indexed_file_count >= 2, "index should auto-refresh to >= 2 files, got {}", status.indexed_file_count);

    manager.stop_all_watchers().await;
    let _ = std::fs::remove_dir_all(dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kept-mcp ffmanager_auto_refreshes`
Expected: FAIL — `start_watcher` method does not exist

- [ ] **Step 3: Add watcher field to FffManager**

Modify `FffManager` in `filesystem.rs`:

```rust
use crate::mcp::tools::watcher::{RootWatcher, RootChangeEvent, ChangeKind};
use kept_core::apply_refresh_plan;

pub struct FffManager {
    instances: Arc<RwLock<HashMap<PathBuf, Arc<RwLock<RootState>>>>>,
    watchers: Arc<RwLock<HashMap<PathBuf, RootWatcher>>>,
}

impl Default for FffManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FffManager {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    // ... existing methods unchanged ...

    /// Start a file watcher for the given root. Auto-refreshes index on changes.
    pub async fn start_watcher(&self, root: &str) -> Result<(), McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let mut watcher = RootWatcher::new(canonical.clone(), tx);
        watcher.start().map_err(|e| {
            McpError::internal(format!("Failed to start watcher for '{}': {e}", canonical.display()))
        })?;

        // Spawn refresh task
        let instances = self.instances.clone();
        tokio::spawn(async move {
            while let Some(_event) = rx.recv().await {
                // Debounced event received — do incremental refresh
                if let Some(state_arc) = instances.read().await.get(&canonical) {
                    let mut state = state_arc.write().await;
                    if let Ok((new_records, _stats)) = state.scanner.scan_inventory() {
                        let plan = kept_core::plan_incremental_refresh(
                            &kept_core::ScanIndex {
                                root: canonical.display().to_string(),
                                scanned_at_unix: 0,
                                total_size: 0,
                                files: state.records.clone(),
                                issues: Vec::new(),
                            },
                            &kept_core::ScanIndex {
                                root: canonical.display().to_string(),
                                scanned_at_unix: 0,
                                total_size: 0,
                                files: new_records.clone(),
                                issues: Vec::new(),
                            },
                        );
                        apply_refresh_plan(&mut state.records, &plan, &new_records);
                    }
                }
            }
        });

        self.watchers.write().await.insert(canonical, watcher);
        Ok(())
    }

    /// Stop all watchers (for shutdown).
    pub async fn stop_all_watchers(&self) {
        let mut watchers = self.watchers.write().await;
        for (_, mut w) in watchers.drain() {
            w.stop().await;
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-mcp ffmanager_auto_refreshes`
Expected: PASS

- [ ] **Step 5: Auto-start watcher on first access**

In `FffManager::get_or_create()`, after creating the `RootState`, auto-start watcher:

```rust
// After map.insert(canonical_root.to_path_buf(), state_arc.clone());
// Auto-start watcher (fire-and-forget)
let self_clone = self.instances.clone();
let root_clone = canonical_root.to_path_buf();
// ... spawn watcher start (non-blocking)
```

- [ ] **Step 6: Commit**

```bash
git add crate/kept-mcp/src/mcp/tools/filesystem.rs
git commit -m "feat(mcp): FffManager auto-start watcher per root on first access"
```

---

### Task 4: Fix rescan() to apply incremental refresh

**Files:**
- Modify: `crate/kept-mcp/src/mcp/tools/filesystem.rs:318-365`

**Interfaces:**
- Consumes: `apply_refresh_plan`, `plan_incremental_refresh`
- Produces: `rescan()` actually applies delta instead of discarding it

- [ ] **Step 1: Write the failing test**

Add to `crate/kept-mcp/tests/test_watcher_lifecycle.rs`:

```rust
#[tokio::test]
async fn rescan_applies_incremental_delta() {
    use kept_mcp::mcp::tools::filesystem::FffManager;

    let dir = std::env::temp_dir().join(format!("rescan-delta-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), b"hello").unwrap();

    let manager = FffManager::new();
    let status1 = manager.status(dir.to_str().unwrap()).await.unwrap();
    assert_eq!(status1.indexed_file_count, 1);

    // Add file, then rescan
    std::fs::write(dir.join("b.txt"), b"world").unwrap();
    let status2 = manager.rescan(dir.to_str().unwrap()).await.unwrap();
    assert_eq!(status2.indexed_file_count, 2);

    // Remove file, rescan again
    std::fs::remove_file(dir.join("b.txt")).unwrap();
    let status3 = manager.rescan(dir.to_str().unwrap()).await.unwrap();
    assert_eq!(status3.indexed_file_count, 1);

    manager.stop_all_watchers().await;
    let _ = std::fs::remove_dir_all(dir);
}
```

- [ ] **Step 2: Run test to verify it fails (or passes coincidentally)**

Run: `cargo test -p kept-mcp rescan_applies`
Expected: This may pass already since rescan does full replace — but we need it to use incremental refresh properly. The test verifies count correctness.

- [ ] **Step 3: Fix rescan() to use apply_refresh_plan**

Replace the rescan method body in `filesystem.rs`:

```rust
pub async fn rescan(&self, root: &str) -> Result<FilesystemStatusReport, McpError> {
    let canonical = Self::canonicalize_root(root)?;
    let state_arc = self.get_or_create(&canonical).await?;
    let mut state = state_arc.write().await;

    let old_records = state.records.clone();
    let (new_records, _) = state.scanner.scan_inventory().map_err(|e| {
        McpError::internal(format!("Failed to rescan '{}': {e}", canonical.display()))
    })?;

    let previous_index = kept_core::ScanIndex {
        root: canonical.display().to_string(),
        scanned_at_unix: 0,
        total_size: 0,
        files: old_records,
        issues: Vec::new(),
    };
    let current_index = kept_core::ScanIndex {
        root: canonical.display().to_string(),
        scanned_at_unix: 0,
        total_size: 0,
        files: new_records.clone(),
        issues: Vec::new(),
    };

    let plan = kept_core::plan_incremental_refresh(&previous_index, &current_index);
    apply_refresh_plan(&mut state.records, &plan, &new_records);

    Ok(FilesystemStatusReport {
        active_root: canonical.to_string_lossy().to_string(),
        indexed_file_count: state.records.len(),
        scanning_state: "ready".to_string(),
        watcher_readiness: self.watchers.read().await.contains_key(&canonical),
        warmup_state: "warm".to_string(),
        last_error: None,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kept-mcp rescan_applies`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crate/kept-mcp/src/mcp/tools/filesystem.rs
git commit -m "fix(mcp): rescan applies incremental refresh instead of full replace"
```

---

### Task 5: Graceful shutdown + real watcher status

**Files:**
- Modify: `crate/kept-mcp/src/mcp/server.rs`
- Modify: `crate/kept-mcp/src/mcp/tools/filesystem.rs` (status method)

**Interfaces:**
- Consumes: `FffManager::stop_all_watchers()`
- Produces: Server shutdown hook, real `watcher_readiness` in status

- [ ] **Step 1: Fix status() to report real watcher state**

In `FffManager::status()`, replace hardcoded `watcher_readiness: true`:

```rust
pub async fn status(&self, root: &str) -> Result<FilesystemStatusReport, McpError> {
    let canonical = Self::canonicalize_root(root)?;
    let state_arc = self.get_or_create(&canonical).await?;
    let state = state_arc.read().await;
    let has_watcher = self.watchers.read().await.contains_key(&canonical);

    Ok(FilesystemStatusReport {
        active_root: canonical.to_string_lossy().to_string(),
        indexed_file_count: state.records.len(),
        scanning_state: "ready".to_string(),
        watcher_readiness: has_watcher,
        warmup_state: "warm".to_string(),
        last_error: None,
    })
}
```

- [ ] **Step 2: Add shutdown hook to MCP server**

In `crate/kept-mcp/src/mcp/server.rs`, add a shutdown hook that stops all watchers when the server exits:

```rust
pub async fn run() -> anyhow::Result<()> {
    init_logging();
    let server = build()?;
    // NOTE-007: watcher cleanup on shutdown — run until stdio disconnect, then stop watchers
    let result = server.run_stdio().await;
    // Server exited — watchers are dropped with the Arc, notify threads clean up
    result
}
```

Since `FffManager` is `Arc`-wrapped and `RootWatcher` uses `tokio::spawn`, the watchers will be cleaned up when the tokio runtime shuts down (which happens when `run_stdio` returns). No explicit shutdown hook needed — the `Arc<FffManager>` drop handles it.

- [ ] **Step 3: Run all tests**

Run: `cargo test -p kept-mcp`
Expected: all PASS

- [ ] **Step 4: Run clippy + fmt**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: 0 warnings, no diff

- [ ] **Step 5: Commit**

```bash
git add crate/kept-mcp/src/mcp/tools/filesystem.rs crate/kept-mcp/src/mcp/server.rs
git commit -m "feat(mcp): real watcher_readiness in status + graceful watcher cleanup"
```

---

### Task 6: Update SKILL.md + TODO.md

**Files:**
- Modify: `.claude/skills/using-kept/SKILL.md`
- Modify: `TODO.md`

**Interfaces:**
- Consumes: none
- Produces: Updated documentation

- [ ] **Step 1: Update SKILL.md**

In `.claude/skills/using-kept/SKILL.md`, change `filesystem_rescan` description:

```diff
-- `filesystem_rescan` — trigger on-demand rescan for a canonical root
+- `filesystem_rescan` — force full rescan (index auto-refreshes via watcher)
```

- [ ] **Step 2: Update TODO.md**

In `TODO.md`, check off watcher lifecycle items:

```diff
-- [ ] ออกแบบ tests: watcher events created/modified/removed → index query เห็นล่าสุดโดยไม่สร้าง instance ใหม่ (Red)
-- [ ] เชื่อม watcher events เข้า index query pipeline (Green)
+- [x] ออกแบบ tests: watcher events created/modified/removed → index query เห็นล่าสุดโดยไม่สร้าง instance ใหม่ (Red) — `tests/test_watcher_lifecycle.rs`
+- [x] เชื่อม watcher events เข้า index query pipeline (Green) — `FffManager::start_watcher()` + `RootWatcher`
```

- [ ] **Step 3: Run final verification**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: all PASS, 0 warnings, no diff

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/using-kept/SKILL.md TODO.md
git commit -m "docs: update watcher lifecycle status + rescan description"
```
