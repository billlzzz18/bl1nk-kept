use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::scanner::types::{FileRecord, ScanIssue};
use fff_search::file_picker::{FFFMode, FilePicker, FilePickerOptions};

#[derive(Debug, Error)]
pub enum FffAdapterError {
    #[error("Failed to initialize FFF FilePicker for root: {root}: {reason}")]
    InitFailed { root: PathBuf, reason: String },
    #[error("Invalid root directory path: {0}")]
    InvalidRoot(PathBuf),
    #[error("FFF scan timeout after {0} seconds")]
    ScanTimeout(u64),
    #[error("FFF index is not ready")]
    IndexNotReady,
}

pub struct FffScanner {
    root: PathBuf,
    picker: FilePicker,
}

impl FffScanner {
    /// Initialize a synchronous FFF FilePicker in AI mode
    pub fn new(root: impl AsRef<Path>) -> Result<Self, FffAdapterError> {
        let root_buf = root.as_ref().to_path_buf();
        if !root_buf.exists() || !root_buf.is_dir() {
            return Err(FffAdapterError::InvalidRoot(root_buf));
        }

        let options = FilePickerOptions {
            base_path: root_buf.to_string_lossy().to_string(),
            mode: FFFMode::Ai,
            ..Default::default()
        };

        let picker = FilePicker::new(options).map_err(|e| FffAdapterError::InitFailed {
            root: root_buf.clone(),
            reason: e.to_string(),
        })?;

        Ok(Self {
            root: root_buf,
            picker,
        })
    }

    /// Perform file inventory collection and map to deterministic FileRecord items
    pub fn scan_inventory(&mut self) -> Result<(Vec<FileRecord>, Vec<ScanIssue>), FffAdapterError> {
        self.picker
            .collect_files()
            .map_err(|e| FffAdapterError::InitFailed {
                root: self.root.clone(),
                reason: e.to_string(),
            })?;

        let mut records = Vec::new();
        let issues = Vec::new();

        let files = self.picker.get_files();

        for file_item in files {
            let rel_path = file_item.relative_path(&self.picker);
            let path_obj = Path::new(&rel_path);

            // Filter out default ignored directories if not caught by FFF internal rules
            let norm_path = rel_path.replace('\\', "/");
            let ignored_segments = [
                "node_modules/",
                "venv/",
                ".venv/",
                "__pycache__/",
                "target/",
            ];
            if ignored_segments
                .iter()
                .any(|seg| norm_path.starts_with(seg) || norm_path.contains(&format!("/{seg}")))
            {
                continue;
            }

            let name = file_item.file_name(&self.picker);
            let extension = path_obj
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();

            let git_status_str = file_item.git_status.as_ref().map(|s| {
                let status_debug = format!("{:?}", s);
                if status_debug.contains("MODIFIED") {
                    "modified".to_string()
                } else if status_debug.contains("NEW") || status_debug.contains("WT_NEW") {
                    "untracked".to_string()
                } else if status_debug.contains("DELETED") || status_debug.contains("WT_DELETED") {
                    "deleted".to_string()
                } else if status_debug.contains("RENAMED") || status_debug.contains("WT_RENAMED") {
                    "renamed".to_string()
                } else {
                    "modified".to_string()
                }
            });
            let is_binary_flag = Some(file_item.is_binary());

            let kind = if file_item.is_binary() {
                "binary".to_string()
            } else {
                "file".to_string()
            };

            records.push(FileRecord {
                path: rel_path,
                name,
                extension,
                size: file_item.size,
                modified_unix: file_item.modified,
                kind,
                is_binary: is_binary_flag,
                git_status: git_status_str,
            });
        }

        // Deterministic path sort
        records.sort_by(|a, b| a.path.cmp(&b.path));

        Ok((records, issues))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
