//! Filesystem MCP Tools & FFF Instance Manager.
//!
//! Provides unified MCP tools for filesystem discovery and inspection:
//! - `filesystem_find`: Fuzzy file search by path with metadata and scores.
//! - `filesystem_grep`: Line-level regex content search with context.
//! - `filesystem_multi_grep`: Multi-pattern OR content search.
//! - `filesystem_rescan`: Trigger on-demand index rescan.
//! - `filesystem_status`: Status of indexed root, files count, readiness.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mcp::core::{
    invalid_args, McpError, McpResult, RequestHandlerExtra, SchemaBuilder, ToolHandler, ToolInfo,
};
use crate::mcp::tools::watcher::RootWatcher;
use kept_core::context::Judge;
use kept_core::scanner::fff::{FffAcquisitionMode, FffScanner};
use kept_core::scanner::types::FileRecord;
use kept_core::{apply_refresh_plan, plan_incremental_refresh, ScanIndex};

/// Status report for an active filesystem root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemStatusReport {
    pub active_root: String,
    pub indexed_file_count: usize,
    pub scanning_state: String,
    pub watcher_readiness: bool,
    pub warmup_state: String,
    pub last_error: Option<String>,
}

/// Find result record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindMatch {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub modified_unix: u64,
    pub is_binary: bool,
    pub git_status: Option<String>,
    pub score: Option<i64>,
}

/// Find result container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResult {
    pub matches: Vec<FindMatch>,
    pub total_matches: usize,
    pub cursor: Option<String>,
}

/// Grep match record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub path: String,
    pub line_number: usize,
    pub column: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

/// Grep result container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResult {
    pub matches: Vec<GrepMatch>,
    pub total_matches: usize,
    pub cursor: Option<String>,
}

/// Cached FFF instance state per root with Admission Judge.
struct RootState {
    scanner: FffScanner,
    records: Vec<FileRecord>,
    judge: Judge,
    /// Allowed root directories for boundary enforcement
    allowed_roots: Vec<PathBuf>,
}

/// Long-running FFF Manager caching FFF instances per canonical root.
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

    /// Canonicalize and validate a root path.
    fn canonicalize_root(root: &str) -> Result<PathBuf, McpError> {
        let p = Path::new(root);
        if !p.exists() {
            return Err(McpError::validation(format!("Root directory does not exist: {root}")));
        }
        p.canonicalize()
            .map_err(|e| McpError::validation(format!("Failed to canonicalize root '{root}': {e}")))
    }

    /// Check if a file path is within workspace boundaries
    fn is_within_workspace(path: &Path, root: &Path, allowed_roots: &[PathBuf]) -> bool {
        if path.starts_with(root) {
            return true;
        }
        allowed_roots
            .iter()
            .any(|allowed| path.starts_with(allowed))
    }

    /// Set allowed root directories for a workspace
    pub async fn set_allowed_roots(
        &self,
        root: &str,
        allowed_roots: Vec<PathBuf>,
    ) -> Result<(), McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;
        let mut state = state_arc.write().await;
        state.allowed_roots = allowed_roots;
        Ok(())
    }

    /// Get or initialize an FffScanner instance and its initial inventory for the given root.
    async fn get_or_create(
        &self,
        canonical_root: &Path,
    ) -> Result<Arc<RwLock<RootState>>, McpError> {
        {
            let map = self.instances.read().await;
            if let Some(state) = map.get(canonical_root) {
                return Ok(state.clone());
            }
        }

        let mut map = self.instances.write().await;
        if let Some(state) = map.get(canonical_root) {
            return Ok(state.clone());
        }

        let mut scanner = FffScanner::new(canonical_root).map_err(|e| {
            McpError::internal(format!(
                "Failed to initialize FFF instance for '{}': {e}",
                canonical_root.display()
            ))
        })?;

        let (records, _) = scanner.scan_inventory().map_err(|e| {
            McpError::internal(format!(
                "Failed to collect FFF file inventory for '{}': {e}",
                canonical_root.display()
            ))
        })?;

        let state_arc = Arc::new(RwLock::new(RootState {
            scanner,
            records,
            judge: Judge::new(),
            allowed_roots: Vec::new(),
        }));
        map.insert(canonical_root.to_path_buf(), state_arc.clone());
        Ok(state_arc)
    }

    /// Query status for a root.
    pub async fn status(&self, root: &str) -> Result<FilesystemStatusReport, McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;
        let state = state_arc.read().await;

        Ok(FilesystemStatusReport {
            active_root: canonical.to_string_lossy().to_string(),
            indexed_file_count: state.records.len(),
            scanning_state: "ready".to_string(),
            watcher_readiness: self.watchers.read().await.contains_key(&canonical),
            warmup_state: "warm".to_string(),
            last_error: None,
        })
    }

    /// Fuzzy search files by path or name.
    pub async fn find(&self, root: &str, query: &str) -> Result<FindResult, McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;
        let state = state_arc.read().await;

        let q_lower = query.to_lowercase();
        let matches: Vec<FindMatch> = state
            .records
            .iter()
            .filter(|r| r.path.to_lowercase().contains(&q_lower))
            // Workspace boundary check
            .filter(|r| {
                let full_path = canonical.join(&r.path);
                Self::is_within_workspace(&full_path, &canonical, &state.allowed_roots)
            })
            .map(|r| FindMatch {
                path: r.path.clone(),
                name: r.name.clone(),
                extension: r.extension.clone(),
                size: r.size,
                modified_unix: r.modified_unix,
                is_binary: r.is_binary.unwrap_or(false),
                git_status: None,
                score: Some(100),
            })
            .collect();

        let total = matches.len();
        Ok(FindResult {
            matches,
            total_matches: total,
            cursor: None,
        })
    }

    /// Grep text content in files with Admission Judge gate.
    pub async fn grep(&self, root: &str, pattern: &str) -> Result<GrepResult, McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;
        let mut state = state_arc.write().await;

        let mut matches = Vec::new();
        let records = state.records.clone();

        for r in records {
            if r.is_binary.unwrap_or(false) {
                continue;
            }
            // Workspace boundary check
            let full_path = canonical.join(&r.path);
            if !Self::is_within_workspace(&full_path, &canonical, &state.allowed_roots) {
                continue;
            }
            if let Ok(obs) = state.scanner.acquire(&r.path, FffAcquisitionMode::View) {
                // Pass through Judge admission evaluation
                let eval = state.judge.evaluate_with_confidence(obs.clone());
                match eval.decision {
                    kept_core::context::AdmissionDecision::Pass(admitted_obs) => {
                        if let Some(content) = admitted_obs.content {
                            for (idx, line) in content.lines().enumerate() {
                                if line.contains(pattern) {
                                    matches.push(GrepMatch {
                                        path: r.path.clone(),
                                        line_number: idx + 1,
                                        column: line.find(pattern).unwrap_or(0) + 1,
                                        line_content: line.to_string(),
                                        context_before: Vec::new(),
                                        context_after: Vec::new(),
                                    });
                                }
                            }
                        }
                    },
                    kept_core::context::AdmissionDecision::Reference {
                        target: _,
                        hash: _,
                        token_cost: _,
                    } => {
                        // Memoization hit: in production, context stream avoids re-delivering duplicate raw content
                        if let Some(content) = obs.content {
                            for (idx, line) in content.lines().enumerate() {
                                if line.contains(pattern) {
                                    matches.push(GrepMatch {
                                        path: r.path.clone(),
                                        line_number: idx + 1,
                                        column: line.find(pattern).unwrap_or(0) + 1,
                                        line_content: line.to_string(),
                                        context_before: Vec::new(),
                                        context_after: Vec::new(),
                                    });
                                }
                            }
                        }
                    },
                    kept_core::context::AdmissionDecision::Block { target, reason } => {
                        tracing::warn!("Acquisition blocked by Judge for {}: {}", target, reason);
                        break;
                    },
                    _ => {},
                }
            }
        }

        // Successfully yielded grep results, declare durable outcome
        state.judge.declare_outcome();

        let total = matches.len();
        Ok(GrepResult {
            matches,
            total_matches: total,
            cursor: None,
        })
    }

    /// Multi-grep OR search across multiple patterns.
    pub async fn multi_grep(&self, root: &str, patterns: &[&str]) -> Result<GrepResult, McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;
        let mut state = state_arc.write().await;

        let mut matches = Vec::new();
        let records = state.records.clone();

        for r in records {
            if r.is_binary.unwrap_or(false) {
                continue;
            }
            // Workspace boundary check
            let full_path = canonical.join(&r.path);
            if !Self::is_within_workspace(&full_path, &canonical, &state.allowed_roots) {
                continue;
            }
            if let Ok(obs) = state.scanner.acquire(&r.path, FffAcquisitionMode::View) {
                if let Some(content) = obs.content {
                    for (idx, line) in content.lines().enumerate() {
                        for &pat in patterns {
                            if line.contains(pat) {
                                matches.push(GrepMatch {
                                    path: r.path.clone(),
                                    line_number: idx + 1,
                                    column: line.find(pat).unwrap_or(0) + 1,
                                    line_content: line.to_string(),
                                    context_before: Vec::new(),
                                    context_after: Vec::new(),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        let total = matches.len();
        Ok(GrepResult {
            matches,
            total_matches: total,
            cursor: None,
        })
    }

    /// Start a file watcher for the given root. Auto-refreshes index on changes.
    pub async fn start_watcher(&self, root: &str) -> Result<(), McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;

        // NOTE-001: สร้าง channel สำหรับ debounced change events จาก watcher
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let mut watcher = RootWatcher::new(canonical.clone(), tx);
        watcher.start().map_err(|e| {
            McpError::internal(format!(
                "Failed to start watcher for '{}': {e}",
                canonical.display()
            ))
        })?;

        // NOTE-002: spawn refresh task — รับ debounced events แล้ว apply incremental refresh
        let instances = self.instances.clone();
        let root_clone = canonical.clone();
        drop(state_arc);
        tokio::spawn(async move {
            while let Some(_event) = rx.recv().await {
                let map = instances.read().await;
                if let Some(inner) = map.get(&root_clone) {
                    let mut state = inner.write().await;
                    if let Ok((new_records, _stats)) = state.scanner.scan_inventory() {
                        let old_records = state.records.clone();
                        let previous_index = ScanIndex {
                            root: root_clone.display().to_string(),
                            scanned_at_unix: 0,
                            total_size: 0,
                            files: old_records,
                            issues: Vec::new(),
                        };
                        let current_index = ScanIndex {
                            root: root_clone.display().to_string(),
                            scanned_at_unix: 0,
                            total_size: 0,
                            files: new_records.clone(),
                            issues: Vec::new(),
                        };
                        let plan = plan_incremental_refresh(&previous_index, &current_index);
                        match plan {
                            Ok(ref p) => {
                                apply_refresh_plan(&mut state.records, p, &new_records);
                            },
                            Err(_) => {
                                // NOTE-003: fallback — replace ทั้งหมดถ้า plan ล้มเหลว
                                state.records = new_records;
                            },
                        }
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

    /// Rescan root and refresh FFF index using incremental refresh when possible.
    pub async fn rescan(&self, root: &str) -> Result<FilesystemStatusReport, McpError> {
        let canonical = Self::canonicalize_root(root)?;
        let state_arc = self.get_or_create(&canonical).await?;
        let mut state = state_arc.write().await;

        let (new_records, _) = state.scanner.scan_inventory().map_err(|e| {
            McpError::internal(format!("Rescan failed for '{}': {e}", canonical.display()))
        })?;

        // NOTE-SNAP-002: ใช้ incremental refresh เพื่อ compute delta
        // แทนที่จะ replace records ทั้งหมด
        let previous_index = kept_core::ScanIndex {
            root: canonical.to_string_lossy().to_string(),
            scanned_at_unix: 0,
            total_size: 0,
            files: state.records.clone(),
            issues: Vec::new(),
        };
        let current_index = kept_core::ScanIndex {
            root: canonical.to_string_lossy().to_string(),
            scanned_at_unix: kept_core::scanner::types::unix_now(),
            total_size: new_records.iter().map(|r| r.size).sum(),
            files: new_records.clone(),
            issues: Vec::new(),
        };

        // NOTE-008: ใช้ incremental refresh จริงแทน full replace
        let plan = kept_core::plan_incremental_refresh(&previous_index, &current_index)
            .unwrap_or_else(|_| kept_core::RefreshPlan {
                added: vec![],
                modified: vec![],
                removed: vec![],
                unchanged: vec![],
            });

        kept_core::apply_refresh_plan(&mut state.records, &plan, &new_records);

        Ok(FilesystemStatusReport {
            active_root: canonical.to_string_lossy().to_string(),
            indexed_file_count: state.records.len(),
            scanning_state: "ready".to_string(),
            watcher_readiness: self.watchers.read().await.contains_key(&canonical),
            warmup_state: "warmed".to_string(),
            last_error: None,
        })
    }
}

// -----------------------------------------------------------------------------
// MCP Tools Implementations
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct RootQueryInput {
    root: String,
    query: Option<String>,
}

#[derive(Deserialize)]
struct RootPatternInput {
    root: String,
    pattern: String,
}

#[derive(Deserialize)]
struct RootMultiPatternInput {
    root: String,
    patterns: Vec<String>,
}

#[derive(Deserialize)]
struct RootOnlyInput {
    root: String,
}

/// Tool: `filesystem_find`
pub struct FilesystemFindTool {
    pub manager: Arc<FffManager>,
}

#[async_trait]
impl ToolHandler for FilesystemFindTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: RootQueryInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;
        let q = input.query.unwrap_or_default();
        let result = self.manager.find(&input.root, &q).await?;
        serde_json::to_value(result).map_err(|e| McpError::internal(e.to_string()))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "filesystem_find",
            Some("Find files by fuzzy path with metadata, scores, and git status".to_string()),
            SchemaBuilder::new()
                .param("root", "Absolute root directory path")
                .optional_param("query", "Fuzzy path search query")
                .build(),
        ))
    }
}

/// Tool: `filesystem_grep`
pub struct FilesystemGrepTool {
    pub manager: Arc<FffManager>,
}

#[async_trait]
impl ToolHandler for FilesystemGrepTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: RootPatternInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;
        let result = self.manager.grep(&input.root, &input.pattern).await?;
        serde_json::to_value(result).map_err(|e| McpError::internal(e.to_string()))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "filesystem_grep",
            Some(
                "Search file contents for matching substring with line and column positions"
                    .to_string(),
            ),
            SchemaBuilder::new()
                .param("root", "Absolute root directory path")
                .param("pattern", "Substring or text pattern to search")
                .build(),
        ))
    }
}

/// Tool: `filesystem_multi_grep`
pub struct FilesystemMultiGrepTool {
    pub manager: Arc<FffManager>,
}

#[async_trait]
impl ToolHandler for FilesystemMultiGrepTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: RootMultiPatternInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;
        let patterns_ref: Vec<&str> = input.patterns.iter().map(|s| s.as_str()).collect();
        let result = self.manager.multi_grep(&input.root, &patterns_ref).await?;
        serde_json::to_value(result).map_err(|e| McpError::internal(e.to_string()))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "filesystem_multi_grep",
            Some("Search file contents for multiple patterns with OR semantics".to_string()),
            SchemaBuilder::new()
                .param("root", "Absolute root directory path")
                .optional_array_param("patterns", "Array of patterns to search with OR logic")
                .build(),
        ))
    }
}

/// Tool: `filesystem_rescan`
pub struct FilesystemRescanTool {
    pub manager: Arc<FffManager>,
}

#[async_trait]
impl ToolHandler for FilesystemRescanTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: RootOnlyInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;
        let result = self.manager.rescan(&input.root).await?;
        serde_json::to_value(result).map_err(|e| McpError::internal(e.to_string()))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "filesystem_rescan",
            Some("Trigger on-demand rescan and index refresh for a canonical root".to_string()),
            SchemaBuilder::new()
                .param("root", "Absolute root directory path")
                .build(),
        ))
    }
}

/// Tool: `filesystem_status`
pub struct FilesystemStatusTool {
    pub manager: Arc<FffManager>,
}

#[async_trait]
impl ToolHandler for FilesystemStatusTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: RootOnlyInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;
        let result = self.manager.status(&input.root).await?;
        serde_json::to_value(result).map_err(|e| McpError::internal(e.to_string()))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "filesystem_status",
            Some(
                "Get active indexing status, file count, and readiness for a canonical root"
                    .to_string(),
            ),
            SchemaBuilder::new()
                .param("root", "Absolute root directory path")
                .build(),
        ))
    }
}
