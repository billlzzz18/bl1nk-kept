//! Directory traversal and scan operations.

use super::types::{to_unix, unix_now, FileRecord, ScanIndex, ScanIssue, ScanOptions};
use std::fs;
use std::path::Path;

pub fn scan_directory(root: impl AsRef<Path>, options: &ScanOptions) -> std::io::Result<ScanIndex> {
    let root = root.as_ref().canonicalize()?;
    let mut files = Vec::new();
    let mut issues = Vec::new();
    visit_directory(&root, &root, options, 0, &mut files, &mut issues);
    let total_size = files.iter().map(|file| file.size).sum();
    Ok(ScanIndex {
        root: root.display().to_string(),
        scanned_at_unix: unix_now(),
        total_size,
        files,
        issues,
    })
}

fn visit_directory(
    root: &Path,
    current: &Path,
    options: &ScanOptions,
    depth: usize,
    files: &mut Vec<FileRecord>,
    issues: &mut Vec<ScanIssue>,
) {
    if options.max_depth.is_some_and(|max| depth > max) {
        return;
    }

    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(error) => {
            record_scan_issue(issues, current, "read_dir", error);
            return;
        }
    };
    for item in entries {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                record_scan_issue(issues, current, "read_dir_entry", error);
                continue;
            }
        };
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();
        if !options.include_hidden && name.starts_with('.') {
            continue;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                record_scan_issue(issues, &path, "metadata", error);
                continue;
            }
        };
        if metadata.is_dir() {
            visit_directory(root, &path, options, depth + 1, files, issues);
            continue;
        }
        if !metadata.is_file() {
            continue;
        }

        let relative = path.strip_prefix(root).unwrap_or(&path);
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let modified_unix = match metadata.modified() {
            Ok(modified) => to_unix(modified),
            Err(error) => {
                record_scan_issue(issues, &path, "modified", error);
                0
            }
        };
        files.push(FileRecord {
            path: relative.display().to_string(),
            name,
            extension,
            size: metadata.len(),
            modified_unix,
            kind: "file".to_string(),
        });
    }
}

fn record_scan_issue(
    issues: &mut Vec<ScanIssue>,
    path: &Path,
    operation: &str,
    error: std::io::Error,
) {
    issues.push(ScanIssue {
        path: path.display().to_string(),
        operation: operation.to_string(),
        message: error.to_string(),
    });
}
