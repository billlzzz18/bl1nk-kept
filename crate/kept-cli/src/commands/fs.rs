//! Legacy filesystem treemap, filter, and index command handlers.

use clap::Subcommand;
use std::path::PathBuf;

use super::duplicates::{run_duplicate_action, DuplicateAction};
use crate::helpers::parse_human_size;

#[derive(Subcommand)]
pub enum FsCommands {
    Index {
        root: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        include_hidden: bool,
    },
    Treemap {
        index: PathBuf,
        #[arg(short, long)]
        json: bool,
    },
    Duplicates {
        #[command(subcommand)]
        cmd: DuplicateCommands,
    },
    Filter {
        index: PathBuf,
        #[arg(long = "all", value_name = "FILTER")]
        all_filters: Vec<String>,
        #[arg(long = "any", value_name = "FILTER")]
        any_filters: Vec<String>,
        #[arg(long, value_name = "FIELD:OPERATOR:VALUE")]
        custom: Vec<String>,
        #[arg(short, long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum DuplicateCommands {
    Scan {
        root: PathBuf,
        #[arg(short, long)]
        json: bool,
        #[arg(long, value_enum)]
        action: Option<DuplicateAction>,
        #[arg(short = 'y', long, requires = "action")]
        yes: bool,
    },
}

pub fn handle_fs(cmd: FsCommands) -> anyhow::Result<()> {
    use kept_core::{build_treemap, filter_index, scan_directory, FilterSet, ScanOptions};

    match cmd {
        FsCommands::Duplicates { cmd } => handle_duplicate_scan(cmd)?,
        FsCommands::Index { root, output, include_hidden } => {
            let index = scan_directory(
                &root,
                &ScanOptions {
                    include_hidden,
                    max_depth: None,
                },
            )?;
            std::fs::write(output, serde_json::to_string_pretty(&index)?)?;
            println!("Indexed {} files.", index.files.len());
        }
        FsCommands::Treemap { index, json } => {
            let index_data: kept_core::ScanIndex =
                serde_json::from_str(&std::fs::read_to_string(index)?)?;
            let tree = build_treemap(&index_data);
            if json {
                println!("{}", serde_json::to_string_pretty(&tree)?);
            } else {
                println!(
                    "Treemap generated: {} bytes across {} top-level node(s).",
                    tree.size,
                    tree.children.len()
                );
            }
        }
        FsCommands::Filter {
            index,
            all_filters,
            any_filters,
            custom,
            json,
        } => {
            let index_data: kept_core::ScanIndex =
                serde_json::from_str(&std::fs::read_to_string(index)?)?;
            let filters = FilterSet {
                all: all_filters
                    .iter()
                    .map(|filter| parse_file_filter(filter))
                    .collect::<anyhow::Result<_>>()?,
                any: any_filters
                    .iter()
                    .map(|filter| parse_file_filter(filter))
                    .collect::<anyhow::Result<_>>()?,
                custom: custom
                    .iter()
                    .map(|filter| parse_custom_filter(filter))
                    .collect::<anyhow::Result<_>>()?,
            };
            let records = filter_index(&index_data, &filters);
            if json {
                println!("{}", serde_json::to_string_pretty(&records)?);
            } else {
                for record in &records {
                    println!("{}\t{}\t{}", record.size, record.modified_unix, record.path);
                }
                println!("Matched {} of {} file(s).", records.len(), index_data.files.len());
            }
        }
    }
    Ok(())
}

pub fn handle_duplicate_scan(cmd: DuplicateCommands) -> anyhow::Result<()> {
    use kept_core::{
        find_content_duplicates, scan_directory, ContentDuplicateOptions, ScanOptions,
    };
    use std::io::{self, IsTerminal};

    let DuplicateCommands::Scan { root, json, action, yes } = cmd;
    let index = scan_directory(&root, &ScanOptions::default())?;
    let (groups, stats) = find_content_duplicates(&index, &ContentDuplicateOptions::default())?;
    let report = serde_json::json!({
        "root": index.root,
        "groups": groups,
        "stats": stats,
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Verified content scan: {} duplicate group(s)", groups.len());
        for (position, group) in groups.iter().enumerate() {
            println!("{}. {} ({} file(s))", position + 1, group.kind, group.items.len());
            for item in &group.items {
                println!("   - {item}");
            }
        }
    }

    if yes && action.is_none() {
        anyhow::bail!("--yes ต้องใช้ร่วมกับ --action ที่ระบุชัดเจน");
    }
    if let Some(action) = action {
        if !yes && !io::stdin().is_terminal() {
            anyhow::bail!("โหมด non-interactive ต้องใช้ --action <name> พร้อม --yes");
        }
        return run_duplicate_action(action, &root, &report, yes, Some(&index));
    }
    if !json && crate::helpers::is_interactive_terminal() {
        if let Some(action) = super::duplicates::choose_duplicate_action()? {
            run_duplicate_action(action, &root, &report, false, Some(&index))?;
        }
    }
    Ok(())
}

pub fn parse_file_filter(raw: &str) -> anyhow::Result<kept_core::FileFilter> {
    use kept_core::FileFilter;

    let parts = raw.splitn(3, ':').collect::<Vec<_>>();
    let (field, operator, value) = match parts.as_slice() {
        [field, value] => (*field, default_filter_operator(field)?, *value),
        [field, operator, value] => (*field, *operator, *value),
        _ => anyhow::bail!("รูปแบบ filter ไม่ถูกต้อง '{raw}'; ใช้ field:value หรือ field:operator:value"),
    };
    if value.is_empty() {
        anyhow::bail!("ค่า filter ของ '{field}' ต้องไม่ว่าง");
    }

    match (field, operator) {
        ("extension" | "ext", "eq") => Ok(FileFilter::Extension(value.to_string())),
        ("name", "contains") => Ok(FileFilter::NameContains(value.to_string())),
        ("path", "contains") => Ok(FileFilter::PathContains(value.to_string())),
        ("kind", "eq") => Ok(FileFilter::Kind(value.to_string())),
        ("size", "ge") => Ok(FileFilter::MinSize(parse_human_size(value)?)),
        ("size", "le") => Ok(FileFilter::MaxSize(parse_human_size(value)?)),
        ("size", "gt") => Ok(FileFilter::MinSize(parse_human_size(value)?.saturating_add(1))),
        ("size", "lt") => Ok(FileFilter::MaxSize(parse_human_size(value)?.saturating_sub(1))),
        ("modified", "after" | "ge") => {
            Ok(FileFilter::ModifiedAfter(parse_filter_number(field, value)?))
        }
        ("modified", "before" | "le") => {
            Ok(FileFilter::ModifiedBefore(parse_filter_number(field, value)?))
        }
        _ => anyhow::bail!("คู่ field:operator ไม่รองรับ '{field}:{operator}'"),
    }
}

pub fn default_filter_operator(field: &str) -> anyhow::Result<&'static str> {
    match field {
        "extension" | "ext" | "kind" => Ok("eq"),
        "name" | "path" => Ok("contains"),
        "size" => anyhow::bail!("size filter ต้องระบุ operator เช่น size:ge:10mb หรือ size:le:50mb"),
        "modified" => anyhow::bail!(
            "modified filter ต้องระบุ operator เช่น modified:after:<unix> หรือ modified:before:<unix>"
        ),
        _ => anyhow::bail!("ไม่รู้จัก filter field '{field}'"),
    }
}

pub fn parse_filter_number(field: &str, value: &str) -> anyhow::Result<u64> {
    value
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("ค่า filter ของ '{field}' ต้องเป็นตัวเลข Unix timestamp"))
}

pub fn parse_custom_filter(raw: &str) -> anyhow::Result<kept_core::CustomFilter> {
    use kept_core::{CustomFilter, CustomOperator};

    let parts = raw.splitn(3, ':').collect::<Vec<_>>();
    let [field, operator, value] = match parts.as_slice() {
        [field, operator, value] => [*field, *operator, *value],
        _ => anyhow::bail!("custom filter ต้องอยู่ในรูปแบบ field:operator:value"),
    };
    if field.is_empty() || operator.is_empty() || value.is_empty() {
        anyhow::bail!("custom filter ต้องมี field, operator และ value ครบถ้วน");
    }

    let operator = match operator {
        "eq" => CustomOperator::Equals,
        "contains" => CustomOperator::Contains,
        "starts_with" | "prefix" => CustomOperator::StartsWith,
        "ends_with" | "suffix" => CustomOperator::EndsWith,
        "ge" => CustomOperator::GreaterOrEqual,
        "le" => CustomOperator::LessOrEqual,
        _ => anyhow::bail!("operator ไม่รองรับ '{operator}'"),
    };

    Ok(CustomFilter {
        field: field.to_string(),
        operator,
        value: value.to_string(),
    })
}
