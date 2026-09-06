//! Directory traversal and scan operations backed by FFF filesystem engine.

use super::fff::FffScanner;
use super::types::{unix_now, FileRecord, ScanIndex, ScanIssue, ScanOptions};
use std::fs;
use std::path::Path;

pub fn scan_directory(root: impl AsRef<Path>, options: &ScanOptions) -> std::io::Result<ScanIndex> {
    let root_canonical = root.as_ref().canonicalize()?;

    let (mut files, issues) = if options.include_hidden {
        // Fallback or explicit traversal when hidden files/internal directories (like .git in tests) are explicitly requested
        let mut f_list = Vec::new();
        let mut i_list = Vec::new();
        visit_dir_all(
            &root_canonical,
            &root_canonical,
            0,
            options,
            &mut f_list,
            &mut i_list,
        );
        (f_list, i_list)
    } else {
        let mut scanner = FffScanner::new(&root_canonical)
            .map_err(|err| std::io::Error::other(err.to_string()))?;

        let (mut f_list, i_list) = scanner
            .scan_inventory()
            .map_err(|err| std::io::Error::other(err.to_string()))?;

        f_list.retain(|f| {
            let p = &f.path;
            !p.starts_with('.') && !p.contains("/.") && !p.contains("\\.")
        });
        (f_list, i_list)
    };

    if let Some(max_depth) = options.max_depth {
        files.retain(|f| {
            let depth = f.path.matches('/').count() + f.path.matches('\\').count();
            depth <= max_depth
        });
    }

    // Deterministic sorting by relative path ascending
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let total_size = files.iter().map(|file| file.size).sum();
    Ok(ScanIndex {
        root: root_canonical.display().to_string(),
        scanned_at_unix: unix_now(),
        total_size,
        files,
        issues,
    })
}

fn visit_dir_all(
    root: &Path,
    current: &Path,
    depth: usize,
    options: &ScanOptions,
    files: &mut Vec<FileRecord>,
    issues: &mut Vec<ScanIssue>,
) {
    if options.max_depth.is_some_and(|max| depth > max) {
        return;
    }

    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(error) => {
            issues.push(ScanIssue::new(
                current.display().to_string(),
                "read_dir".to_string(),
                error.to_string(),
            ));
            return;
        }
    };

    for item in entries {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                issues.push(ScanIssue::new(
                    current.display().to_string(),
                    "read_dir_entry".to_string(),
                    error.to_string(),
                ));
                continue;
            }
        };
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();

        let relative = path.strip_prefix(root).unwrap_or(&path);
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                issues.push(ScanIssue::new(
                    path.display().to_string(),
                    "metadata".to_string(),
                    error.to_string(),
                ));
                continue;
            }
        };

        if metadata.is_dir() {
            visit_dir_all(root, &path, depth + 1, options, files, issues);
            continue;
        }

        if !metadata.is_file() {
            continue;
        }

        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let modified_unix = match metadata.modified() {
            Ok(modified) => crate::scanner::types::to_unix(modified),
            Err(error) => {
                issues.push(ScanIssue::new(
                    path.display().to_string(),
                    "modified".to_string(),
                    error.to_string(),
                ));
                0
            }
        };

        let mut buf = [0u8; 1024];
        let is_binary = if let Ok(mut f) = fs::File::open(&path) {
            use std::io::Read;
            if let Ok(n) = f.read(&mut buf) {
                buf[..n].contains(&0)
            } else {
                false
            }
        } else {
            false
        };

        files.push(FileRecord {
            path: relative_str,
            name,
            extension,
            size: metadata.len(),
            modified_unix,
            kind: "file".to_string(),
            is_binary: Some(is_binary),
            git_status: None,
        });
    }
}
