//! Duplicate action handler, simulation, mutation, and rollback execution.

use clap::ValueEnum;
use dialoguer::Select;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use crate::helpers::{confirm_action, default_scan_snapshot_path, is_interactive_terminal};

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum DuplicateAction {
    Show,
    ExportPlan,
    Simulate,
    Trash,
    Delete,
    HardLink,
    Rollback,
}

pub fn choose_duplicate_action() -> anyhow::Result<Option<DuplicateAction>> {
    let choices = [
        "1. ดูรายงาน JSON อีกครั้ง",
        "2. export duplicate plan",
        "3. simulate duplicate mutation (dry-run)",
        "4. trash duplicates to backup directory",
        "5. rollback recent duplicate mutation",
        "0. จบโดยไม่เปลี่ยนไฟล์",
    ];
    let selected = Select::new()
        .with_prompt("เลือกการทำงาน (ใช้ ↑/↓ หรือ j/k แล้วกด Enter)")
        .items(&choices)
        .default(0)
        .interact_opt()?;
    Ok(selected.and_then(|position| {
        duplicate_action_from_selection(match position {
            0 => "1",
            1 => "2",
            2 => "3",
            3 => "4",
            4 => "5",
            _ => "0",
        })
    }))
}

pub fn duplicate_action_from_selection(selection: &str) -> Option<DuplicateAction> {
    match selection.trim().to_ascii_lowercase().as_str() {
        "1" | "show" => Some(DuplicateAction::Show),
        "2" | "export" | "export-plan" => Some(DuplicateAction::ExportPlan),
        "3" | "simulate" => Some(DuplicateAction::Simulate),
        "4" | "trash" => Some(DuplicateAction::Trash),
        "5" | "rollback" => Some(DuplicateAction::Rollback),
        "0" | "q" | "" => None,
        _ => None,
    }
}

pub fn run_duplicate_action(
    action: DuplicateAction,
    root: &Path,
    report: &serde_json::Value,
    yes: bool,
) -> anyhow::Result<()> {
    use kept_core::{
        create_duplicate_mutation_plan, execute_duplicate_mutation, rollback_duplicate_mutation,
        simulate_duplicate_mutation, DuplicateActionKind, DuplicateGroup, DuplicateMutationPlan,
        DuplicateMutationPolicy, RollbackJournal, ScanIndex,
    };

    let journal_file = root.join("kept-duplicate-rollback.json");

    match action {
        DuplicateAction::Show => {
            println!("{}", serde_json::to_string_pretty(report)?);
            Ok(())
        }
        DuplicateAction::ExportPlan => {
            let output = root.join("kept-duplicate-plan.json");
            if !yes && !confirm_action(&format!("บันทึก duplicate plan ไปยัง {}", output.display()))?
            {
                println!("ยกเลิกแล้ว");
                return Ok(());
            }
            std::fs::write(&output, serde_json::to_string_pretty(report)?)?;
            println!("Exported duplicate plan: {}", output.display());
            Ok(())
        }
        DuplicateAction::Simulate
        | DuplicateAction::Trash
        | DuplicateAction::Delete
        | DuplicateAction::HardLink => {
            let groups: Vec<DuplicateGroup> =
                serde_json::from_value(report["groups"].clone()).unwrap_or_default();
            let index: ScanIndex = ScanIndex {
                root: root.display().to_string(),
                scanned_at_unix: 0,
                total_size: 0,
                files: Vec::new(),
                issues: Vec::new(),
            };

            let kind = match action {
                DuplicateAction::Delete => DuplicateActionKind::Delete,
                DuplicateAction::HardLink => DuplicateActionKind::HardLink,
                _ => DuplicateActionKind::Trash,
            };

            let plan = create_duplicate_mutation_plan(&index, &groups, kind);
            let policy = DuplicateMutationPolicy {
                allowed_roots: vec![root.display().to_string()],
                protected_patterns: vec![".git".to_string(), "node_modules".to_string()],
                allow_list: Vec::new(),
                backup_directory: Some(root.join(".kept-trash").display().to_string()),
                preserve_canonical: true,
                verify_checksum_before_action: true,
            };

            let simulation = simulate_duplicate_mutation(&plan, &index, &policy);
            println!(
                "Simulation: {} action(s) valid, {} blocked. Total reclaimable: {} bytes.",
                simulation.valid_actions.len(),
                simulation.blocked_actions.len(),
                simulation.total_reclaimable_bytes
            );
            for blocked in &simulation.blocked_actions {
                println!(
                    "  [BLOCKED] {} ({})",
                    blocked.action.target_path,
                    blocked.rejection_reason.as_deref().unwrap_or("policy rule")
                );
            }

            if matches!(action, DuplicateAction::Simulate) {
                return Ok(());
            }

            if !simulation.simulation_passed {
                anyhow::bail!("Cannot apply mutation plan: safety simulation blocked some actions");
            }

            if !yes
                && !confirm_action(&format!(
                    "Apply duplicate {:?} mutation to {} file(s)",
                    kind,
                    simulation.valid_actions.len()
                ))?
            {
                println!("ยกเลิกแล้ว");
                return Ok(());
            }

            let safe_plan = DuplicateMutationPlan {
                root: plan.root.clone(),
                created_at_unix: plan.created_at_unix,
                actions: simulation.valid_actions,
                total_reclaimable_bytes: simulation.total_reclaimable_bytes,
            };

            let journal = execute_duplicate_mutation(&safe_plan, &index, &policy)?;
            std::fs::write(&journal_file, serde_json::to_string_pretty(&journal)?)?;
            println!(
                "Applied duplicate mutation. Rollback journal saved to: {}",
                journal_file.display()
            );
            Ok(())
        }
        DuplicateAction::Rollback => {
            if !journal_file.is_file() {
                anyhow::bail!("No rollback journal found at: {}", journal_file.display());
            }
            let mut journal: RollbackJournal =
                serde_json::from_str(&std::fs::read_to_string(&journal_file)?)?;
            if !yes
                && !confirm_action(&format!(
                    "Rollback {} duplicate mutation entries",
                    journal.entries.len()
                ))?
            {
                println!("ยกเลิกแล้ว");
                return Ok(());
            }
            let restored = rollback_duplicate_mutation(&mut journal)?;
            std::fs::write(&journal_file, serde_json::to_string_pretty(&journal)?)?;
            println!("Rollback complete: {restored} file(s) restored.");
            Ok(())
        }
    }
}

pub fn handle_task_duplicates(
    root: PathBuf,
    index: Option<PathBuf>,
    json: bool,
    action: Option<DuplicateAction>,
    yes: bool,
) -> anyhow::Result<()> {
    use kept_core::{find_content_duplicates, ContentDuplicateOptions, PersistentScanSnapshot};

    let canonical_root = root.canonicalize()?;
    let snapshot_path =
        index.unwrap_or_else(|| default_scan_snapshot_path(&canonical_root.display().to_string()));
    let snapshot: PersistentScanSnapshot =
        serde_json::from_str(&std::fs::read_to_string(&snapshot_path).map_err(|_| {
            anyhow::anyhow!(
                "ยังไม่มี index สำหรับ '{}'; รัน kept scan <path> ก่อน",
                canonical_root.display()
            )
        })?)?;
    if snapshot.root != canonical_root.display().to_string() {
        anyhow::bail!("index ไม่ตรงกับ root ที่ร้องขอ; รัน kept scan <path> ใหม่");
    }
    let (groups, stats) =
        find_content_duplicates(&snapshot.index, &ContentDuplicateOptions::default())?;
    let report = serde_json::json!({
        "root": snapshot.root,
        "indexPath": snapshot_path,
        "groups": groups,
        "stats": stats,
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let groups = report["groups"].as_array().map_or(&[][..], Vec::as_slice);
        println!(
            "Verified {} duplicate group(s) from the scan index.",
            groups.len()
        );
        for (position, group) in groups.iter().enumerate() {
            let kind = group["kind"].as_str().unwrap_or("unknown");
            let items = group["items"].as_array().map_or(&[][..], Vec::as_slice);
            println!("{}. {} ({} file(s))", position + 1, kind, items.len());
            for item in items {
                if let Some(path) = item.as_str() {
                    println!("   - {path}");
                }
            }
        }
        let unreadable = report["stats"]["unreadable_files"].as_u64().unwrap_or(0);
        if unreadable > 0 {
            println!("Scan issue: {unreadable} candidate file(s) could not be read for hashing.");
        }
    }

    if yes && action.is_none() {
        anyhow::bail!("--yes ต้องใช้ร่วมกับ --action ที่ระบุชัดเจน");
    }
    if let Some(action) = action {
        if !yes && !io::stdin().is_terminal() {
            anyhow::bail!("โหมด non-interactive ต้องใช้ --action <name> พร้อม --yes");
        }
        return run_duplicate_action(action, &canonical_root, &report, yes);
    }
    if !json && is_interactive_terminal() {
        if let Some(action) = choose_duplicate_action()? {
            run_duplicate_action(action, &canonical_root, &report, false)?;
        }
    }
    Ok(())
}
