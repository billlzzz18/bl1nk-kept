//! File types and metadata structures for directory scanning and persistent indexing.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    #[serde(rename = "modifiedUnix")]
    pub modified_unix: u64,
    pub kind: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanIndex {
    pub root: String,
    #[serde(rename = "scannedAtUnix")]
    pub scanned_at_unix: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub files: Vec<FileRecord>,
    #[serde(default)]
    pub issues: Vec<ScanIssue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ScanIssue {
    pub path: String,
    pub operation: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RefreshPlan {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    pub unchanged: Vec<String>,
}

pub const CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION: &str = "1.2.0";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentScanSnapshot {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub root: String,
    #[serde(rename = "rootFingerprint")]
    pub root_fingerprint: String,
    #[serde(rename = "includeHidden")]
    pub include_hidden: bool,
    #[serde(rename = "maxDepth")]
    pub max_depth: Option<usize>,
    pub index: ScanIndex,
}

pub fn create_persistent_snapshot(
    index: ScanIndex,
    options: &ScanOptions,
) -> PersistentScanSnapshot {
    let root_fingerprint = scan_context_fingerprint(&index.root, options);
    PersistentScanSnapshot {
        schema_version: CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION.to_string(),
        root: index.root.clone(),
        root_fingerprint,
        include_hidden: options.include_hidden,
        max_depth: options.max_depth,
        index,
    }
}

fn scan_context_fingerprint(root: &str, options: &ScanOptions) -> String {
    let mut digest = Sha256::new();
    digest.update(root.as_bytes());
    digest.update([0]);
    digest.update([u8::from(options.include_hidden)]);
    digest.update([0]);
    digest.update(options.max_depth.unwrap_or(usize::MAX).to_le_bytes());
    format!("{:x}", digest.finalize())
}

pub fn plan_incremental_refresh(
    previous: &ScanIndex,
    current: &ScanIndex,
) -> Result<RefreshPlan, String> {
    if previous.root != current.root {
        return Err("Cannot refresh indexes with different roots".to_string());
    }
    let previous_by_path = previous
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let current_by_path = current
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut plan = RefreshPlan::default();

    for (path, file) in &current_by_path {
        match previous_by_path.get(path) {
            None => plan.added.push((*path).to_string()),
            Some(previous_file) if file_changed(previous_file, file) => {
                plan.modified.push((*path).to_string())
            }
            Some(_) => plan.unchanged.push((*path).to_string()),
        }
    }
    for path in previous_by_path.keys() {
        if !current_by_path.contains_key(path) {
            plan.removed.push((*path).to_string());
        }
    }
    Ok(plan)
}

fn file_changed(previous: &FileRecord, current: &FileRecord) -> bool {
    previous.size != current.size
        || previous.modified_unix != current.modified_unix
        || previous.kind != current.kind
        || previous.extension != current.extension
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn to_unix(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
