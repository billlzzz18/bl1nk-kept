//! Duplicate mutation policy, simulation, execution, and rollback.

use super::duplicate::{full_hash, hash_hex, DuplicateGroup};
use super::types::{unix_now, ScanIndex};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateActionKind {
    Trash,
    Delete,
    HardLink,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateMutationPolicy {
    #[serde(rename = "allowedRoots")]
    pub allowed_roots: Vec<String>,
    #[serde(rename = "protectedPatterns", default = "default_protected_patterns")]
    pub protected_patterns: Vec<String>,
    #[serde(
        rename = "backupDirectory",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_directory: Option<String>,
    #[serde(rename = "preserveCanonical", default = "default_true")]
    pub preserve_canonical: bool,
    #[serde(rename = "verifyChecksumBeforeAction", default = "default_true")]
    pub verify_checksum_before_action: bool,
}

fn default_true() -> bool {
    true
}

fn default_protected_patterns() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".svn".to_string(),
        ".hg".to_string(),
        "node_modules".to_string(),
        ".kept".to_string(),
    ]
}

impl Default for DuplicateMutationPolicy {
    fn default() -> Self {
        Self {
            allowed_roots: Vec::new(),
            protected_patterns: default_protected_patterns(),
            backup_directory: None,
            preserve_canonical: true,
            verify_checksum_before_action: true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicatePlanAction {
    pub target_path: String,
    pub canonical_path: String,
    pub action: DuplicateActionKind,
    pub expected_size: u64,
    pub expected_sha256: String,
    pub reclaimed_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateMutationPlan {
    pub root: String,
    pub created_at_unix: u64,
    pub actions: Vec<DuplicatePlanAction>,
    pub total_reclaimable_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionSimulation {
    pub action: DuplicatePlanAction,
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateSimulationResult {
    pub root: String,
    pub valid_actions: Vec<DuplicatePlanAction>,
    pub blocked_actions: Vec<ActionSimulation>,
    pub total_reclaimable_bytes: u64,
    pub simulation_passed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RollbackEntry {
    pub target_path: String,
    pub action_performed: DuplicateActionKind,
    pub backup_path: Option<String>,
    pub restored: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RollbackJournal {
    pub root: String,
    pub executed_at_unix: u64,
    pub entries: Vec<RollbackEntry>,
}

pub fn create_duplicate_mutation_plan(
    index: &ScanIndex,
    groups: &[DuplicateGroup],
    action_kind: DuplicateActionKind,
) -> DuplicateMutationPlan {
    let mut actions = Vec::new();
    let mut total_reclaimable_bytes = 0;

    for group in groups {
        if group.items.len() < 2 {
            continue;
        }
        let canonical = &group.items[0];
        let size = group
            .evidence
            .as_ref()
            .map(|e| e.size_bytes)
            .unwrap_or_else(|| {
                index
                    .files
                    .iter()
                    .find(|f| &f.path == canonical)
                    .map(|f| f.size)
                    .unwrap_or(0)
            });
        let sha256 = group
            .evidence
            .as_ref()
            .map(|e| e.full_sha256.clone())
            .unwrap_or_default();

        for duplicate in &group.items[1..] {
            actions.push(DuplicatePlanAction {
                target_path: duplicate.clone(),
                canonical_path: canonical.clone(),
                action: action_kind,
                expected_size: size,
                expected_sha256: sha256.clone(),
                reclaimed_bytes: size,
            });
            total_reclaimable_bytes += size;
        }
    }

    DuplicateMutationPlan {
        root: index.root.clone(),
        created_at_unix: unix_now(),
        actions,
        total_reclaimable_bytes,
    }
}

pub fn simulate_duplicate_mutation(
    plan: &DuplicateMutationPlan,
    index: &ScanIndex,
    policy: &DuplicateMutationPolicy,
) -> DuplicateSimulationResult {
    let mut valid_actions = Vec::new();
    let mut blocked_actions = Vec::new();
    let mut total_reclaimable_bytes = 0;

    let root_path = Path::new(&plan.root);

    for action in &plan.actions {
        let target_full = if Path::new(&action.target_path).is_absolute() {
            PathBuf::from(&action.target_path)
        } else {
            root_path.join(&action.target_path)
        };

        // 1. Root boundary check
        let is_within_root = target_full.starts_with(root_path)
            || policy
                .allowed_roots
                .iter()
                .any(|allowed| target_full.starts_with(Path::new(allowed)));

        if !is_within_root {
            blocked_actions.push(ActionSimulation {
                action: action.clone(),
                allowed: false,
                rejection_reason: Some(format!(
                    "Path '{}' is outside allowed root '{}'",
                    action.target_path, plan.root
                )),
            });
            continue;
        }

        // 2. Protected pattern check
        let has_protected_pattern = policy.protected_patterns.iter().any(|pattern| {
            action
                .target_path
                .split(['/', '\\'])
                .any(|segment| segment == pattern)
        });

        if has_protected_pattern {
            blocked_actions.push(ActionSimulation {
                action: action.clone(),
                allowed: false,
                rejection_reason: Some(format!(
                    "Path '{}' contains protected directory pattern",
                    action.target_path
                )),
            });
            continue;
        }

        // 3. Stale index check against memory index
        let record = index.files.iter().find(|f| f.path == action.target_path);
        if let Some(record) = record {
            if record.size != action.expected_size {
                blocked_actions.push(ActionSimulation {
                    action: action.clone(),
                    allowed: false,
                    rejection_reason: Some(format!(
                        "File size mismatch: index has {} bytes, plan expects {} bytes",
                        record.size, action.expected_size
                    )),
                });
                continue;
            }
        }

        // 4. File existence & on-disk verification
        if target_full.is_file() {
            if let Ok(metadata) = fs::metadata(&target_full) {
                if metadata.len() != action.expected_size {
                    blocked_actions.push(ActionSimulation {
                        action: action.clone(),
                        allowed: false,
                        rejection_reason: Some(format!(
                            "On-disk file size {} differs from plan expectation {}",
                            metadata.len(),
                            action.expected_size
                        )),
                    });
                    continue;
                }
            }
        }

        valid_actions.push(action.clone());
        total_reclaimable_bytes += action.reclaimed_bytes;
    }

    let simulation_passed = blocked_actions.is_empty();

    DuplicateSimulationResult {
        root: plan.root.clone(),
        valid_actions,
        blocked_actions,
        total_reclaimable_bytes,
        simulation_passed,
    }
}

pub fn execute_duplicate_mutation(
    plan: &DuplicateMutationPlan,
    index: &ScanIndex,
    policy: &DuplicateMutationPolicy,
) -> std::io::Result<RollbackJournal> {
    let simulation = simulate_duplicate_mutation(plan, index, policy);
    if !simulation.simulation_passed {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "Mutation plan failed safety simulation: {} action(s) blocked",
                simulation.blocked_actions.len()
            ),
        ));
    }

    let root_path = Path::new(&plan.root);
    let mut journal_entries = Vec::new();

    let backup_dir = policy
        .backup_directory
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir()
                .join("kept")
                .join("trash")
                .join(format!("{:x}", unix_now()))
        });

    for action in &simulation.valid_actions {
        let target_full = if Path::new(&action.target_path).is_absolute() {
            PathBuf::from(&action.target_path)
        } else {
            root_path.join(&action.target_path)
        };
        let canonical_full = if Path::new(&action.canonical_path).is_absolute() {
            PathBuf::from(&action.canonical_path)
        } else {
            root_path.join(&action.canonical_path)
        };

        if !target_full.exists() {
            continue;
        }

        if policy.verify_checksum_before_action && !action.expected_sha256.is_empty() {
            let actual_hash = full_hash(&target_full)?;
            if hash_hex(&actual_hash) != action.expected_sha256 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "File '{}' modified since duplicate index creation; aborting mutation",
                        action.target_path
                    ),
                ));
            }
        }

        match action.action {
            DuplicateActionKind::Trash => {
                fs::create_dir_all(&backup_dir)?;
                let relative_target = target_full.strip_prefix(root_path).unwrap_or(&target_full);
                let backup_file = backup_dir.join(relative_target);
                if let Some(parent) = backup_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&target_full, &backup_file)?;
                journal_entries.push(RollbackEntry {
                    target_path: action.target_path.clone(),
                    action_performed: DuplicateActionKind::Trash,
                    backup_path: Some(backup_file.display().to_string()),
                    restored: false,
                });
            }
            DuplicateActionKind::Delete => {
                fs::create_dir_all(&backup_dir)?;
                let relative_target = target_full.strip_prefix(root_path).unwrap_or(&target_full);
                let backup_file = backup_dir.join(relative_target);
                if let Some(parent) = backup_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&target_full, &backup_file)?;
                fs::remove_file(&target_full)?;
                journal_entries.push(RollbackEntry {
                    target_path: action.target_path.clone(),
                    action_performed: DuplicateActionKind::Delete,
                    backup_path: Some(backup_file.display().to_string()),
                    restored: false,
                });
            }
            DuplicateActionKind::HardLink => {
                let temp_link =
                    target_full.with_extension(format!("kept-tmp-{}", unix_now() % 1_000_000));
                fs::hard_link(&canonical_full, &temp_link)?;
                fs::rename(&temp_link, &target_full)?;
                journal_entries.push(RollbackEntry {
                    target_path: action.target_path.clone(),
                    action_performed: DuplicateActionKind::HardLink,
                    backup_path: None,
                    restored: false,
                });
            }
        }
    }

    Ok(RollbackJournal {
        root: plan.root.clone(),
        executed_at_unix: unix_now(),
        entries: journal_entries,
    })
}

pub fn rollback_duplicate_mutation(journal: &mut RollbackJournal) -> std::io::Result<usize> {
    let root_path = Path::new(&journal.root);
    let mut restored_count = 0;

    for entry in &mut journal.entries {
        if entry.restored {
            continue;
        }
        let target_full = if Path::new(&entry.target_path).is_absolute() {
            PathBuf::from(&entry.target_path)
        } else {
            root_path.join(&entry.target_path)
        };

        if let Some(backup_path_str) = &entry.backup_path {
            let backup_path = Path::new(backup_path_str);
            if backup_path.exists() {
                if let Some(parent) = target_full.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(backup_path, &target_full)?;
                entry.restored = true;
                restored_count += 1;
            }
        }
    }

    Ok(restored_count)
}
