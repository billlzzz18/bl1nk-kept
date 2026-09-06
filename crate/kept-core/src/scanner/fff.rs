use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::observation::{
    ContentIdentity, Observation, Provenance, Revision, Source, SourceKind, Target,
};
use crate::scanner::types::{FileRecord, ScanIssue};
use fff_search::file_picker::{FFFMode, FilePicker, FilePickerOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FffAcquisitionMode {
    /// Inspect metadata, size, timestamps, git status, and structural summary without loading full content.
    Look,
    /// Materialize full file content into the observation body.
    View,
}

#[derive(Debug, Error)]
pub enum FffAdapterError {
    #[error("Failed to initialize FFF FilePicker for root: {root}: {reason}")]
    InitFailed { root: PathBuf, reason: String },
    #[error("Invalid root directory path: {0}")]
    InvalidRoot(PathBuf),
    #[error("File not found in FFF index or filesystem: {0}")]
    FileNotFound(String),
    #[error("Failed to read file '{path}': {reason}")]
    ReadFailed { path: PathBuf, reason: String },
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

    /// Acquire a single file as an Observation under Look or View mode
    pub fn acquire(
        &mut self,
        relative_path: &str,
        mode: FffAcquisitionMode,
    ) -> Result<Observation, FffAdapterError> {
        let normalized = relative_path.replace('\\', "/");
        let full_path = self.root.join(&normalized);

        if !full_path.exists() {
            return Err(FffAdapterError::FileNotFound(normalized));
        }

        let meta = fs::metadata(&full_path).map_err(|e| FffAdapterError::ReadFailed {
            path: full_path.clone(),
            reason: e.to_string(),
        })?;

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let size = meta.len();

        let bytes = fs::read(&full_path).map_err(|e| FffAdapterError::ReadFailed {
            path: full_path.clone(),
            reason: e.to_string(),
        })?;

        let is_binary = bytes.iter().take(8192).any(|&b| b == 0);
        let identity = ContentIdentity::from_bytes(&bytes);
        let revision = Revision::new(modified, None);

        let target = Target::File(normalized.clone());
        let source = Source {
            kind: SourceKind::File,
            adapter: "fff".to_string(),
            target: target.clone(),
            revision: revision.clone(),
            identity,
        };

        let content = match mode {
            FffAcquisitionMode::Look => None,
            FffAcquisitionMode::View => {
                if is_binary {
                    None
                } else {
                    String::from_utf8(bytes).ok()
                }
            }
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let provenance = Provenance {
            actor: "fff-adapter".to_string(),
            session_id: None,
            input_digest: None,
            timestamp,
        };

        Ok(Observation {
            id: format!("obs_{}_{}", normalized.replace('/', "_"), timestamp),
            source,
            target,
            revision,
            event: None,
            structure: None,
            evidence: Vec::new(),
            content,
            metadata: serde_json::json!({
                "size": size,
                "is_binary": is_binary,
                "modified": modified,
            }),
            provenance,
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
