//! File-type magic signature and bad extension detection.

use super::types::{ScanIndex, ScanIssue, ScanIssueKind};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IntegrityStatus {
    Valid,
    Mismatch {
        detected_extension: String,
        actual_extension: String,
    },
    CorruptOrEmpty,
    UnknownSignature,
}

#[derive(Debug, Clone, Copy)]
pub struct MagicRule {
    pub ext: &'static str,
    pub offset: usize,
    pub magic: &'static [u8],
}

pub static MAGIC_RULES: &[MagicRule] = &[
    // Images
    MagicRule {
        ext: "png",
        offset: 0,
        magic: &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
    },
    MagicRule {
        ext: "jpg",
        offset: 0,
        magic: &[0xFF, 0xD8, 0xFF],
    },
    MagicRule {
        ext: "gif",
        offset: 0,
        magic: b"GIF87a",
    },
    MagicRule {
        ext: "gif",
        offset: 0,
        magic: b"GIF89a",
    },
    MagicRule {
        ext: "webp",
        offset: 8,
        magic: b"WEBP",
    },
    // Documents & Archives
    MagicRule {
        ext: "pdf",
        offset: 0,
        magic: b"%PDF-",
    },
    MagicRule {
        ext: "zip",
        offset: 0,
        magic: &[0x50, 0x4B, 0x03, 0x04],
    }, // docx, xlsx, pptx, jar, zip
    MagicRule {
        ext: "gz",
        offset: 0,
        magic: &[0x1F, 0x8B],
    },
    MagicRule {
        ext: "7z",
        offset: 0,
        magic: &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C],
    },
    MagicRule {
        ext: "tar",
        offset: 257,
        magic: b"ustar",
    },
];

pub fn detect_extension_from_bytes(buf: &[u8]) -> Option<&'static str> {
    for rule in MAGIC_RULES {
        let end = rule.offset + rule.magic.len();
        if buf.len() >= end && &buf[rule.offset..end] == rule.magic {
            return Some(rule.ext);
        }
    }
    None
}

pub fn check_file_extension_integrity(path: &Path) -> io::Result<IntegrityStatus> {
    let mut file = File::open(path)?;
    let mut buf = [0u8; 512];
    let n = file.read(&mut buf)?;
    if n == 0 {
        return Ok(IntegrityStatus::CorruptOrEmpty);
    }

    let actual_ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if let Some(detected) = detect_extension_from_bytes(&buf[..n]) {
        let matches = match (detected, actual_ext.as_str()) {
            ("jpg", "jpeg") | ("jpg", "jpg") => true,
            ("zip", "docx")
            | ("zip", "xlsx")
            | ("zip", "pptx")
            | ("zip", "jar")
            | ("zip", "apk")
            | ("zip", "zip") => true,
            ("tar", "tar") => true,
            (expected, actual) => expected == actual,
        };

        if matches {
            Ok(IntegrityStatus::Valid)
        } else {
            Ok(IntegrityStatus::Mismatch {
                detected_extension: detected.to_string(),
                actual_extension: actual_ext,
            })
        }
    } else {
        Ok(IntegrityStatus::UnknownSignature)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BadExtensionIssue {
    pub path: String,
    pub actual_extension: String,
    pub detected_extension: String,
}

pub fn scan_index_integrity(index: &ScanIndex) -> (Vec<BadExtensionIssue>, Vec<ScanIssue>) {
    let root_path = Path::new(&index.root);
    let mut bad_extensions = Vec::new();
    let mut scan_issues = Vec::new();

    for record in &index.files {
        let full_path = if Path::new(&record.path).is_absolute() {
            Path::new(&record.path).to_path_buf()
        } else {
            root_path.join(&record.path)
        };

        match check_file_extension_integrity(&full_path) {
            Ok(IntegrityStatus::Mismatch {
                detected_extension,
                actual_extension,
            }) => {
                bad_extensions.push(BadExtensionIssue {
                    path: record.path.clone(),
                    actual_extension,
                    detected_extension,
                });
            }
            Ok(IntegrityStatus::CorruptOrEmpty) => {
                scan_issues.push(ScanIssue {
                    path: record.path.clone(),
                    operation: "integrity_check".to_string(),
                    kind: ScanIssueKind::CorruptData,
                    message: "File is 0 bytes or corrupt".to_string(),
                    remediation: Some(
                        "Verify file source; empty or incomplete download.".to_string(),
                    ),
                });
            }
            Ok(IntegrityStatus::Valid) | Ok(IntegrityStatus::UnknownSignature) => {}
            Err(e) => {
                scan_issues.push(ScanIssue::new(record.path.clone(), "read_header", e.to_string()));
            }
        }
    }

    (bad_extensions, scan_issues)
}
