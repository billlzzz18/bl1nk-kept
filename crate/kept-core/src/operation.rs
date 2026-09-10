use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

use crate::policy::{NamingIssue, UserConfig};
use crate::scanner::{DuplicateGroup, FileRecord, ScanIndex};

// NOTE-001: Error ประเภทต่าง ๆ สำหรับการตรวจสอบความถูกต้องของ OperationContract
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("invalid path in duplicate group")]
    InvalidPath,
    #[error("empty groups for duplicate removal")]
    EmptyGroups,
    #[error("invalid rename change: {0}")]
    InvalidRename(String),
    #[error("invalid move change: {0}")]
    InvalidMove(String),
}

/// การเปลี่ยนแปลงสำหรับการเปลี่ยนชื่อไฟล์
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameChange {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// การเปลี่ยนแปลงสำหรับการย้ายไฟล์
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveChange {
    pub source: PathBuf,
    pub destination: PathBuf,
}

/// ประเด็นปัญหาเกี่ยวกับ Naming พร้อมพาธเป้าหมายสำหรับเปลี่ยนชื่อ
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamingIssueItem {
    pub path: PathBuf,
    pub proposed_path: Option<PathBuf>,
    pub issues: Vec<NamingIssue>,
}

impl NamingIssueItem {
    pub fn to_rename_change(&self) -> RenameChange {
        RenameChange {
            from: self.path.clone(),
            to: self
                .proposed_path
                .clone()
                .unwrap_or_else(|| self.path.clone()),
        }
    }
}

/// สถานะปัจจุบันที่สังเกตได้ของระบบไฟล์
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedState {
    pub records: Vec<FileRecord>,
    pub scan_index: ScanIndex,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub duplicates: Vec<DuplicateGroup>,
    #[serde(default)]
    pub naming_issues: Vec<NamingIssueItem>,
}

impl ObservedState {
    pub fn new(scan_index: ScanIndex) -> Self {
        Self {
            records: scan_index.files.clone(),
            scan_index,
            timestamp: Utc::now(),
            duplicates: Vec::new(),
            naming_issues: Vec::new(),
        }
    }

    pub fn with_duplicates(mut self, duplicates: Vec<DuplicateGroup>) -> Self {
        self.duplicates = duplicates;
        self
    }

    pub fn with_naming_issues(mut self, naming_issues: Vec<NamingIssueItem>) -> Self {
        self.naming_issues = naming_issues;
        self
    }
}

/// ผลลัพธ์ที่คาดหวังจากการทำ operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExpectedEffect {
    /// ลบไฟล์ซ้ำ
    DuplicateRemoval { groups: Vec<DuplicateGroup> },
    /// เปลี่ยนชื่อไฟล์
    Rename { changes: Vec<RenameChange> },
    /// ย้ายไฟล์
    Move { changes: Vec<MoveChange> },
    /// ไม่ทำอะไร (validation only)
    NoOp { reason: String },
}

/// Policy alias สำหรับการใช้งานใน OperationContract
pub type Policy = UserConfig;

/// Formal contract ระหว่าง observed state, expected effect, และ policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationContract {
    /// สถานะปัจจุบันที่สังเกตได้
    pub observed: ObservedState,
    /// ผลลัพธ์ที่คาดหวัง
    pub expected: ExpectedEffect,
    /// กฎ/constraints ที่ต้องปฏิบัติตาม
    pub policy: Policy,
}

impl OperationContract {
    /// ตรวจสอบว่า effect ถูกต้องก่อน execute
    pub fn validate(&self) -> Result<(), ValidationError> {
        match &self.expected {
            ExpectedEffect::DuplicateRemoval { groups } => {
                if groups.is_empty() {
                    return Err(ValidationError::EmptyGroups);
                }
                for group in groups {
                    // ตรวจสอบว่า items ทั้งหมดหรืออย่างน้อย path มีอยู่ใน observed records
                    for item in &group.items {
                        if !self
                            .observed
                            .records
                            .iter()
                            .any(|record| record.path == *item)
                        {
                            return Err(ValidationError::InvalidPath);
                        }
                    }
                }
                Ok(())
            },
            ExpectedEffect::Rename { changes } => {
                for change in changes {
                    let from_str = change.from.to_string_lossy();
                    if !self
                        .observed
                        .records
                        .iter()
                        .any(|record| record.path == from_str)
                    {
                        return Err(ValidationError::InvalidRename(format!(
                            "Source path not observed: {}",
                            from_str
                        )));
                    }
                }
                Ok(())
            },
            ExpectedEffect::Move { changes } => {
                for change in changes {
                    let source_str = change.source.to_string_lossy();
                    if !self
                        .observed
                        .records
                        .iter()
                        .any(|record| record.path == source_str)
                    {
                        return Err(ValidationError::InvalidMove(format!(
                            "Source path not observed: {}",
                            source_str
                        )));
                    }
                }
                Ok(())
            },
            ExpectedEffect::NoOp { .. } => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(path: &str) -> FileRecord {
        FileRecord {
            path: path.to_string(),
            name: "test.txt".to_string(),
            extension: "txt".to_string(),
            size: 100,
            modified_unix: 0,
            kind: "file".to_string(),
            is_binary: Some(false),
            git_status: None,
        }
    }

    #[test]
    fn validate_duplicate_removal_success() {
        let record_a = create_test_record("a.txt");
        let record_b = create_test_record("b.txt");
        let scan_index = ScanIndex {
            root: ".".into(),
            scanned_at_unix: 0,
            total_size: 200,
            files: vec![record_a.clone(), record_b.clone()],
            issues: vec![],
        };
        let observed = ObservedState::new(scan_index);
        let group = DuplicateGroup {
            kind: "exact".into(),
            similarity: 1.0,
            items: vec!["a.txt".into(), "b.txt".into()],
            evidence: None,
        };

        let contract = OperationContract {
            observed,
            expected: ExpectedEffect::DuplicateRemoval { groups: vec![group] },
            policy: Policy::starter(),
        };

        assert!(contract.validate().is_ok());
    }

    #[test]
    fn validate_duplicate_removal_invalid_path() {
        let record_a = create_test_record("a.txt");
        let scan_index = ScanIndex {
            root: ".".into(),
            scanned_at_unix: 0,
            total_size: 100,
            files: vec![record_a],
            issues: vec![],
        };
        let observed = ObservedState::new(scan_index);
        let group = DuplicateGroup {
            kind: "exact".into(),
            similarity: 1.0,
            items: vec!["nonexistent.txt".into()],
            evidence: None,
        };

        let contract = OperationContract {
            observed,
            expected: ExpectedEffect::DuplicateRemoval { groups: vec![group] },
            policy: Policy::starter(),
        };

        assert_eq!(contract.validate(), Err(ValidationError::InvalidPath));
    }
}
