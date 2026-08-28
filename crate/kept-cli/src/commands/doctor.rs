//! Doctor command handler for environment, configuration, and editor diagnostics.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::setup::{setup_user_config_at, SetupResult};
use crate::helpers::{editor_is_available, resolve_config_editor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    Ok,
    Error,
}

#[derive(Debug)]
pub struct DoctorFinding {
    pub code: &'static str,
    pub status: DoctorStatus,
    pub message: String,
    pub remediation: &'static str,
}

#[derive(Debug)]
pub struct DoctorReport {
    pub findings: Vec<DoctorFinding>,
}

impl DoctorReport {
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.status == DoctorStatus::Error)
    }
}

pub fn doctor_editor_at(editor: OsString) -> DoctorReport {
    if editor_is_available(&editor) {
        DoctorReport {
            findings: vec![DoctorFinding {
                code: "CONFIG_EDITOR_AVAILABLE",
                status: DoctorStatus::Ok,
                message: format!("config editor ใช้งานได้: '{}'", Path::new(&editor).display()),
                remediation: "No action required",
            }],
        }
    } else {
        DoctorReport {
            findings: vec![DoctorFinding {
                code: "CONFIG_EDITOR_MISSING",
                status: DoctorStatus::Error,
                message: format!(
                    "config editor ไม่พบ: '{}'",
                    Path::new(&editor).display()
                ),
                remediation: "Install nano with your OS package manager, or set KEPT_EDITOR to an editor executable path",
            }],
        }
    }
}

pub fn repair_missing_config_at(path: &Path) -> anyhow::Result<SetupResult> {
    if path.exists() {
        anyhow::bail!(
            "ไม่สามารถ fix config ที่มีอยู่ด้วยการเขียนทับ: '{}'; ใช้ kept config edit หรือ backup ไฟล์ก่อน",
            path.display()
        );
    }
    setup_user_config_at(path)
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidConfigRepair {
    pub backup_path: PathBuf,
}

pub fn repair_invalid_config_at(path: &Path) -> anyhow::Result<InvalidConfigRepair> {
    if !path.is_file() {
        anyhow::bail!("ไม่พบ config ที่ '{}' สำหรับ repair", path.display());
    }
    if kept_core::load_user_config(path).is_ok() {
        anyhow::bail!("config ที่ '{}' ใช้งานได้อยู่แล้ว; ไม่ต้อง repair", path.display());
    }
    let backup_path = next_invalid_config_backup_path(path)?;
    std::fs::rename(path, &backup_path)?;
    setup_user_config_at(path)?;
    Ok(InvalidConfigRepair { backup_path })
}

pub fn next_invalid_config_backup_path(path: &Path) -> anyhow::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("config path has no parent: {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("config path has no valid file name: {}", path.display()))?;
    for ordinal in 0_u32.. {
        let suffix = if ordinal == 0 {
            ".invalid.bak".to_string()
        } else {
            format!(".invalid.{ordinal}.bak")
        };
        let candidate = parent.join(format!("{file_name}{suffix}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("could not allocate backup path for {}", path.display())
}

pub fn doctor_config_at(path: &Path) -> DoctorReport {
    if !path.exists() {
        return DoctorReport {
            findings: vec![DoctorFinding {
                code: "CONFIG_MISSING",
                status: DoctorStatus::Error,
                message: format!("ไม่พบ config.yaml ที่ '{}'", path.display()),
                remediation:
                    "Run `kept setup` or `kept doctor --fix` to create the default configuration",
            }],
        };
    }

    match kept_core::load_user_config(path) {
        Ok(_) => DoctorReport {
            findings: vec![DoctorFinding {
                code: "CONFIG_VALID",
                status: DoctorStatus::Ok,
                message: format!("config ใช้งานได้: '{}'", path.display()),
                remediation: "No action required",
            }],
        },
        Err(error) => DoctorReport {
            findings: vec![DoctorFinding {
                code: "CONFIG_INVALID",
                status: DoctorStatus::Error,
                message: format!("config ที่ '{}' ไม่ถูกต้อง: {error}", path.display()),
                remediation: "Edit the configuration using `kept config edit` or reset via `kept doctor --fix`",
            }],
        },
    }
}

pub fn with_editor_diagnostic(
    mut report: DoctorReport,
    explicit_editor: Option<OsString>,
) -> DoctorReport {
    let editor = resolve_config_editor(explicit_editor);
    let editor_report = doctor_editor_at(editor);
    report.findings.extend(editor_report.findings);
    report
}

#[derive(Debug)]
pub struct DoctorRunResult {
    pub report: DoctorReport,
    pub fixed: bool,
}

pub fn run_doctor_at(
    path: &Path,
    fix: bool,
    explicit_editor: Option<OsString>,
) -> anyhow::Result<DoctorRunResult> {
    let initial_report = with_editor_diagnostic(doctor_config_at(path), explicit_editor.clone());
    if !fix {
        return Ok(DoctorRunResult {
            report: initial_report,
            fixed: false,
        });
    }

    let is_missing = initial_report
        .findings
        .iter()
        .any(|finding| finding.code == "CONFIG_MISSING");
    let is_invalid = initial_report
        .findings
        .iter()
        .any(|finding| finding.code == "CONFIG_INVALID");

    let fixed = if is_missing {
        repair_missing_config_at(path)?;
        true
    } else if is_invalid {
        repair_invalid_config_at(path)?;
        true
    } else {
        false
    };

    let final_report = with_editor_diagnostic(doctor_config_at(path), explicit_editor);
    Ok(DoctorRunResult {
        report: final_report,
        fixed,
    })
}

pub fn handle_doctor(fix: bool) -> anyhow::Result<()> {
    let path = kept_core::default_user_config_path()?;
    let result = run_doctor_at(&path, fix, None)?;

    if result.fixed {
        println!("Repaired config: '{}'", path.display());
    }

    for finding in &result.report.findings {
        let status = match finding.status {
            DoctorStatus::Ok => "OK",
            DoctorStatus::Error => "ERROR",
        };
        println!("{status} {}: {}", finding.code, finding.message);
        println!("  Fix: {}", finding.remediation);
    }

    if result.report.has_errors() {
        anyhow::bail!("doctor found configuration problems")
    }

    Ok(())
}
