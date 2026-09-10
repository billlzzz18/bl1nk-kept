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
    #[serde(rename = "isBinary", default)]
    pub is_binary: Option<bool>,
    #[serde(rename = "gitStatus", default)]
    pub git_status: Option<String>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanIssueKind {
    PermissionDenied,
    NotFound,
    LockedOrBusy,
    InvalidEncoding,
    CorruptData,
    BadExtension,
    #[default]
    GenericIo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ScanIssue {
    pub path: String,
    pub operation: String,
    #[serde(default)]
    pub kind: ScanIssueKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

impl ScanIssue {
    pub fn new(
        path: impl Into<String>,
        operation: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let msg_str = message.into();
        let (kind, remediation) = classify_io_message(&msg_str);
        Self {
            path: path.into(),
            operation: operation.into(),
            kind,
            message: msg_str,
            remediation,
        }
    }
}

fn classify_io_message(msg: &str) -> (ScanIssueKind, Option<String>) {
    let lower = msg.to_lowercase();
    if lower.contains("permission")
        || lower.contains("access is denied")
        || lower.contains("operation not permitted")
    {
        (
            ScanIssueKind::PermissionDenied,
            Some("Check file permissions or run with appropriate access privileges.".to_string()),
        )
    } else if lower.contains("not found")
        || lower.contains("cannot find")
        || lower.contains("no such file")
    {
        (
            ScanIssueKind::NotFound,
            Some("Verify path exists and has not been moved or deleted.".to_string()),
        )
    } else if lower.contains("used by another process")
        || lower.contains("locked")
        || lower.contains("busy")
    {
        (
            ScanIssueKind::LockedOrBusy,
            Some("Close the application currently locking this file and retry.".to_string()),
        )
    } else if lower.contains("encoding") || lower.contains("invalid utf-8") {
        (
            ScanIssueKind::InvalidEncoding,
            Some("File contains invalid character encoding.".to_string()),
        )
    } else {
        (
            ScanIssueKind::GenericIo,
            Some("Check storage device integrity and file accessibility.".to_string()),
        )
    }
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
    hex::encode(digest.finalize())
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

/// Apply a RefreshPlan delta to the records list in-place.
///
// NOTE-REFRESH-001: ใช้สำหรับ rescan no-op fix — อัปเด트 records ตาม plan
/// โดยไม่ต้อง re-read ทั้งหมดใหม่
pub fn apply_refresh_plan(
    records: &mut Vec<FileRecord>,
    plan: &RefreshPlan,
    new_records: &[FileRecord],
) {
    // ลบ records ที่ถูก mark ว่า removed
    records.retain(|r| !plan.removed.contains(&r.path));
    // อัปเดต/เพิ่ม records ที่ added หรือ modified จาก new_records
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

/// Migrate a snapshot from an older schema version to the current version.
///
/// Currently only version "1.1.0" → "1.2.0" is supported (added `root_fingerprint`).
/// Returns `None` if the snapshot is already at the current version or has an unknown version.
pub fn migrate_snapshot(
    snapshot: PersistentScanSnapshot,
) -> Result<PersistentScanSnapshot, String> {
    match snapshot.schema_version.as_str() {
        v if v == CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION => Ok(snapshot),
        "1.1.0" => {
            // NOTE-SNAP-001: v1.1.0 → v1.2.0 — เพิ่ม root_fingerprint field
            let root_fingerprint =
                scan_context_fingerprint(&snapshot.root, &ScanOptions {
                    include_hidden: snapshot.include_hidden,
                    max_depth: snapshot.max_depth,
                });
            Ok(PersistentScanSnapshot {
                schema_version: CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION.to_string(),
                root_fingerprint,
                ..snapshot
            })
        }
        other => Err(format!(
            "unsupported snapshot schema version: {other} (expected {CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION})"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(path: &str, size: u64, modified: u64) -> FileRecord {
        FileRecord {
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            extension: path.rsplit('.').next().unwrap_or("").to_string(),
            size,
            modified_unix: modified,
            kind: "file".to_string(),
            is_binary: None,
            git_status: None,
        }
    }

    fn make_index(root: &str, files: Vec<FileRecord>) -> ScanIndex {
        let total_size = files.iter().map(|f| f.size).sum();
        ScanIndex {
            root: root.to_string(),
            scanned_at_unix: 1000,
            total_size,
            files,
            issues: Vec::new(),
        }
    }

    #[test]
    fn incremental_refresh_detects_added_files() {
        let previous = make_index("/root", vec![make_record("a.txt", 10, 100)]);
        let current = make_index(
            "/root",
            vec![make_record("a.txt", 10, 100), make_record("b.txt", 20, 200)],
        );
        let plan = plan_incremental_refresh(&previous, &current).unwrap();
        assert_eq!(plan.added, vec!["b.txt"]);
        assert!(plan.modified.is_empty());
        assert!(plan.removed.is_empty());
    }

    #[test]
    fn incremental_refresh_detects_modified_files() {
        let previous = make_index("/root", vec![make_record("a.txt", 10, 100)]);
        let current = make_index("/root", vec![make_record("a.txt", 99, 200)]);
        let plan = plan_incremental_refresh(&previous, &current).unwrap();
        assert!(plan.added.is_empty());
        assert_eq!(plan.modified, vec!["a.txt"]);
        assert!(plan.removed.is_empty());
    }

    #[test]
    fn incremental_refresh_detects_removed_files() {
        let previous = make_index(
            "/root",
            vec![make_record("a.txt", 10, 100), make_record("b.txt", 20, 200)],
        );
        let current = make_index("/root", vec![make_record("a.txt", 10, 100)]);
        let plan = plan_incremental_refresh(&previous, &current).unwrap();
        assert!(plan.added.is_empty());
        assert!(plan.modified.is_empty());
        assert_eq!(plan.removed, vec!["b.txt"]);
    }

    #[test]
    fn incremental_refresh_tracks_unchanged_files() {
        let previous = make_index("/root", vec![make_record("a.txt", 10, 100)]);
        let current = make_index("/root", vec![make_record("a.txt", 10, 100)]);
        let plan = plan_incremental_refresh(&previous, &current).unwrap();
        assert!(plan.added.is_empty());
        assert!(plan.modified.is_empty());
        assert!(plan.removed.is_empty());
        assert_eq!(plan.unchanged, vec!["a.txt"]);
    }

    #[test]
    fn incremental_refresh_rejects_different_roots() {
        let previous = make_index("/root-a", vec![make_record("a.txt", 10, 100)]);
        let current = make_index("/root-b", vec![make_record("a.txt", 10, 100)]);
        assert!(plan_incremental_refresh(&previous, &current).is_err());
    }

    #[test]
    fn snapshot_migration_current_version_passthrough() {
        let snapshot = PersistentScanSnapshot {
            schema_version: CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION.to_string(),
            root: "/root".to_string(),
            root_fingerprint: "abc".to_string(),
            include_hidden: false,
            max_depth: None,
            index: make_index("/root", vec![]),
        };
        let migrated = migrate_snapshot(snapshot.clone()).unwrap();
        assert_eq!(
            migrated.schema_version,
            CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION
        );
        assert_eq!(migrated.root_fingerprint, "abc");
    }

    #[test]
    fn snapshot_migration_1_1_to_1_2_adds_fingerprint() {
        let snapshot = PersistentScanSnapshot {
            schema_version: "1.1.0".to_string(),
            root: "/root".to_string(),
            root_fingerprint: String::new(), // empty in v1.1.0
            include_hidden: false,
            max_depth: None,
            index: make_index("/root", vec![]),
        };
        let migrated = migrate_snapshot(snapshot).unwrap();
        assert_eq!(
            migrated.schema_version,
            CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION
        );
        assert!(
            !migrated.root_fingerprint.is_empty(),
            "fingerprint must be computed"
        );
    }

    #[test]
    fn snapshot_migration_unknown_version_rejected() {
        let snapshot = PersistentScanSnapshot {
            schema_version: "99.0.0".to_string(),
            root: "/root".to_string(),
            root_fingerprint: String::new(),
            include_hidden: false,
            max_depth: None,
            index: make_index("/root", vec![]),
        };
        assert!(migrate_snapshot(snapshot).is_err());
    }

    #[test]
    fn apply_refresh_plan_adds_removes_and_modifies_records() {
        let mut records = vec![
            FileRecord {
                path: "unchanged.txt".into(),
                name: "unchanged.txt".into(),
                extension: "txt".into(),
                size: 10,
                modified_unix: 1,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
            FileRecord {
                path: "modified.txt".into(),
                name: "modified.txt".into(),
                extension: "txt".into(),
                size: 10,
                modified_unix: 1,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
            FileRecord {
                path: "removed.txt".into(),
                name: "removed.txt".into(),
                extension: "txt".into(),
                size: 10,
                modified_unix: 1,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
        ];
        let new_records = vec![
            FileRecord {
                path: "unchanged.txt".into(),
                name: "unchanged.txt".into(),
                extension: "txt".into(),
                size: 10,
                modified_unix: 1,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
            FileRecord {
                path: "modified.txt".into(),
                name: "modified.txt".into(),
                extension: "txt".into(),
                size: 99,
                modified_unix: 2,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
            FileRecord {
                path: "added.txt".into(),
                name: "added.txt".into(),
                extension: "txt".into(),
                size: 5,
                modified_unix: 3,
                kind: "file".into(),
                is_binary: None,
                git_status: None,
            },
        ];
        let plan = RefreshPlan {
            added: vec!["added.txt".into()],
            modified: vec!["modified.txt".into()],
            removed: vec!["removed.txt".into()],
            unchanged: vec!["unchanged.txt".into()],
        };

        apply_refresh_plan(&mut records, &plan, &new_records);

        assert_eq!(records.len(), 3);
        assert!(records
            .iter()
            .any(|r| r.path == "unchanged.txt" && r.size == 10));
        assert!(records
            .iter()
            .any(|r| r.path == "modified.txt" && r.size == 99));
        assert!(records.iter().any(|r| r.path == "added.txt" && r.size == 5));
        assert!(!records.iter().any(|r| r.path == "removed.txt"));
    }
}
