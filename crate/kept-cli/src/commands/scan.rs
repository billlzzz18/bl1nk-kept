//! Scan, find, and interactive review command handlers.

use dialoguer::Select;
use std::path::{Path, PathBuf};

use super::duplicates::handle_task_duplicates;
use crate::helpers::{
    default_scan_snapshot_path, is_interactive_terminal, parse_human_size, prompt_optional,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanReviewAction {
    Space,
    Find,
    Duplicates,
    Integrity,
    Issues,
    Naming,
}

#[derive(Debug)]
pub struct TaskScanResult {
    pub root: String,
    pub index_path: PathBuf,
    pub file_count: usize,
    pub total_size: u64,
    pub issue_count: usize,
    pub refresh: Option<kept_core::RefreshPlan>,
}

#[derive(Debug)]
pub struct FindRequest {
    pub root: PathBuf,
    pub query: Option<String>,
    pub explain: bool,
    pub file_type: Option<String>,
    pub name: Option<String>,
    pub path_contains: Option<String>,
    pub min_size: Option<String>,
    pub max_size: Option<String>,
    pub after: Option<u64>,
    pub before: Option<u64>,
    pub index: Option<PathBuf>,
    pub json: bool,
}

pub fn handle_task_scan(
    root: PathBuf,
    output: Option<PathBuf>,
    include_hidden: bool,
    json: bool,
) -> anyhow::Result<()> {
    let result = create_or_refresh_scan(&root, output, include_hidden)?;
    if json {
        let payload = serde_json::json!({
            "root": result.root,
            "indexPath": result.index_path,
            "fileCount": result.file_count,
            "totalSize": result.total_size,
            "issueCount": result.issue_count,
            "refresh": result.refresh,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "Indexed {} files ({} bytes, {} issues) at {}",
            result.file_count,
            result.total_size,
            result.issue_count,
            result.index_path.display()
        );
        if let Some(refresh) = &result.refresh {
            println!(
                "Refresh: +{} added, ~{} modified, -{} removed, {} unchanged.",
                refresh.added.len(),
                refresh.modified.len(),
                refresh.removed.len(),
                refresh.unchanged.len()
            );
        }
    }
    if !json && is_interactive_terminal() {
        handle_task_review(root, Some(result.index_path.clone()), true)?;
    }
    Ok(())
}

pub fn create_or_refresh_scan(
    root: &Path,
    output: Option<PathBuf>,
    include_hidden: bool,
) -> anyhow::Result<TaskScanResult> {
    use kept_core::{
        create_persistent_snapshot, plan_incremental_refresh, scan_directory,
        PersistentScanSnapshot, ScanOptions,
    };

    let options = ScanOptions {
        include_hidden,
        max_depth: None,
    };
    let index = scan_directory(root, &options)?;
    let output = output.unwrap_or_else(|| default_scan_snapshot_path(&index.root));
    let previous = if output.is_file() {
        Some(serde_json::from_str::<PersistentScanSnapshot>(&std::fs::read_to_string(&output)?)?)
    } else {
        None
    };
    let refresh = previous
        .as_ref()
        .filter(|snapshot| {
            snapshot.root == index.root
                && snapshot.include_hidden == options.include_hidden
                && snapshot.max_depth == options.max_depth
        })
        .map(|snapshot| plan_incremental_refresh(&snapshot.index, &index))
        .transpose()
        .map_err(anyhow::Error::msg)?;
    let snapshot = create_persistent_snapshot(index, &options);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, serde_json::to_string_pretty(&snapshot)?)?;

    Ok(TaskScanResult {
        root: snapshot.root,
        index_path: output,
        file_count: snapshot.index.files.len(),
        total_size: snapshot.index.total_size,
        issue_count: snapshot.index.issues.len(),
        refresh,
    })
}

pub fn handle_task_find(request: FindRequest) -> anyhow::Result<()> {
    use kept_core::{compile_query, filter_index, FileFilter, FilterSet, PersistentScanSnapshot};

    if let Some(q) = &request.query {
        let (compiled_filters, plan) = compile_query(q).map_err(anyhow::Error::msg)?;
        if request.explain {
            if request.json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                println!("Query: '{}'", plan.raw_query);
                println!("Explanation:");
                for step in &plan.explanation {
                    println!("  - {}", step);
                }
            }
            return Ok(());
        }

        let canonical_root = request.root.canonicalize()?;
        let snapshot_path = request
            .index
            .unwrap_or_else(|| default_scan_snapshot_path(&canonical_root.display().to_string()));
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

        let records = filter_index(&snapshot.index, &compiled_filters);
        if request.json {
            println!("{}", serde_json::to_string_pretty(&records)?);
        } else {
            for record in &records {
                println!("{}\t{}\t{}", record.size, record.modified_unix, record.path);
            }
            println!("Matched {} of {} file(s).", records.len(), snapshot.index.files.len());
        }
        return Ok(());
    }

    let canonical_root = request.root.canonicalize()?;
    let snapshot_path = request
        .index
        .unwrap_or_else(|| default_scan_snapshot_path(&canonical_root.display().to_string()));
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

    let mut filters = Vec::new();
    if let Some(file_type) = request.file_type {
        filters.push(FileFilter::Extension(file_type));
    }
    if let Some(name) = request.name {
        filters.push(FileFilter::NameContains(name));
    }
    if let Some(path) = request.path_contains {
        filters.push(FileFilter::PathContains(path));
    }
    if let Some(min_size) = request.min_size {
        filters.push(FileFilter::MinSize(parse_human_size(&min_size)?));
    }
    if let Some(max_size) = request.max_size {
        filters.push(FileFilter::MaxSize(parse_human_size(&max_size)?));
    }
    if let Some(after) = request.after {
        filters.push(FileFilter::ModifiedAfter(after));
    }
    if let Some(before) = request.before {
        filters.push(FileFilter::ModifiedBefore(before));
    }

    let records = filter_index(
        &snapshot.index,
        &FilterSet {
            all: filters,
            any: Vec::new(),
            custom: Vec::new(),
        },
    );

    if request.json {
        println!("{}", serde_json::to_string_pretty(&records)?);
    } else {
        for record in &records {
            println!("{}\t{}\t{}", record.size, record.modified_unix, record.path);
        }
        println!("Matched {} of {} file(s).", records.len(), snapshot.index.files.len());
    }
    Ok(())
}

pub fn handle_task_review(
    root: PathBuf,
    index: Option<PathBuf>,
    interactive: bool,
) -> anyhow::Result<()> {
    use kept_core::PersistentScanSnapshot;

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

    println!("{}", scan_review_summary(&snapshot.index));
    println!("{}", naming_review_summary(&snapshot.index));
    if !interactive || !is_interactive_terminal() {
        return Ok(());
    }

    loop {
        let choices = [
            "1. ดูก้อนพื้นที่ใหญ่ (Largest Space Users)",
            "2. หาไฟล์ด้วยตัวช่วยกรอง (Filter & Query)",
            "3. ตรวจ duplicate จาก index นี้ (Duplicate Candidates Queue)",
            "4. ตรวจความถูกต้องของนามสกุลไฟล์ (File Integrity & Bad Extensions)",
            "5. ดู scan issues และคำแนะนำแก้ไข (Scan Issues & Error Taxonomy)",
            "6. ตรวจ naming policy (Naming Policy Queue)",
            "0. จบ",
        ];
        let selected = Select::new()
            .with_prompt("เลือกการทำงาน (ใช้ ↑/↓ หรือ j/k แล้วกด Enter)")
            .items(choices)
            .default(0)
            .interact_opt()?;
        let action = selected.and_then(|position| {
            scan_review_action_from_selection(match position {
                0 => "1",
                1 => "2",
                2 => "3",
                3 => "4",
                4 => "5",
                5 => "6",
                _ => "0",
            })
        });
        let Some(action) = action else {
            return Ok(());
        };
        match action {
            ScanReviewAction::Space => {
                println!("Largest top-level paths:");
                for (path, size) in build_scan_space_summary(&snapshot.index)
                    .into_iter()
                    .take(20)
                {
                    println!("{size}\t{path}");
                }
            }
            ScanReviewAction::Find => run_interactive_find(canonical_root.clone())?,
            ScanReviewAction::Duplicates => handle_task_duplicates(
                canonical_root.clone(),
                Some(snapshot_path.clone()),
                false,
                None,
                false,
            )?,
            ScanReviewAction::Integrity => {
                let (bad_extensions, issues) = kept_core::scan_index_integrity(&snapshot.index);
                println!(
                    "File Integrity: {} bad extension(s) detected, {} corrupted/empty issue(s).",
                    bad_extensions.len(),
                    issues.len()
                );
                for bad in bad_extensions {
                    println!(
                        "  [BAD EXTENSION] {} (detected: {}, actual: {})",
                        bad.path, bad.detected_extension, bad.actual_extension
                    );
                }
            }
            ScanReviewAction::Issues => println!("{}", scan_review_summary(&snapshot.index)),
            ScanReviewAction::Naming => println!("{}", naming_review_summary(&snapshot.index)),
        }
    }
}

pub fn run_interactive_find(root: PathBuf) -> anyhow::Result<()> {
    let file_type = prompt_optional("ชนิดไฟล์ เช่น pdf (Enter เพื่อข้าม)")?;
    let name = prompt_optional("ชื่อไฟล์ที่ต้องมี (Enter เพื่อข้าม)")?;
    let path_contains = prompt_optional("ส่วนหนึ่งของ path (Enter เพื่อข้าม)")?;
    let min_size = prompt_optional("ขนาดต่ำสุด เช่น 50mb (Enter เพื่อข้าม)")?;
    let max_size = prompt_optional("ขนาดสูงสุด เช่น 100mb (Enter เพื่อข้าม)")?;
    let after = prompt_optional("modified หลัง Unix time (Enter เพื่อข้าม)")?
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| anyhow::anyhow!("ค่า modified-after ต้องเป็น Unix time"))?;
    let before = prompt_optional("modified ก่อน Unix time (Enter เพื่อข้าม)")?
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| anyhow::anyhow!("ค่า modified-before ต้องเป็น Unix time"))?;
    handle_task_find(FindRequest {
        root,
        query: None,
        explain: false,
        file_type,
        name,
        path_contains,
        min_size,
        max_size,
        after,
        before,
        index: None,
        json: false,
    })
}

pub fn build_scan_space_summary(index: &kept_core::ScanIndex) -> Vec<(String, u64)> {
    let mut sizes = std::collections::BTreeMap::<String, u64>::new();
    for record in &index.files {
        let segment = record.path.split('/').next().unwrap_or(&record.path);
        *sizes.entry(segment.to_string()).or_default() += record.size;
    }
    let mut summary = sizes.into_iter().collect::<Vec<_>>();
    summary.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    summary
}

pub fn naming_review_summary(index: &kept_core::ScanIndex) -> String {
    match kept_core::default_user_config_path() {
        Ok(path) => naming_review_summary_at(index, &path),
        Err(error) => format!(
            "Naming policy unavailable: cannot resolve config path ({error}); run kept doctor"
        ),
    }
}

pub fn naming_review_summary_at(index: &kept_core::ScanIndex, config_path: &Path) -> String {
    if !config_path.is_file() {
        return format!("Naming policy: no config at '{}'; run kept setup", config_path.display());
    }
    let config = match kept_core::load_user_config(config_path) {
        Ok(config) => config,
        Err(error) => {
            return format!(
                "Naming policy: config error at '{}': {error}; run kept doctor",
                config_path.display()
            )
        }
    };
    let findings = match kept_core::analyze_index_naming(index, &config) {
        Ok(findings) => findings,
        Err(error) => return format!("Naming policy: analysis error: {error}; run kept doctor"),
    };
    let mut lines = vec![format!("Naming policy: {} finding(s)", findings.len())];
    for finding in findings {
        let issues = finding
            .issues
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        let target = finding
            .proposed_target
            .as_deref()
            .map_or_else(|| "no target".to_string(), |name| format!("proposed: {name}"));
        let blocked = if finding.blocked { " [BLOCKED]" } else { "" };
        lines.push(format!(
            "- {} [{}]: {}; {}{}",
            finding.source.display(),
            finding.rule_id,
            issues,
            target,
            blocked
        ));
    }
    lines.join("\n")
}

pub fn scan_review_summary(index: &kept_core::ScanIndex) -> String {
    let mut lines = vec![format!(
        "{} file(s), {} bytes, {} scan issue(s)",
        index.files.len(),
        index.total_size,
        index.issues.len()
    )];
    for issue in &index.issues {
        let remediation = issue
            .remediation
            .as_deref()
            .map(|r| format!(" | Fix: {r}"))
            .unwrap_or_default();
        lines.push(format!(
            "- [{:?}] {}: {} ({}){}",
            issue.kind, issue.path, issue.message, issue.operation, remediation
        ));
    }
    lines.join("\n")
}

pub fn scan_review_action_from_selection(selection: &str) -> Option<ScanReviewAction> {
    match selection.trim() {
        "1" => Some(ScanReviewAction::Space),
        "2" => Some(ScanReviewAction::Find),
        "3" => Some(ScanReviewAction::Duplicates),
        "4" => Some(ScanReviewAction::Integrity),
        "5" => Some(ScanReviewAction::Issues),
        "6" => Some(ScanReviewAction::Naming),
        "0" | "q" | "" => None,
        _ => None,
    }
}
