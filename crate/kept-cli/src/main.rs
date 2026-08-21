use clap::{Parser, Subcommand};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use clap::ValueEnum;
use dialoguer::{Input, Select};

#[derive(Parser)]
#[command(
    name = "kept",
    version = env!("CARGO_PKG_VERSION"),
    about = "Unified CLI for Keywords, Filesystem Analysis, and Document Sync"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory and create or refresh its reusable index.
    Scan {
        root: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        include_hidden: bool,
        #[arg(short, long)]
        json: bool,
    },
    /// Find files from the latest scan index using simple filter facts.
    Find {
        root: PathBuf,
        #[arg(long = "type")]
        file_type: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long = "path")]
        path_contains: Option<String>,
        #[arg(long = "min-size")]
        min_size: Option<String>,
        #[arg(long = "max-size")]
        max_size: Option<String>,
        #[arg(long)]
        after: Option<u64>,
        #[arg(long)]
        before: Option<u64>,
        #[arg(long)]
        index: Option<PathBuf>,
        #[arg(short, long)]
        json: bool,
    },
    /// Search a keyword registry.
    Search {
        registry: PathBuf,
        query: String,
        #[arg(short, long)]
        group: Option<String>,
        #[arg(short, long)]
        json: bool,
    },
    /// Inspect and manage keyword registry groups.
    Group {
        #[command(subcommand)]
        cmd: GroupCommands,
    },
    /// Convert a document between supported offline formats.
    Convert { input: PathBuf, output: PathBuf },
    // NOTE-001: setup เตรียม config.yaml ที่ผู้ใช้เป็นเจ้าของโดยไม่แตะไฟล์งานหรือสร้าง scope เดาเอง
    /// สร้าง config.yaml ของผู้ใช้เมื่อยังไม่มี และเปิดขั้นตอนตั้งค่าแบบ interactive.
    Setup,
    // NOTE-001: config ทำงานกับ config.yaml จริง ไม่ใช่ชั้น policy ภายในของโปรแกรม
    /// ดูหรือแก้ไข config.yaml ของผู้ใช้.
    Config {
        #[command(subcommand)]
        cmd: Option<ConfigCommands>,
    },
    /// ตรวจ configuration และสภาพแวดล้อมที่ kept ใช้งานได้จริง.
    Doctor {
        #[arg(long)]
        fix: bool,
    },
    /// Open the interactive review menu for an existing scan index.
    Review {
        root: PathBuf,
        #[arg(long)]
        index: Option<PathBuf>,
    },
    /// Review verified duplicate candidates from the latest scan index.
    Duplicates {
        root: PathBuf,
        #[arg(short, long)]
        json: bool,
        #[arg(long)]
        index: Option<PathBuf>,
        #[arg(long, value_enum)]
        action: Option<DuplicateAction>,
        #[arg(short = 'y', long, requires = "action")]
        yes: bool,
    },
    /// Run the offline evidence and correction loop.
    Evidence {
        #[command(subcommand)]
        cmd: EvidenceCommands,
    },
    /// Import, validate, snapshot, and replay reviewed gold assertions.
    Corpus {
        #[command(subcommand)]
        cmd: CorpusCommands,
    },
    #[command(hide = true)]
    /// Legacy compatibility: manage keyword registries and search.
    Registry {
        #[command(subcommand)]
        cmd: RegistryCommands,
    },
    #[command(hide = true)]
    /// Legacy compatibility: filesystem analysis and treemap.
    Fs {
        #[command(subcommand)]
        cmd: FsCommands,
    },
    #[command(hide = true)]
    /// Legacy compatibility: document sync and conversion.
    Doc {
        #[command(subcommand)]
        cmd: DocCommands,
    },
    #[command(hide = true)]
    /// Reserved TUI command.
    Tui,
    #[command(hide = true)]
    /// Reserved MCP server command.
    Mcp {
        #[arg(long, default_value = "stdio")]
        transport: String,
    },
}

#[derive(Subcommand)]
enum GroupCommands {
    /// แสดงชนิด field ที่ validator รองรับจริง.
    Types,
    /// แสดง groups ตามลำดับใน registry.
    List { registry: PathBuf },
    /// แสดงรายละเอียด group หนึ่งรายการ.
    Show { registry: PathBuf, id: String },
    /// เพิ่ม group ด้วย base fields id และ aliases ที่ค้นหาได้ทันที.
    Add {
        registry: PathBuf,
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    /// แก้ชื่อหรือคำอธิบาย group.
    Set {
        registry: PathBuf,
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// ลบ group ที่ระบุจาก registry.
    Remove { registry: PathBuf, id: String },
    /// ย้าย group ก่อนหรือหลัง group อื่นเพื่อกำหนดลำดับ registry.
    Move {
        registry: PathBuf,
        id: String,
        #[arg(long, conflicts_with = "after")]
        before: Option<String>,
        #[arg(long, conflicts_with = "before")]
        after: Option<String>,
    },
    /// จัดการ field schema ของ group.
    Field {
        #[command(subcommand)]
        cmd: GroupFieldCommands,
    },
}

#[derive(Subcommand)]
enum GroupFieldCommands {
    List {
        registry: PathBuf,
        group: String,
    },
    Add {
        registry: PathBuf,
        group: String,
        name: String,
        field_type: String,
        #[arg(long)]
        item_type: Option<String>,
        #[arg(long)]
        values: Option<String>,
        #[arg(long)]
        required: bool,
        #[arg(long, default_value = "")]
        description: String,
    },
    Remove {
        registry: PathBuf,
        group: String,
        name: String,
    },
}

#[derive(Subcommand)]
enum RegistryCommands {
    Validate {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(value_name = "ID")]
        entry_id: Option<String>,
        #[arg(short, long)]
        group: Option<String>,
    },
    Search {
        #[arg(short, long)]
        path: PathBuf,
        query: String,
        #[arg(short, long)]
        group: Option<String>,
        #[arg(short, long)]
        json: bool,
    },
    Analyze {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long)]
        json: bool,
    },
    Import {
        #[arg(short, long)]
        csv: PathBuf,
        #[arg(short, long)]
        group_id: String,
        #[arg(short = 'n', long)]
        group_name: String,
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum FsCommands {
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
        /// เงื่อนไขที่ทุกข้อจำเป็นต้องผ่าน เช่น `extension:rs` หรือ `size:ge:1048576`
        #[arg(long = "all", value_name = "FILTER")]
        all_filters: Vec<String>,
        /// เงื่อนไขที่ต้องผ่านอย่างน้อยหนึ่งข้อ
        #[arg(long = "any", value_name = "FILTER")]
        any_filters: Vec<String>,
        /// กรองฟิลด์ด้วยรูปแบบ `field:operator:value` เช่น `name:contains:report`
        #[arg(long, value_name = "FIELD:OPERATOR:VALUE")]
        custom: Vec<String>,
        #[arg(short, long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// แสดง field, type และค่าที่ naming config รองรับ.
    Fields,
    /// ดูและแก้ baseline naming defaults.
    Defaults {
        #[command(subcommand)]
        cmd: DefaultCommands,
    },
    /// จัดการ naming profiles ใน config.yaml.
    Profile {
        #[command(subcommand)]
        cmd: ProfileCommands,
    },
    /// จัดการ path scopes ที่ผู้ใช้ระบุแบบ absolute.
    Scope {
        #[command(subcommand)]
        cmd: ScopeCommands,
    },
    /// เปิด config.yaml ใน nano หรือ editor ที่ผู้ใช้กำหนด.
    Edit,
}

#[derive(Subcommand)]
enum DefaultCommands {
    /// แสดง YAML ของ defaults.naming.
    Show,
    /// ตั้ง default naming field.
    Set { field: String, value: String },
    /// ล้าง default naming field.
    Unset { field: String },
}

#[derive(Subcommand)]
enum ScopeCommands {
    /// แสดง scopes ตาม resolve precedence: path ลึกก่อน แล้ว priority สูงก่อน.
    List,
    /// แสดง YAML ของ scope หนึ่งรายการ.
    Show { path: PathBuf },
    /// เพิ่ม scope ใหม่ให้ profile ที่มีอยู่.
    Add {
        profile: String,
        path: PathBuf,
        #[arg(long, default_value_t = 0)]
        priority: i32,
        #[arg(long, default_value_t = true, value_parser = clap::value_parser!(bool))]
        recursive: bool,
    },
    /// แก้ profile, priority หรือ recursive ของ scope.
    Set {
        path: PathBuf,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        priority: Option<i32>,
        #[arg(long, value_parser = clap::value_parser!(bool))]
        recursive: Option<bool>,
    },
    /// ลบ scope หนึ่งรายการ.
    Remove { path: PathBuf },
    /// จัดการ absolute exception paths ของ scope.
    Exception {
        #[command(subcommand)]
        cmd: ScopeExceptionCommands,
    },
    /// ตั้งหรือล้าง naming overrides เฉพาะ scope.
    Setting {
        #[command(subcommand)]
        cmd: ScopeSettingCommands,
    },
}

#[derive(Subcommand)]
enum ScopeSettingCommands {
    Set {
        scope: PathBuf,
        field: String,
        value: String,
    },
    Unset {
        scope: PathBuf,
        field: String,
    },
}

#[derive(Subcommand)]
enum ScopeExceptionCommands {
    Add { scope: PathBuf, path: PathBuf },
    Remove { scope: PathBuf, path: PathBuf },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// แสดง profiles ทั้งหมดตามลำดับชื่อ.
    List,
    /// แสดง YAML ของ profile หนึ่งรายการ.
    Show { name: String },
    /// เพิ่ม profile เปล่าที่สืบทอด defaults.
    Add {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// ลบ profile ที่ไม่มี scope อ้างถึง.
    Remove { name: String },
    /// ตั้งค่า naming field ของ profile.
    Set {
        name: String,
        field: String,
        value: String,
    },
    /// ล้างค่า naming field ของ profile เพื่อกลับไปใช้ defaults.
    Unset { name: String, field: String },
}

#[derive(Subcommand)]
enum DuplicateCommands {
    /// Scan a directory and verify duplicate content without changing files.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanReviewAction {
    Space,
    Find,
    Duplicates,
    Issues,
    Naming,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SetupAction {
    EditConfig,
    Exit,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DuplicateAction {
    Show,
    ExportPlan,
}

#[derive(Subcommand)]
enum DocCommands {
    Sync {
        #[arg(short, long)]
        dir: PathBuf,
        #[arg(long)]
        notion_db: String,
    },
    Convert {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

mod corpus;
mod evidence;
use corpus::{handle_corpus, CorpusCommands};
use evidence::{handle_evidence, EvidenceCommands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            root,
            output,
            include_hidden,
            json,
        } => handle_task_scan(root, output, include_hidden, json)?,
        Commands::Find {
            root,
            file_type,
            name,
            path_contains,
            min_size,
            max_size,
            after,
            before,
            index,
            json,
        } => handle_task_find(FindRequest {
            root,
            file_type,
            name,
            path_contains,
            min_size,
            max_size,
            after,
            before,
            index,
            json,
        })?,
        Commands::Search {
            registry,
            query,
            group,
            json,
        } => handle_registry(RegistryCommands::Search {
            path: registry,
            query,
            group,
            json,
        })?,
        Commands::Group { cmd } => handle_group(cmd)?,
        Commands::Convert { input, output } => {
            handle_doc(DocCommands::Convert { input, output }).await?
        }
        Commands::Setup => handle_setup()?,
        Commands::Config { cmd } => handle_config(cmd)?,
        Commands::Doctor { fix } => handle_doctor(fix)?,
        Commands::Review { root, index } => handle_task_review(root, index, true)?,
        Commands::Duplicates {
            root,
            json,
            index,
            action,
            yes,
        } => handle_task_duplicates(root, index, json, action, yes)?,
        Commands::Evidence { cmd } => handle_evidence(cmd)?,
        Commands::Corpus { cmd } => handle_corpus(cmd)?,
        Commands::Registry { cmd } => handle_registry(cmd)?,
        Commands::Fs { cmd } => handle_fs(cmd)?,
        Commands::Doc { cmd } => handle_doc(cmd).await?,
        Commands::Tui => anyhow::bail!(
            "TUI ยังไม่เปิดใช้ใน kept CLI รุ่นนี้; ใช้คำสั่ง registry, fs หรือ doc convert แทน"
        ),
        Commands::Mcp { transport } => anyhow::bail!(
            "MCP transport '{transport}' ยังไม่เปิดใช้ใน kept CLI รุ่นนี้; เปิด feature mcp ของ kept-doc เมื่อต้องการ server แยก"
        ),
    }

    Ok(())
}

const REGISTRY_GROUP_FIELD_TYPES: &[(&str, &str)] = &[
    ("string", "ข้อความหนึ่งค่า"),
    ("enum", "ข้อความที่จำกัด values"),
    ("array", "รายการค่า; itemType เป็น string, number หรือ boolean"),
    ("object", "object JSON/YAML"),
    ("number", "ตัวเลข"),
    ("boolean", "true หรือ false"),
];

fn load_registry_for_group(
    path: &std::path::Path,
) -> anyhow::Result<kept_core::schema::KeywordRegistry> {
    kept_core::load_registry(path).map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn save_registry_for_group(
    path: &std::path::Path,
    registry: &kept_core::schema::KeywordRegistry,
) -> anyhow::Result<()> {
    kept_core::Validator::new(registry.clone())
        .validate_registry()
        .map_err(|errors| anyhow::anyhow!("registry validation failed: {errors:?}"))?;
    kept_core::save_registry(path, registry).map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn handle_group(cmd: GroupCommands) -> anyhow::Result<()> {
    match cmd {
        GroupCommands::Types => {
            println!("Type\tMeaning");
            for (field_type, meaning) in REGISTRY_GROUP_FIELD_TYPES {
                println!("{field_type}\t{meaning}");
            }
            Ok(())
        }
        GroupCommands::List { registry: path } => {
            let registry = load_registry_for_group(&path)?;
            println!("Position\tGroup ID\tName\tEntries\tDescription");
            for (position, group) in registry.groups.iter().enumerate() {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    position + 1,
                    group.group_id,
                    group.group_name,
                    group.entries.len(),
                    group.description
                );
            }
            Ok(())
        }
        GroupCommands::Show { registry: path, id } => {
            let registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter()
                .find(|group| group.group_id == id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{id}'"))?;
            println!("{}", serde_json::to_string_pretty(group)?);
            Ok(())
        }
        GroupCommands::Add {
            registry: path,
            id,
            name,
            description,
        } => {
            let mut registry = load_registry_for_group(&path)?;
            if registry.groups.iter().any(|group| group.group_id == id) {
                anyhow::bail!("มี group '{id}' อยู่แล้ว")
            }
            registry
                .groups
                .push(default_registry_group(id.clone(), name, description));
            save_registry_for_group(&path, &registry)?;
            println!("Added group: {id}");
            Ok(())
        }
        GroupCommands::Set {
            registry: path,
            id,
            name,
            description,
        } => {
            if name.is_none() && description.is_none() {
                anyhow::bail!("group set ต้องระบุ --name หรือ --description")
            }
            let mut registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter_mut()
                .find(|group| group.group_id == id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{id}'"))?;
            if let Some(name) = name {
                group.group_name = name;
            }
            if let Some(description) = description {
                group.description = description;
            }
            save_registry_for_group(&path, &registry)?;
            println!("Updated group: {id}");
            Ok(())
        }
        GroupCommands::Remove { registry: path, id } => {
            let mut registry = load_registry_for_group(&path)?;
            let position = registry
                .groups
                .iter()
                .position(|group| group.group_id == id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{id}'"))?;
            registry.groups.remove(position);
            save_registry_for_group(&path, &registry)?;
            println!("Removed group: {id}");
            Ok(())
        }
        GroupCommands::Field { cmd } => handle_group_field(cmd),
        GroupCommands::Move {
            registry: path,
            id,
            before,
            after,
        } => {
            let insert_after = after.is_some();
            let target = before.or(after).ok_or_else(|| {
                anyhow::anyhow!("ต้องระบุ --before <group-id> หรือ --after <group-id>")
            })?;
            if id == target {
                anyhow::bail!("ย้าย group '{id}' เทียบกับตัวเองไม่ได้")
            }
            let mut registry = load_registry_for_group(&path)?;
            let source = registry
                .groups
                .iter()
                .position(|group| group.group_id == id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{id}'"))?;
            let group = registry.groups.remove(source);
            let target_position = registry
                .groups
                .iter()
                .position(|group| group.group_id == target)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ target group '{target}'"))?;
            registry
                .groups
                .insert(target_position + usize::from(insert_after), group);
            save_registry_for_group(&path, &registry)?;
            println!("Moved group '{id}'");
            Ok(())
        }
    }
}

fn group_field_schema(
    field_type: &str,
    item_type: Option<String>,
    values: Option<String>,
    required: bool,
    description: String,
) -> anyhow::Result<kept_core::schema::FieldSchema> {
    if !matches!(
        field_type,
        "string" | "enum" | "array" | "object" | "number" | "boolean"
    ) {
        anyhow::bail!("field type '{field_type}' ไม่รองรับ; ดู kept group types")
    }
    if field_type == "array" {
        let Some(item_type) = &item_type else {
            anyhow::bail!("array field ต้องระบุ --item-type string, number หรือ boolean")
        };
        if !matches!(item_type.as_str(), "string" | "number" | "boolean") {
            anyhow::bail!("array item type '{item_type}' ไม่รองรับ")
        }
    } else if item_type.is_some() {
        anyhow::bail!("--item-type ใช้ได้กับ array field เท่านั้น")
    }
    let values = values.map(|value| parse_string_list(&value));
    if field_type == "enum" && values.as_ref().is_none_or(Vec::is_empty) {
        anyhow::bail!("enum field ต้องระบุ --values เช่น THB,USD")
    }
    Ok(kept_core::schema::FieldSchema {
        field_type: field_type.to_string(),
        item_type,
        values,
        required: Some(required),
        description,
        ..Default::default()
    })
}

fn handle_group_field(cmd: GroupFieldCommands) -> anyhow::Result<()> {
    match cmd {
        GroupFieldCommands::List {
            registry: path,
            group,
        } => {
            let registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter()
                .find(|item| item.group_id == group)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group"))?;
            println!("Field\tType\tItem type\tRequired\tValues\tDescription");
            let mut fields = group.base_fields_schema.iter().collect::<Vec<_>>();
            fields.sort_by(|left, right| left.0.cmp(right.0));
            for (name, schema) in fields {
                println!(
                    "{name}\t{}\t{}\t{}\t{}\t{}",
                    schema.field_type,
                    schema.item_type.as_deref().unwrap_or(""),
                    schema.required.unwrap_or(false),
                    schema
                        .values
                        .as_ref()
                        .map(|items| items.join(","))
                        .unwrap_or_default(),
                    schema.description
                );
            }
            Ok(())
        }
        GroupFieldCommands::Add {
            registry: path,
            group: group_id,
            name,
            field_type,
            item_type,
            values,
            required,
            description,
        } => {
            let schema = group_field_schema(&field_type, item_type, values, required, description)?;
            let mut registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter_mut()
                .find(|item| item.group_id == group_id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{group_id}'"))?;
            if group.base_fields_schema.contains_key(&name) {
                anyhow::bail!("group '{group_id}' มี field '{name}' อยู่แล้ว")
            }
            if required && !group.entries.is_empty() {
                anyhow::bail!(
                    "เพิ่ม required field '{name}' ไม่ได้ เพราะ group '{group_id}' มี entries อยู่แล้ว"
                )
            }
            group.base_fields_schema.insert(name.clone(), schema);
            save_registry_for_group(&path, &registry)?;
            println!("Added field '{name}' to group '{group_id}'");
            Ok(())
        }
        GroupFieldCommands::Remove {
            registry: path,
            group: group_id,
            name,
        } => {
            let mut registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter_mut()
                .find(|item| item.group_id == group_id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{group_id}'"))?;
            if !group.base_fields_schema.contains_key(&name) {
                anyhow::bail!("group '{group_id}' ไม่มี field '{name}'")
            }
            if matches!(name.as_str(), "id" | "aliases") {
                anyhow::bail!("ลบ base search field '{name}' ไม่ได้")
            }
            group.base_fields_schema.remove(&name);
            save_registry_for_group(&path, &registry)?;
            println!("Removed field '{name}' from group '{group_id}'");
            Ok(())
        }
    }
}

fn default_registry_group(
    id: String,
    name: String,
    description: String,
) -> kept_core::schema::KeywordGroup {
    use std::collections::HashMap;

    let mut base_fields_schema = HashMap::new();
    base_fields_schema.insert(
        "id".to_string(),
        kept_core::schema::FieldSchema {
            field_type: "string".to_string(),
            required: Some(true),
            description: "Stable entry identifier".to_string(),
            ..Default::default()
        },
    );
    base_fields_schema.insert(
        "aliases".to_string(),
        kept_core::schema::FieldSchema {
            field_type: "array".to_string(),
            item_type: Some("string".to_string()),
            required: Some(true),
            description: "Terms used by keyword search".to_string(),
            ..Default::default()
        },
    );
    kept_core::schema::KeywordGroup {
        group_id: id,
        group_name: name,
        description,
        base_fields_schema,
        custom_field_allowed: kept_core::schema::CustomFieldConfig {
            enabled: true,
            description: "Custom fields are allowed per entry".to_string(),
            ..Default::default()
        },
        entries: Vec::new(),
        group_stats: None,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SetupResult {
    path: PathBuf,
    created: bool,
}

/// NOTE-001: ใช้ตัวสร้าง config เดียวกับ core และ validate ทันที เพื่อให้ setup บอกได้ว่าไฟล์พร้อมใช้งาน ไม่ใช่เพียงเขียน bytes สำเร็จ
fn setup_user_config_at(path: &std::path::Path) -> anyhow::Result<SetupResult> {
    let created = kept_core::create_user_config_if_missing(path)?;
    kept_core::load_user_config(path)?;
    Ok(SetupResult {
        path: path.to_path_buf(),
        created,
    })
}

fn handle_setup() -> anyhow::Result<()> {
    let path = kept_core::default_user_config_path()?;
    let result = setup_user_config_at(&path)?;
    if result.created {
        println!("Created user config: {}", result.path.display());
    } else {
        println!("User config already exists: {}", result.path.display());
    }
    if !is_interactive_terminal() {
        return Ok(());
    }
    let choices = ["1. เปิด config.yaml เพื่อแก้ไข", "0. ออกจาก setup"];
    let selected = Select::new()
        .with_prompt("setup เสร็จแล้ว เลือกขั้นตอนถัดไป")
        .items(&choices)
        .default(0)
        .interact_opt()?;
    let action = selected.and_then(|position| {
        setup_action_from_selection(match position {
            0 => "1",
            _ => "0",
        })
    });
    match action {
        Some(SetupAction::EditConfig) => handle_config(Some(ConfigCommands::Edit)),
        Some(SetupAction::Exit) | None => Ok(()),
    }
}

/// NOTE-001: dialoguer ต้องมีทั้ง input และ output terminal; pipe ฝั่งใดฝั่งหนึ่งต้องทำให้คำสั่งจบแบบ non-interactive
fn is_interactive_terminal() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

fn setup_action_from_selection(selection: &str) -> Option<SetupAction> {
    match selection.trim().to_ascii_lowercase().as_str() {
        "1" => Some(SetupAction::EditConfig),
        "0" => Some(SetupAction::Exit),
        _ => None,
    }
}

/// NOTE-001: `KEPT_EDITOR` รับเฉพาะ executable เดี่ยวเพื่อไม่ต้องตีความ shell string; ถ้าไม่กำหนด kept เปิด nano ตาม contract
fn resolve_config_editor(explicit_editor: Option<std::ffi::OsString>) -> std::ffi::OsString {
    explicit_editor
        .or_else(|| std::env::var_os("KEPT_EDITOR"))
        .unwrap_or_else(|| std::ffi::OsString::from("nano"))
}

fn handle_doctor(fix: bool) -> anyhow::Result<()> {
    let path = kept_core::default_user_config_path()?;
    let result = run_doctor_at(&path, fix)?;
    if result.repaired {
        println!("Repaired config: {}", path.display());
        if let Some(backup_path) = &result.backup_path {
            println!("Backup: {}", backup_path.display());
        }
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

const CONFIG_FIELD_CATALOG: &[(&str, &str, &str)] = &[
    ("unicode", "string", "nfc"),
    ("case", "enum", "lower | upper | preserve"),
    ("separator", "enum", "kebab | snake | preserve"),
    (
        "extensions",
        "string list",
        "file extensions without a leading dot",
    ),
    ("flagControlCharacters", "boolean", "true | false"),
    ("flagTrimWhitespace", "boolean", "true | false"),
    ("flagCaseCollisions", "boolean", "true | false"),
    ("flagPortabilityConflicts", "boolean", "true | false"),
    ("numbers.allow", "boolean", "true | false"),
    ("numbers.maxDigitsPerToken", "integer", "0 or greater"),
    ("words.min", "integer", "0 or greater"),
    ("words.max", "integer", "0 or greater"),
    ("length.stem.min", "integer", "0 or greater"),
    ("length.stem.max", "integer", "0 or greater"),
    ("whitespace.trim", "boolean", "true | false"),
    ("whitespace.collapseInternal", "boolean", "true | false"),
    ("prefix.required", "string", "a literal prefix"),
    ("prefix.allow", "string list", "allowed literal prefixes"),
    ("aliases.<token>", "string", "replacement for one token"),
    (
        "shortcuts.<token>",
        "string",
        "expansion for one filename token",
    ),
    (
        "variables.<name>",
        "string",
        "value expanded in replacements",
    ),
    ("similarity.name.threshold", "number", "0.0 through 1.0"),
    ("similarity.name.caseSensitive", "boolean", "true | false"),
    (
        "replacements",
        "YAML list",
        "from, to, caseSensitive entries",
    ),
    (
        "reposition",
        "YAML list",
        "token with front or back position",
    ),
    ("stemRegex", "string", "Rust regular expression"),
];

/// NOTE-001: catalog นี้เปิดเผยเฉพาะ key ที่ map กับ NamingSettings จริง เพื่อไม่ให้ user ต้องเดาหรือ CLI รับ field ที่ analyzer ไม่อ่าน
fn config_field_default_value(field: &str, naming: &kept_core::NamingSettings) -> String {
    match field {
        "unicode" => naming.unicode.clone().unwrap_or_default(),
        "case" => naming.case.clone().unwrap_or_default(),
        "separator" => naming.separator.clone().unwrap_or_default(),
        "extensions" => naming.extensions.join(","),
        "flagControlCharacters" => naming
            .flag_control_characters
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "flagTrimWhitespace" => naming
            .flag_trim_whitespace
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "flagCaseCollisions" => naming
            .flag_case_collisions
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "flagPortabilityConflicts" => naming
            .flag_portability_conflicts
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "numbers.allow" => naming
            .numbers
            .allow
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "numbers.maxDigitsPerToken" => naming
            .numbers
            .max_digits_per_token
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "words.min" => naming
            .words
            .min
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "words.max" => naming
            .words
            .max
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "length.stem.min" => naming
            .length
            .stem
            .as_ref()
            .and_then(|range| range.min)
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "length.stem.max" => naming
            .length
            .stem
            .as_ref()
            .and_then(|range| range.max)
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "whitespace.trim" => naming
            .whitespace
            .trim
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "whitespace.collapseInternal" => naming
            .whitespace
            .collapse_internal
            .map(|value| value.to_string())
            .unwrap_or_default(),
        "prefix.required" => naming.prefix.required.clone().unwrap_or_default(),
        "prefix.allow" => naming.prefix.allow.join(","),
        "aliases.<token>" => format!("{} mapping(s)", naming.aliases.len()),
        "shortcuts.<token>" => format!("{} mapping(s)", naming.shortcuts.len()),
        "variables.<name>" => format!("{} mapping(s)", naming.variables.len()),
        "similarity.name.threshold" => naming
            .similarity
            .name
            .as_ref()
            .map(|rule| rule.threshold.to_string())
            .unwrap_or_default(),
        "similarity.name.caseSensitive" => naming
            .similarity
            .name
            .as_ref()
            .map(|rule| rule.case_sensitive.to_string())
            .unwrap_or_default(),
        "replacements" => format!("{} rule(s)", naming.replacements.len()),
        "reposition" => format!("{} rule(s)", naming.reposition.len()),
        "stemRegex" => naming.stem_regex.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn print_config_field_catalog(naming: &kept_core::NamingSettings) {
    println!("Field\tType\tAllowed values\tDefault value");
    for (field, value_type, allowed_values) in CONFIG_FIELD_CATALOG {
        println!(
            "{field}\t{value_type}\t{allowed_values}\t{}",
            config_field_default_value(field, naming)
        );
    }
}

fn handle_config(cmd: Option<ConfigCommands>) -> anyhow::Result<()> {
    let path = kept_core::default_user_config_path()?;
    match cmd {
        None => {
            let config = load_config_for_mutation(&path)?;
            println!("Config: {}", path.display());
            println!("Profiles: {}", config.profiles.len());
            for (name, profile) in &config.profiles {
                println!(
                    "  - {name}: {}",
                    profile.description.as_deref().unwrap_or("no description")
                );
            }
            println!("Scopes: {}", config.scopes.len());
            for scope in &config.scopes {
                println!(
                    "  - {} -> {} (priority {}, recursive {})",
                    scope.path.display(),
                    scope.profile,
                    scope.priority,
                    scope.recursive
                );
            }
            println!("\nNext steps:");
            println!("  kept config fields");
            println!("  kept config defaults show");
            println!("  kept config profile list");
            println!("  kept config scope list");
            println!("  kept config edit  # advanced YAML editing");
            Ok(())
        }
        Some(ConfigCommands::Fields) => {
            let config = load_config_for_mutation(&path)?;
            print_config_field_catalog(&config.defaults.naming);
            Ok(())
        }
        Some(ConfigCommands::Defaults { cmd }) => handle_config_defaults(&path, cmd),
        Some(ConfigCommands::Profile { cmd }) => handle_config_profile(&path, cmd),
        Some(ConfigCommands::Scope { cmd }) => handle_config_scope(&path, cmd),
        Some(ConfigCommands::Edit) => {
            if !path.is_file() {
                anyhow::bail!(
                    "ยังไม่มี config ที่ '{}'; รัน kept setup ก่อน แล้วจึงใช้ kept config edit",
                    path.display()
                );
            }
            let editor = resolve_config_editor(None);
            let status = std::process::Command::new(&editor)
                .arg(&path)
                .status()
                .map_err(|error| {
                    anyhow::anyhow!(
                        "เปิด editor '{}' ไม่ได้: {error}; ติดตั้ง nano หรือกำหนด KEPT_EDITOR เป็น path ของ executable",
                        std::path::Path::new(&editor).display()
                    )
                })?;
            if !status.success() {
                anyhow::bail!(
                    "editor '{}' ออกด้วยสถานะ {status}; config ยังอยู่ที่ '{}'",
                    std::path::Path::new(&editor).display(),
                    path.display()
                );
            }
            Ok(())
        }
    }
}

fn load_config_for_mutation(path: &std::path::Path) -> anyhow::Result<kept_core::UserConfig> {
    kept_core::load_user_config(path).map_err(|error| {
        anyhow::anyhow!(
            "เปิด config ไม่ได้ที่ '{}': {error}; รัน kept setup เพื่อสร้างหรือ kept doctor --fix เพื่อกู้ไฟล์ที่เสีย",
            path.display()
        )
    })
}

fn handle_config_defaults(path: &std::path::Path, cmd: DefaultCommands) -> anyhow::Result<()> {
    let mut config = load_config_for_mutation(path)?;
    match cmd {
        DefaultCommands::Show => {
            println!("{}", serde_yaml::to_string(&config.defaults.naming)?);
            Ok(())
        }
        DefaultCommands::Set { field, value } => {
            set_profile_field(&mut config.defaults.naming, &field, &value)?;
            kept_core::save_user_config(path, &config)?;
            println!("Set default field '{field}'");
            Ok(())
        }
        DefaultCommands::Unset { field } => {
            unset_profile_field(&mut config.defaults.naming, &field)?;
            kept_core::save_user_config(path, &config)?;
            println!("Unset default field '{field}'");
            Ok(())
        }
    }
}

fn handle_config_profile(path: &std::path::Path, cmd: ProfileCommands) -> anyhow::Result<()> {
    let mut config = load_config_for_mutation(path)?;
    match cmd {
        ProfileCommands::List => {
            println!("Profile\tDescription\tExtensions");
            for (name, profile) in &config.profiles {
                let description = profile.description.as_deref().unwrap_or("");
                println!(
                    "{name}\t{description}\t{}",
                    profile.naming.extensions.join(",")
                );
            }
            Ok(())
        }
        ProfileCommands::Show { name } => {
            let profile = config
                .profiles
                .get(&name)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ profile '{name}'"))?;
            println!("{}", serde_yaml::to_string(profile)?);
            Ok(())
        }
        ProfileCommands::Add { name, description } => {
            if config.profiles.contains_key(&name) {
                anyhow::bail!("มี profile '{name}' อยู่แล้ว; ใช้ kept config profile set หรือ edit")
            }
            config.profiles.insert(
                name.clone(),
                kept_core::NamingProfile {
                    description,
                    naming: kept_core::NamingSettings::default(),
                },
            );
            kept_core::save_user_config(path, &config)?;
            println!("Added profile: {name}");
            Ok(())
        }
        ProfileCommands::Remove { name } => {
            if config.scopes.iter().any(|scope| scope.profile == name) {
                anyhow::bail!("ลบ profile '{name}' ไม่ได้ เพราะมี scope อ้างถึง; ย้ายหรือลบ scope ก่อน")
            }
            if config.profiles.remove(&name).is_none() {
                anyhow::bail!("ไม่พบ profile '{name}'")
            }
            kept_core::save_user_config(path, &config)?;
            println!("Removed profile: {name}");
            Ok(())
        }
        ProfileCommands::Set { name, field, value } => {
            let profile = config
                .profiles
                .get_mut(&name)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ profile '{name}'"))?;
            set_profile_field(&mut profile.naming, &field, &value)?;
            kept_core::save_user_config(path, &config)?;
            println!("Set profile '{name}' field '{field}'");
            Ok(())
        }
        ProfileCommands::Unset { name, field } => {
            let profile = config
                .profiles
                .get_mut(&name)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ profile '{name}'"))?;
            unset_profile_field(&mut profile.naming, &field)?;
            kept_core::save_user_config(path, &config)?;
            println!("Unset profile '{name}' field '{field}'");
            Ok(())
        }
    }
}

fn require_absolute_user_path(path: &std::path::Path, label: &str) -> anyhow::Result<()> {
    if path.is_absolute() {
        Ok(())
    } else {
        anyhow::bail!("{label} ต้องเป็น absolute path ที่ผู้ใช้ระบุ: {}", path.display())
    }
}

fn find_scope_index(
    config: &kept_core::UserConfig,
    path: &std::path::Path,
) -> anyhow::Result<usize> {
    config
        .scopes
        .iter()
        .position(|scope| scope.path == path)
        .ok_or_else(|| anyhow::anyhow!("ไม่พบ scope '{}'", path.display()))
}

fn handle_config_scope(path: &std::path::Path, cmd: ScopeCommands) -> anyhow::Result<()> {
    let mut config = load_config_for_mutation(path)?;
    match cmd {
        ScopeCommands::List => {
            let mut scopes = config.scopes.iter().collect::<Vec<_>>();
            scopes.sort_by(|left, right| {
                right
                    .path
                    .components()
                    .count()
                    .cmp(&left.path.components().count())
                    .then_with(|| right.priority.cmp(&left.priority))
                    .then_with(|| left.path.cmp(&right.path))
            });
            println!("Resolve order\tPath\tProfile\tPriority\tRecursive");
            for (position, scope) in scopes.iter().enumerate() {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    position + 1,
                    scope.path.display(),
                    scope.profile,
                    scope.priority,
                    scope.recursive
                );
            }
            Ok(())
        }
        ScopeCommands::Show { path: scope_path } => {
            require_absolute_user_path(&scope_path, "scope path")?;
            let index = find_scope_index(&config, &scope_path)?;
            println!("{}", serde_yaml::to_string(&config.scopes[index])?);
            Ok(())
        }
        ScopeCommands::Add {
            profile,
            path: scope_path,
            priority,
            recursive,
        } => {
            require_absolute_user_path(&scope_path, "scope path")?;
            if !config.profiles.contains_key(&profile) {
                anyhow::bail!("ไม่พบ profile '{profile}'")
            }
            if config.scopes.iter().any(|scope| scope.path == scope_path) {
                anyhow::bail!(
                    "มี scope '{}' อยู่แล้ว; ใช้ kept config scope set เพื่อแก้ไข",
                    scope_path.display()
                )
            }
            config.scopes.push(kept_core::NamingScope {
                path: scope_path.clone(),
                profile,
                recursive,
                priority,
                exceptions: Vec::new(),
                overrides: kept_core::policy::ScopeOverrides::default(),
            });
            kept_core::save_user_config(path, &config)?;
            println!("Added scope: {}", scope_path.display());
            Ok(())
        }
        ScopeCommands::Set {
            path: scope_path,
            profile,
            priority,
            recursive,
        } => {
            require_absolute_user_path(&scope_path, "scope path")?;
            if let Some(profile) = &profile {
                if !config.profiles.contains_key(profile) {
                    anyhow::bail!("ไม่พบ profile '{profile}'")
                }
            }
            let index = find_scope_index(&config, &scope_path)?;
            let scope = &mut config.scopes[index];
            if let Some(profile) = profile {
                scope.profile = profile;
            }
            if let Some(priority) = priority {
                scope.priority = priority;
            }
            if let Some(recursive) = recursive {
                scope.recursive = recursive;
            }
            kept_core::save_user_config(path, &config)?;
            println!("Updated scope: {}", scope_path.display());
            Ok(())
        }
        ScopeCommands::Remove { path: scope_path } => {
            require_absolute_user_path(&scope_path, "scope path")?;
            let index = find_scope_index(&config, &scope_path)?;
            config.scopes.remove(index);
            kept_core::save_user_config(path, &config)?;
            println!("Removed scope: {}", scope_path.display());
            Ok(())
        }
        ScopeCommands::Setting { cmd } => match cmd {
            ScopeSettingCommands::Set {
                scope: scope_path,
                field,
                value,
            } => {
                require_absolute_user_path(&scope_path, "scope path")?;
                let index = find_scope_index(&config, &scope_path)?;
                set_profile_field(&mut config.scopes[index].overrides.naming, &field, &value)?;
                kept_core::save_user_config(path, &config)?;
                println!("Set scope '{}' field '{field}'", scope_path.display());
                Ok(())
            }
            ScopeSettingCommands::Unset {
                scope: scope_path,
                field,
            } => {
                require_absolute_user_path(&scope_path, "scope path")?;
                let index = find_scope_index(&config, &scope_path)?;
                unset_profile_field(&mut config.scopes[index].overrides.naming, &field)?;
                kept_core::save_user_config(path, &config)?;
                println!("Unset scope '{}' field '{field}'", scope_path.display());
                Ok(())
            }
        },
        ScopeCommands::Exception { cmd } => match cmd {
            ScopeExceptionCommands::Add {
                scope: scope_path,
                path: exception_path,
            } => {
                require_absolute_user_path(&scope_path, "scope path")?;
                require_absolute_user_path(&exception_path, "exception path")?;
                let index = find_scope_index(&config, &scope_path)?;
                let scope = &mut config.scopes[index];
                if !exception_path.starts_with(&scope.path) {
                    anyhow::bail!(
                        "exception path '{}' ต้องอยู่ใต้ scope '{}'",
                        exception_path.display(),
                        scope.path.display()
                    )
                }
                if !scope.exceptions.contains(&exception_path) {
                    scope.exceptions.push(exception_path.clone());
                }
                kept_core::save_user_config(path, &config)?;
                println!("Added scope exception: {}", exception_path.display());
                Ok(())
            }
            ScopeExceptionCommands::Remove {
                scope: scope_path,
                path: exception_path,
            } => {
                require_absolute_user_path(&scope_path, "scope path")?;
                require_absolute_user_path(&exception_path, "exception path")?;
                let index = find_scope_index(&config, &scope_path)?;
                let scope = &mut config.scopes[index];
                let position = scope
                    .exceptions
                    .iter()
                    .position(|path| path == &exception_path)
                    .ok_or_else(|| {
                        anyhow::anyhow!("scope ไม่มี exception '{}'", exception_path.display())
                    })?;
                scope.exceptions.remove(position);
                kept_core::save_user_config(path, &config)?;
                println!("Removed scope exception: {}", exception_path.display());
                Ok(())
            }
        },
    }
}

fn parse_boolean(value: &str, field: &str) -> anyhow::Result<bool> {
    value
        .parse::<bool>()
        .map_err(|_| anyhow::anyhow!("{field} ต้องเป็น true หรือ false"))
}

fn parse_usize(value: &str, field: &str) -> anyhow::Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("{field} ต้องเป็น integer ตั้งแต่ 0"))
}

fn parse_string_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

fn parse_map_field<'a>(field: &'a str, prefix: &str) -> anyhow::Result<&'a str> {
    field
        .strip_prefix(prefix)
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("field ต้องอยู่ในรูปแบบ {prefix}<name>"))
}

fn set_profile_field(
    naming: &mut kept_core::NamingSettings,
    field: &str,
    value: &str,
) -> anyhow::Result<()> {
    match field {
        "unicode" if value == "nfc" => naming.unicode = Some(value.to_string()),
        "unicode" => anyhow::bail!("unicode รองรับ nfc"),
        "case" if matches!(value, "lower" | "upper" | "preserve") => {
            naming.case = Some(value.to_string())
        }
        "case" => anyhow::bail!("case ต้องเป็น lower, upper หรือ preserve"),
        "separator" if matches!(value, "kebab" | "snake" | "preserve") => {
            naming.separator = Some(value.to_string())
        }
        "separator" => anyhow::bail!("separator ต้องเป็น kebab, snake หรือ preserve"),
        "extensions" => naming.extensions = parse_string_list(value),
        "flagControlCharacters" => {
            naming.flag_control_characters = Some(parse_boolean(value, field)?)
        }
        "flagTrimWhitespace" => naming.flag_trim_whitespace = Some(parse_boolean(value, field)?),
        "flagCaseCollisions" => naming.flag_case_collisions = Some(parse_boolean(value, field)?),
        "flagPortabilityConflicts" => {
            naming.flag_portability_conflicts = Some(parse_boolean(value, field)?)
        }
        "numbers.allow" => naming.numbers.allow = Some(parse_boolean(value, field)?),
        "numbers.maxDigitsPerToken" => {
            naming.numbers.max_digits_per_token = Some(parse_usize(value, field)?)
        }
        "words.min" => naming.words.min = Some(parse_usize(value, field)?),
        "words.max" => naming.words.max = Some(parse_usize(value, field)?),
        "length.stem.min" => {
            naming.length.stem.get_or_insert_with(Default::default).min =
                Some(parse_usize(value, field)?)
        }
        "length.stem.max" => {
            naming.length.stem.get_or_insert_with(Default::default).max =
                Some(parse_usize(value, field)?)
        }
        "whitespace.trim" => naming.whitespace.trim = Some(parse_boolean(value, field)?),
        "whitespace.collapseInternal" => {
            naming.whitespace.collapse_internal = Some(parse_boolean(value, field)?)
        }
        "prefix.required" => naming.prefix.required = Some(value.to_string()),
        "prefix.allow" => naming.prefix.allow = parse_string_list(value),
        "similarity.name.threshold" => {
            let threshold = value
                .parse::<f64>()
                .map_err(|_| anyhow::anyhow!("{field} ต้องเป็น number 0.0 ถึง 1.0"))?;
            if !(0.0..=1.0).contains(&threshold) {
                anyhow::bail!("{field} ต้องอยู่ระหว่าง 0.0 ถึง 1.0")
            }
            naming
                .similarity
                .name
                .get_or_insert(kept_core::policy::SimilarityRule {
                    threshold: 0.90,
                    case_sensitive: false,
                })
                .threshold = threshold;
        }
        "similarity.name.caseSensitive" => {
            naming
                .similarity
                .name
                .get_or_insert(kept_core::policy::SimilarityRule {
                    threshold: 0.90,
                    case_sensitive: false,
                })
                .case_sensitive = parse_boolean(value, field)?;
        }
        "replacements" => {
            naming.replacements = serde_yaml::from_str(value).map_err(|error| {
                anyhow::anyhow!("replacements ต้องเป็น YAML list ของ from/to/caseSensitive: {error}")
            })?
        }
        "reposition" => {
            naming.reposition = serde_yaml::from_str(value).map_err(|error| {
                anyhow::anyhow!("reposition ต้องเป็น YAML list ของ token/position: {error}")
            })?
        }
        "stemRegex" => naming.stem_regex = Some(value.to_string()),
        _ if field.starts_with("aliases.") => {
            naming.aliases.insert(
                parse_map_field(field, "aliases.")?.to_string(),
                value.to_string(),
            );
        }
        _ if field.starts_with("shortcuts.") => {
            naming.shortcuts.insert(
                parse_map_field(field, "shortcuts.")?.to_string(),
                value.to_string(),
            );
        }
        _ if field.starts_with("variables.") => {
            naming.variables.insert(
                parse_map_field(field, "variables.")?.to_string(),
                value.to_string(),
            );
        }
        _ => anyhow::bail!("ไม่รู้จัก naming field '{field}'; ดู kept config fields"),
    }
    Ok(())
}

fn unset_profile_field(naming: &mut kept_core::NamingSettings, field: &str) -> anyhow::Result<()> {
    match field {
        "unicode" => naming.unicode = None,
        "case" => naming.case = None,
        "separator" => naming.separator = None,
        "extensions" => naming.extensions.clear(),
        "flagControlCharacters" => naming.flag_control_characters = None,
        "flagTrimWhitespace" => naming.flag_trim_whitespace = None,
        "flagCaseCollisions" => naming.flag_case_collisions = None,
        "flagPortabilityConflicts" => naming.flag_portability_conflicts = None,
        "numbers.allow" => naming.numbers.allow = None,
        "numbers.maxDigitsPerToken" => naming.numbers.max_digits_per_token = None,
        "words.min" => naming.words.min = None,
        "words.max" => naming.words.max = None,
        "length.stem.min" => {
            if let Some(stem) = &mut naming.length.stem {
                stem.min = None;
            }
        }
        "length.stem.max" => {
            if let Some(stem) = &mut naming.length.stem {
                stem.max = None;
            }
        }
        "whitespace.trim" => naming.whitespace.trim = None,
        "whitespace.collapseInternal" => naming.whitespace.collapse_internal = None,
        "prefix.required" => naming.prefix.required = None,
        "prefix.allow" => naming.prefix.allow.clear(),
        "similarity.name.threshold" | "similarity.name.caseSensitive" => {
            naming.similarity.name = None
        }
        "replacements" => naming.replacements.clear(),
        "reposition" => naming.reposition.clear(),
        "stemRegex" => naming.stem_regex = None,
        _ if field.starts_with("aliases.") => {
            naming.aliases.remove(parse_map_field(field, "aliases.")?);
        }
        _ if field.starts_with("shortcuts.") => {
            naming
                .shortcuts
                .remove(parse_map_field(field, "shortcuts.")?);
        }
        _ if field.starts_with("variables.") => {
            naming
                .variables
                .remove(parse_map_field(field, "variables.")?);
        }
        _ => anyhow::bail!("ไม่รู้จัก naming field '{field}'; ดู kept config fields"),
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorStatus {
    Ok,
    Error,
}

#[derive(Debug)]
struct DoctorFinding {
    code: &'static str,
    status: DoctorStatus,
    message: String,
    remediation: &'static str,
}

#[derive(Debug)]
struct DoctorReport {
    findings: Vec<DoctorFinding>,
}

impl DoctorReport {
    fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.status == DoctorStatus::Error)
    }
}

fn doctor_editor_at(editor: std::ffi::OsString) -> DoctorReport {
    if editor_is_available(&editor) {
        DoctorReport {
            findings: vec![DoctorFinding {
                code: "CONFIG_EDITOR_AVAILABLE",
                status: DoctorStatus::Ok,
                message: format!(
                    "config editor ใช้งานได้: '{}'",
                    std::path::Path::new(&editor).display()
                ),
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
                    std::path::Path::new(&editor).display()
                ),
                remediation: "Install nano with your OS package manager, or set KEPT_EDITOR to an editor executable path",
            }],
        }
    }
}

fn editor_is_available(editor: &std::ffi::OsStr) -> bool {
    let editor_path = std::path::Path::new(editor);
    if editor_path.is_absolute() || editor_path.components().count() > 1 {
        return editor_path.is_file();
    }
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| directory.join(editor_path).is_file())
    })
}

/// NOTE-001: doctor อ่านและ validate config เดิมโดยไม่แก้ไข เพื่อรายงานตำแหน่งและสาเหตุที่แก้ได้จริงก่อนเสนอ --fix
/// NOTE-001: --fix สร้างได้เฉพาะกรณีไม่มี config; กรณีไฟล์มีอยู่แล้วแม้เสียหายต้อง backup ก่อนเสมอ ห้าม overwrite เงียบ
fn repair_missing_config_at(path: &std::path::Path) -> anyhow::Result<SetupResult> {
    if path.exists() {
        anyhow::bail!(
            "ไม่สามารถ fix config ที่มีอยู่ด้วยการเขียนทับ: '{}'; ใช้ kept config edit หรือ backup ไฟล์ก่อน",
            path.display()
        );
    }
    setup_user_config_at(path)
}

#[derive(Debug, PartialEq, Eq)]
struct InvalidConfigRepair {
    backup_path: PathBuf,
}

/// NOTE-001: ไฟล์ config ที่ parse ไม่ได้ต้องถูกย้ายไป backup ที่ไม่ชนก่อนสร้าง starter ใหม่ เพื่อให้ doctor --fix แก้ได้โดยไม่ทำข้อมูลผู้ใช้สูญหาย
fn repair_invalid_config_at(path: &std::path::Path) -> anyhow::Result<InvalidConfigRepair> {
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

fn next_invalid_config_backup_path(path: &std::path::Path) -> anyhow::Result<PathBuf> {
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
    unreachable!("u32 backup ordinal must eventually find an unused path")
}

#[derive(Debug)]
struct DoctorRunResult {
    report: DoctorReport,
    repaired: bool,
    backup_path: Option<PathBuf>,
}

fn run_doctor_at(path: &std::path::Path, fix: bool) -> anyhow::Result<DoctorRunResult> {
    let initial = doctor_config_at(path);
    if !fix || !initial.has_errors() {
        return Ok(DoctorRunResult {
            report: with_editor_diagnostic(initial),
            repaired: false,
            backup_path: None,
        });
    }

    let mut repaired = false;
    let mut backup_path = None;
    for finding in &initial.findings {
        match finding.code {
            "CONFIG_MISSING" => {
                repair_missing_config_at(path)?;
                repaired = true;
            }
            "CONFIG_INVALID" => {
                let repair = repair_invalid_config_at(path)?;
                backup_path = Some(repair.backup_path);
                repaired = true;
            }
            _ => {}
        }
    }
    Ok(DoctorRunResult {
        report: with_editor_diagnostic(doctor_config_at(path)),
        repaired,
        backup_path,
    })
}

fn with_editor_diagnostic(mut report: DoctorReport) -> DoctorReport {
    report
        .findings
        .extend(doctor_editor_at(resolve_config_editor(None)).findings);
    report
}

fn doctor_config_at(path: &std::path::Path) -> DoctorReport {
    let findings = if !path.is_file() {
        vec![DoctorFinding {
            code: "CONFIG_MISSING",
            status: DoctorStatus::Error,
            message: format!("config ไม่พบที่ '{}'", path.display()),
            remediation: "Run kept setup",
        }]
    } else {
        match kept_core::load_user_config(path) {
            Ok(_) => vec![DoctorFinding {
                code: "CONFIG_VALID",
                status: DoctorStatus::Ok,
                message: format!("config ใช้งานได้: '{}'", path.display()),
                remediation: "No action required",
            }],
            Err(error) => vec![DoctorFinding {
                code: "CONFIG_INVALID",
                status: DoctorStatus::Error,
                message: format!("config ใช้งานไม่ได้ที่ '{}': {error}", path.display()),
                remediation: "Run kept config edit, correct the reported YAML or scope error, then run kept doctor again",
            }],
        }
    };
    DoctorReport { findings }
}

#[derive(Debug)]
struct TaskScanResult {
    root: String,
    index_path: PathBuf,
    file_count: usize,
    total_size: u64,
    issue_count: usize,
    refresh: Option<kept_core::RefreshPlan>,
}

fn handle_task_scan(
    root: PathBuf,
    output: Option<PathBuf>,
    include_hidden: bool,
    json: bool,
) -> anyhow::Result<()> {
    let result = create_or_refresh_scan(&root, output, include_hidden)?;
    let report = serde_json::json!({
        "root": result.root,
        "indexPath": result.index_path,
        "fileCount": result.file_count,
        "totalSize": result.total_size,
        "issueCount": result.issue_count,
        "refresh": result.refresh,
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Scanned {} file(s), {} bytes, {} issue(s). Index: {}",
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

fn create_or_refresh_scan(
    root: &std::path::Path,
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
        Some(serde_json::from_str::<PersistentScanSnapshot>(
            &std::fs::read_to_string(&output)?,
        )?)
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

fn default_scan_snapshot_path(root: &str) -> PathBuf {
    use std::hash::{Hash, Hasher};

    let state_root = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    root.hash(&mut hasher);
    state_root
        .join("kept")
        .join("scans")
        .join(format!("{:016x}.json", hasher.finish()))
}

#[derive(Debug)]
struct FindRequest {
    root: PathBuf,
    file_type: Option<String>,
    name: Option<String>,
    path_contains: Option<String>,
    min_size: Option<String>,
    max_size: Option<String>,
    after: Option<u64>,
    before: Option<u64>,
    index: Option<PathBuf>,
    json: bool,
}

fn handle_task_find(request: FindRequest) -> anyhow::Result<()> {
    use kept_core::{filter_index, FileFilter, FilterSet, PersistentScanSnapshot};

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

    let mut filters = FilterSet::default();
    if let Some(file_type) = request.file_type {
        filters.all.push(FileFilter::Extension(file_type));
    }
    if let Some(name) = request.name {
        filters.all.push(FileFilter::NameContains(name));
    }
    if let Some(path_contains) = request.path_contains {
        filters.all.push(FileFilter::PathContains(path_contains));
    }
    if let Some(min_size) = request.min_size {
        filters
            .all
            .push(FileFilter::MinSize(parse_human_size(&min_size)?));
    }
    if let Some(max_size) = request.max_size {
        filters
            .all
            .push(FileFilter::MaxSize(parse_human_size(&max_size)?));
    }
    if let Some(after) = request.after {
        filters.all.push(FileFilter::ModifiedAfter(after));
    }
    if let Some(before) = request.before {
        filters.all.push(FileFilter::ModifiedBefore(before));
    }
    let records = filter_index(&snapshot.index, &filters);
    if request.json {
        println!("{}", serde_json::to_string_pretty(&records)?);
    } else {
        for record in &records {
            println!("{}\t{}", record.size, record.path);
        }
        println!(
            "Found {} of {} file(s) from index {}.",
            records.len(),
            snapshot.index.files.len(),
            snapshot_path.display()
        );
    }
    Ok(())
}

fn parse_human_size(value: &str) -> anyhow::Result<u64> {
    let normalized = value.trim().to_ascii_lowercase();
    let (number, multiplier) = if let Some(number) = normalized.strip_suffix("gb") {
        (number, 1024_u64.pow(3))
    } else if let Some(number) = normalized.strip_suffix("mb") {
        (number, 1024_u64.pow(2))
    } else if let Some(number) = normalized.strip_suffix("kb") {
        (number, 1024_u64)
    } else if let Some(number) = normalized.strip_suffix('b') {
        (number, 1)
    } else {
        (normalized.as_str(), 1)
    };
    let number = number
        .trim()
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("ขนาด '{}' ต้องเป็น byte หรือใช้ suffix kb, mb, gb", value))?;
    number
        .checked_mul(multiplier)
        .ok_or_else(|| anyhow::anyhow!("ขนาด '{}' ใหญ่เกินขอบเขต", value))
}

fn handle_task_review(
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
            "1. ดูก้อนพื้นที่ใหญ่",
            "2. หาไฟล์ด้วยตัวช่วยกรอง",
            "3. ตรวจ duplicate จาก index นี้",
            "4. ดู scan issues",
            "5. ตรวจ naming policy",
            "0. จบ",
        ];
        let selected = Select::new()
            .with_prompt("เลือกการทำงาน (ใช้ ↑/↓ หรือ j/k แล้วกด Enter)")
            .items(&choices)
            .default(0)
            .interact_opt()?;
        let action = selected.and_then(|position| {
            scan_review_action_from_selection(match position {
                0 => "1",
                1 => "2",
                2 => "3",
                3 => "4",
                4 => "5",
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
            ScanReviewAction::Issues => println!("{}", scan_review_summary(&snapshot.index)),
            ScanReviewAction::Naming => println!("{}", naming_review_summary(&snapshot.index)),
        }
    }
}

fn run_interactive_find(root: PathBuf) -> anyhow::Result<()> {
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

fn prompt_optional(prompt: &str) -> anyhow::Result<Option<String>> {
    let value: String = Input::new()
        .with_prompt(prompt)
        .allow_empty(true)
        .interact_text()?;
    Ok((!value.trim().is_empty()).then_some(value))
}

fn handle_task_duplicates(
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

fn prepare_imported_registry(
    registry: kept_core::schema::KeywordRegistry,
) -> anyhow::Result<kept_core::schema::KeywordRegistry> {
    kept_core::migrate_registry(registry).map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn handle_registry(cmd: RegistryCommands) -> anyhow::Result<()> {
    use kept_core::{
        import_csv, load_registry, save_registry, KeywordSearch, RegistryAnalyzer, Validator,
    };

    match cmd {
        RegistryCommands::Validate {
            path,
            entry_id,
            group,
        } => {
            let registry =
                load_registry(path).map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let validator = Validator::new(registry);
            if let Some(id) = entry_id {
                let target_group = validator
                    .registry()
                    .groups
                    .iter()
                    .filter(|candidate| {
                        group
                            .as_deref()
                            .is_none_or(|name| candidate.group_id == name)
                    })
                    .find(|candidate| {
                        candidate.entries.iter().any(|entry| {
                            entry.get("id").and_then(|value| value.as_str()) == Some(id.as_str())
                        })
                    })
                    .ok_or_else(|| anyhow::anyhow!("ไม่พบ entry '{id}' ใน registry"))?;
                let entry = target_group
                    .entries
                    .iter()
                    .find(|entry| {
                        entry.get("id").and_then(|value| value.as_str()) == Some(id.as_str())
                    })
                    .expect("entry was located in its group");
                validator
                    .validate_entry(&target_group.group_id, entry)
                    .map_err(|errors| anyhow::anyhow!("{:?}", errors))?;
                println!(
                    "Entry '{id}' is valid in group '{}'.",
                    target_group.group_id
                );
            } else {
                validator
                    .validate_registry()
                    .map_err(|errors| anyhow::anyhow!("{:?}", errors))?;
                println!("Registry is valid!");
            }
        }
        RegistryCommands::Search {
            path,
            query,
            group,
            json,
        } => {
            let registry =
                load_registry(path).map_err(|error| anyhow::anyhow!(error.to_string()))?;
            Validator::new(registry.clone())
                .validate_registry()
                .map_err(|errors| anyhow::anyhow!("{:?}", errors))?;
            let search = KeywordSearch::new(registry);
            let results = search.search(&query, group.as_deref());
            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                for r in results {
                    println!("{}: {} (score: {:.2})", r.id, r.description, r.score);
                }
            }
        }
        RegistryCommands::Analyze { path, json } => {
            let mut registry =
                load_registry(path).map_err(|error| anyhow::anyhow!(error.to_string()))?;
            RegistryAnalyzer::build_index_and_stats(&mut registry);
            if json {
                println!("{}", serde_json::to_string_pretty(&registry)?);
            } else {
                println!("Analysis complete for {}", registry.metadata.description);
            }
        }
        RegistryCommands::Import {
            csv,
            group_id,
            group_name,
            output,
        } => {
            let legacy_registry = import_csv(csv, &group_id, &group_name)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let registry = prepare_imported_registry(legacy_registry)?;
            save_registry(output, &registry).map_err(|error| anyhow::anyhow!(error.to_string()))?;
            println!("Imported successfully!");
        }
    }
    Ok(())
}

fn handle_fs(cmd: FsCommands) -> anyhow::Result<()> {
    use kept_core::{build_treemap, filter_index, scan_directory, FilterSet, ScanOptions};

    match cmd {
        FsCommands::Duplicates { cmd } => handle_duplicate_scan(cmd)?,
        FsCommands::Index {
            root,
            output,
            include_hidden,
        } => {
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
                println!(
                    "Matched {} of {} file(s).",
                    records.len(),
                    index_data.files.len()
                );
            }
        }
    }
    Ok(())
}

fn handle_duplicate_scan(cmd: DuplicateCommands) -> anyhow::Result<()> {
    use kept_core::{
        find_content_duplicates, scan_directory, ContentDuplicateOptions, ScanOptions,
    };

    let DuplicateCommands::Scan {
        root,
        json,
        action,
        yes,
    } = cmd;
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
            println!(
                "{}. {} ({} file(s))",
                position + 1,
                group.kind,
                group.items.len()
            );
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
        return run_duplicate_action(action, &root, &report, yes);
    }
    if !json && is_interactive_terminal() {
        if let Some(action) = choose_duplicate_action()? {
            run_duplicate_action(action, &root, &report, false)?;
        }
    }
    Ok(())
}

fn build_scan_space_summary(index: &kept_core::ScanIndex) -> Vec<(String, u64)> {
    let mut sizes = std::collections::BTreeMap::<String, u64>::new();
    for record in &index.files {
        let segment = record.path.split('/').next().unwrap_or(&record.path);
        *sizes.entry(segment.to_string()).or_default() += record.size;
    }
    let mut summary = sizes.into_iter().collect::<Vec<_>>();
    summary.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    summary
}

/// NOTE-001: review อ่าน config.yaml ที่ผู้ใช้เป็นเจ้าของและ ScanIndex เดิมเท่านั้น; ไม่สร้าง scope, ไม่เขียน config และไม่เปลี่ยนชื่อไฟล์
fn naming_review_summary(index: &kept_core::ScanIndex) -> String {
    match kept_core::default_user_config_path() {
        Ok(path) => naming_review_summary_at(index, &path),
        Err(error) => format!(
            "Naming policy unavailable: cannot resolve config path ({error}); run kept doctor"
        ),
    }
}

fn naming_review_summary_at(index: &kept_core::ScanIndex, config_path: &std::path::Path) -> String {
    if !config_path.is_file() {
        return format!(
            "Naming policy: no config at '{}'; run kept setup",
            config_path.display()
        );
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
        let target = finding.proposed_target.as_deref().map_or_else(
            || "no target".to_string(),
            |name| format!("proposed: {name}"),
        );
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

fn scan_review_summary(index: &kept_core::ScanIndex) -> String {
    let mut lines = vec![format!(
        "{} file(s), {} bytes, {} scan issue(s)",
        index.files.len(),
        index.total_size,
        index.issues.len()
    )];
    for issue in &index.issues {
        lines.push(format!(
            "- {}: {} ({})",
            issue.path, issue.message, issue.operation
        ));
    }
    lines.join("\n")
}

fn scan_review_action_from_selection(selection: &str) -> Option<ScanReviewAction> {
    match selection.trim() {
        "1" => Some(ScanReviewAction::Space),
        "2" => Some(ScanReviewAction::Find),
        "3" => Some(ScanReviewAction::Duplicates),
        "4" => Some(ScanReviewAction::Issues),
        "5" => Some(ScanReviewAction::Naming),
        "0" | "q" | "" => None,
        _ => None,
    }
}

fn choose_duplicate_action() -> anyhow::Result<Option<DuplicateAction>> {
    let choices = [
        "1. ดูรายงาน JSON อีกครั้ง",
        "2. export duplicate plan",
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
            _ => "0",
        })
    }))
}

fn duplicate_action_from_selection(selection: &str) -> Option<DuplicateAction> {
    match selection.trim().to_ascii_lowercase().as_str() {
        "1" => Some(DuplicateAction::Show),
        "2" => Some(DuplicateAction::ExportPlan),
        "0" | "q" | "" => None,
        _ => None,
    }
}

fn run_duplicate_action(
    action: DuplicateAction,
    root: &std::path::Path,
    report: &serde_json::Value,
    yes: bool,
) -> anyhow::Result<()> {
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
    }
}

fn confirm_action(description: &str) -> anyhow::Result<bool> {
    print!("{description} ใช่หรือไม่? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

fn parse_file_filter(raw: &str) -> anyhow::Result<kept_core::FileFilter> {
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

    match (
        field.to_ascii_lowercase().as_str(),
        operator.to_ascii_lowercase().as_str(),
    ) {
        ("extension" | "ext", "eq") => Ok(FileFilter::Extension(value.to_string())),
        ("path", "contains") => Ok(FileFilter::PathContains(value.to_string())),
        ("name", "contains") => Ok(FileFilter::NameContains(value.to_string())),
        ("size", "ge") => Ok(FileFilter::MinSize(parse_filter_number(field, value)?)),
        ("size", "le") => Ok(FileFilter::MaxSize(parse_filter_number(field, value)?)),
        ("modified" | "modifiedunix", "after" | "ge") => Ok(FileFilter::ModifiedAfter(
            parse_filter_number(field, value)?,
        )),
        ("modified" | "modifiedunix", "before" | "le") => Ok(FileFilter::ModifiedBefore(
            parse_filter_number(field, value)?,
        )),
        ("kind", "eq") => Ok(FileFilter::Kind(value.to_string())),
        _ => anyhow::bail!("operator '{operator}' ใช้กับ field '{field}' ไม่ได้"),
    }
}

fn default_filter_operator(field: &str) -> anyhow::Result<&'static str> {
    match field.to_ascii_lowercase().as_str() {
        "extension" | "ext" | "kind" => Ok("eq"),
        "path" | "name" => Ok("contains"),
        _ => anyhow::bail!("field '{field}' ต้องระบุ operator เช่น size:ge:1048576"),
    }
}

fn parse_filter_number(field: &str, value: &str) -> anyhow::Result<u64> {
    value
        .parse()
        .map_err(|_| anyhow::anyhow!("ค่า '{value}' ของ field '{field}' ต้องเป็นจำนวนเต็มบวก"))
}

fn parse_custom_filter(raw: &str) -> anyhow::Result<kept_core::CustomFilter> {
    use kept_core::{CustomFilter, CustomOperator};

    let parts = raw.splitn(3, ':').collect::<Vec<_>>();
    let [field, operator, value] = parts.as_slice() else {
        anyhow::bail!("รูปแบบ custom filter ไม่ถูกต้อง '{raw}'; ใช้ field:operator:value");
    };
    if field.is_empty() || value.is_empty() {
        anyhow::bail!("field และ value ของ custom filter ต้องไม่ว่าง");
    }
    let operator = match operator.to_ascii_lowercase().as_str() {
        "eq" => CustomOperator::Equals,
        "contains" => CustomOperator::Contains,
        "prefix" => CustomOperator::StartsWith,
        "suffix" => CustomOperator::EndsWith,
        "ge" => CustomOperator::GreaterOrEqual,
        "le" => CustomOperator::LessOrEqual,
        _ => anyhow::bail!(
            "custom operator '{operator}' ไม่รองรับ; ใช้ eq, contains, prefix, suffix, ge หรือ le"
        ),
    };
    Ok(CustomFilter {
        field: (*field).to_string(),
        operator,
        value: (*value).to_string(),
    })
}

async fn handle_doc(cmd: DocCommands) -> anyhow::Result<()> {
    match cmd {
        DocCommands::Sync { dir, notion_db } => anyhow::bail!(
            "doc sync สำหรับ '{}' ไปยัง Notion database '{}' ยังไม่พร้อมใช้งาน: live source/sink ยังไม่ถูก implement; ใช้ doc convert สำหรับ offline conversion ได้",
            dir.display(),
            notion_db
        ),
        DocCommands::Convert { input, output } => {
            use kept_doc::converter::markdown::MarkdownConverter;
            use kept_doc::{FromPlatform, ToPlatform};

            let input_format = input
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(input_format.as_str(), "md" | "markdown") {
                anyhow::bail!("doc convert รองรับ input Markdown (.md) เท่านั้นในรุ่นนี้");
            }

            let markdown = tokio::fs::read_to_string(&input).await?;
            let document = MarkdownConverter::from_platform(markdown)
                .map_err(|error| anyhow::anyhow!("แปลง Markdown เป็น Universal IR ไม่สำเร็จ: {error}"))?;
            let output_format = output
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let rendered = match output_format.as_str() {
                "json" => serde_json::to_string_pretty(&document)?,
                "md" | "markdown" => MarkdownConverter::to_platform(&document)
                    .map_err(|error| anyhow::anyhow!("แปลง Universal IR เป็น Markdown ไม่สำเร็จ: {error}"))?,
                _ => anyhow::bail!("doc convert รองรับ output .json, .md หรือ .markdown"),
            };
            tokio::fs::write(&output, rendered).await?;
            println!("Converted {} -> {}", input.display(), output.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_scan_space_summary, create_or_refresh_scan, default_scan_snapshot_path,
        doctor_config_at, doctor_editor_at, duplicate_action_from_selection,
        handle_task_duplicates, handle_task_find, handle_task_review, naming_review_summary_at,
        parse_custom_filter, parse_file_filter, repair_invalid_config_at, repair_missing_config_at,
        resolve_config_editor, run_doctor_at, scan_review_action_from_selection,
        scan_review_summary, set_profile_field, setup_action_from_selection, setup_user_config_at,
        unset_profile_field, Cli, DoctorStatus, DuplicateAction, FindRequest, ScanReviewAction,
        SetupAction,
    };
    use clap::{CommandFactory, Parser};
    use kept_core::{CustomOperator, FileFilter, FileRecord, ScanIndex, ScanIssue};

    #[test]
    fn parses_default_and_numeric_file_filters() {
        assert!(matches!(
            parse_file_filter("extension:md").unwrap(),
            FileFilter::Extension(value) if value == "md"
        ));
        assert!(matches!(
            parse_file_filter("size:ge:1048576").unwrap(),
            FileFilter::MinSize(1_048_576)
        ));
        assert!(matches!(
            parse_file_filter("modified:before:1700000000").unwrap(),
            FileFilter::ModifiedBefore(1_700_000_000)
        ));
    }

    #[test]
    fn rejects_ambiguous_file_filter() {
        assert!(parse_file_filter("size:100").is_err());
        assert!(parse_file_filter("unknown:value").is_err());
    }

    #[test]
    fn task_first_scan_reports_refresh_delta_after_rescan() {
        let root =
            std::env::temp_dir().join(format!("kept-task-scan-refresh-{}", std::process::id()));
        let snapshot_path = root.with_extension("scan.json");
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        std::fs::write(root.join("first.txt"), "first")
            .expect("first fixture file must be written");

        let first = create_or_refresh_scan(&root, Some(snapshot_path.clone()), false)
            .expect("first scan must succeed");
        assert!(
            first.refresh.is_none(),
            "first scan has no prior refresh delta"
        );

        std::fs::write(root.join("second.txt"), "second")
            .expect("second fixture file must be written");
        let second = create_or_refresh_scan(&root, Some(snapshot_path.clone()), false)
            .expect("second scan must succeed");

        assert_eq!(
            second
                .refresh
                .expect("rescan must describe its delta")
                .added,
            vec!["second.txt"]
        );
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
        std::fs::remove_file(snapshot_path).expect("fixture snapshot must be removed");
    }

    #[test]
    fn task_first_find_reads_the_existing_scan_index() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root = std::env::temp_dir().join(format!("kept-task-find-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        std::fs::write(root.join("report.pdf"), "fixture").expect("fixture file must be written");
        let options = ScanOptions::default();
        let index = scan_directory(&root, &options).expect("fixture must scan");
        let snapshot_path = default_scan_snapshot_path(&index.root);
        let previous = std::fs::read(&snapshot_path).ok();
        std::fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("snapshot path must have a parent"),
        )
        .expect("snapshot directory must be created");
        std::fs::write(
            &snapshot_path,
            serde_json::to_string(&create_persistent_snapshot(index, &options))
                .expect("snapshot must serialize"),
        )
        .expect("snapshot must be written");

        let result = handle_task_find(FindRequest {
            root: root.clone(),
            file_type: Some("pdf".to_string()),
            name: Some("report".to_string()),
            path_contains: None,
            min_size: None,
            max_size: None,
            after: None,
            before: None,
            index: None,
            json: true,
        });

        match previous {
            Some(bytes) => {
                std::fs::write(&snapshot_path, bytes).expect("existing snapshot must be restored")
            }
            None => std::fs::remove_file(&snapshot_path).expect("fixture snapshot must be removed"),
        }
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
        assert!(result.is_ok(), "find must reuse the persisted scan index");
    }

    #[test]
    fn task_first_find_uses_user_selected_portable_index() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root =
            std::env::temp_dir().join(format!("kept-task-find-portable-{}", std::process::id()));
        let snapshot_path = root.with_extension("portable.json");
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        std::fs::write(root.join("report.pdf"), "fixture").expect("fixture file must be written");
        let options = ScanOptions::default();
        let index = scan_directory(&root, &options).expect("fixture must scan");
        std::fs::write(
            &snapshot_path,
            serde_json::to_string(&create_persistent_snapshot(index, &options))
                .expect("snapshot must serialize"),
        )
        .expect("portable snapshot must be written");

        let result = handle_task_find(FindRequest {
            root: root.clone(),
            file_type: Some("pdf".to_string()),
            name: None,
            path_contains: None,
            min_size: None,
            max_size: None,
            after: None,
            before: None,
            index: Some(snapshot_path.clone()),
            json: true,
        });

        std::fs::remove_dir_all(root).expect("fixture root must be removed");
        std::fs::remove_file(snapshot_path).expect("portable snapshot must be removed");
        assert!(
            result.is_ok(),
            "find must use the index explicitly selected by the user"
        );
    }

    #[test]
    fn task_first_review_reads_the_existing_scan_index() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root = std::env::temp_dir().join(format!("kept-task-review-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        std::fs::write(root.join("report.txt"), "fixture").expect("fixture file must be written");
        let options = ScanOptions::default();
        let index = scan_directory(&root, &options).expect("fixture must scan");
        let snapshot_path = default_scan_snapshot_path(&index.root);
        let previous = std::fs::read(&snapshot_path).ok();
        std::fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("snapshot path must have a parent"),
        )
        .expect("snapshot directory must be created");
        std::fs::write(
            &snapshot_path,
            serde_json::to_string(&create_persistent_snapshot(index, &options))
                .expect("snapshot must serialize"),
        )
        .expect("snapshot must be written");

        let result = handle_task_review(root.clone(), None, false);

        match previous {
            Some(bytes) => {
                std::fs::write(&snapshot_path, bytes).expect("existing snapshot must be restored")
            }
            None => std::fs::remove_file(&snapshot_path).expect("fixture snapshot must be removed"),
        }
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
        assert!(result.is_ok(), "review must use the persisted scan index");
    }

    #[test]
    fn review_renders_read_only_naming_findings_from_existing_config_and_index() {
        let directory =
            std::env::temp_dir().join(format!("kept-review-naming-{}", std::process::id()));
        let config_path = directory.join("config.yaml");
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("review fixture directory must be created");
        std::fs::write(
            &config_path,
            format!(
                "version: 1\ndefaults:\n  naming: {{}}\nprofiles:\n  reports:\n    naming:\n      case: lower\n      separator: kebab\n      extensions: [pdf]\nscopes:\n  - path: {}\n    profile: reports\n    recursive: true\n    priority: 100\n",
                directory.display()
            ),
        )
        .expect("review config fixture must be written");
        let index = ScanIndex {
            root: directory.display().to_string(),
            scanned_at_unix: 0,
            total_size: 1,
            files: vec![FileRecord {
                path: "Q1_Report.pdf".to_string(),
                name: "Q1_Report.pdf".to_string(),
                extension: "pdf".to_string(),
                size: 1,
                modified_unix: 0,
                kind: "file".to_string(),
            }],
            issues: Vec::new(),
        };

        let summary = naming_review_summary_at(&index, &config_path);

        assert!(summary.contains("Naming policy: 1 finding(s)"));
        assert!(summary.contains("Q1_Report.pdf"));
        assert!(summary.contains("q1-report.pdf"));
        assert!(!summary.contains("rename applied"));
        std::fs::remove_dir_all(directory).expect("review fixture directory must be removed");
    }

    #[test]
    fn task_first_duplicates_reads_the_existing_scan_index() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root =
            std::env::temp_dir().join(format!("kept-task-duplicates-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        std::fs::write(root.join("left.txt"), "same fixture")
            .expect("first fixture file must be written");
        std::fs::write(root.join("right.txt"), "same fixture")
            .expect("second fixture file must be written");
        let options = ScanOptions::default();
        let index = scan_directory(&root, &options).expect("fixture must scan");
        let snapshot_path = default_scan_snapshot_path(&index.root);
        let previous = std::fs::read(&snapshot_path).ok();
        std::fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("snapshot path must have a parent"),
        )
        .expect("snapshot directory must be created");
        std::fs::write(
            &snapshot_path,
            serde_json::to_string(&create_persistent_snapshot(index, &options))
                .expect("snapshot must serialize"),
        )
        .expect("snapshot must be written");

        let result = handle_task_duplicates(root.clone(), None, true, None, false);

        match previous {
            Some(bytes) => {
                std::fs::write(&snapshot_path, bytes).expect("existing snapshot must be restored")
            }
            None => std::fs::remove_file(&snapshot_path).expect("fixture snapshot must be removed"),
        }
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
        assert!(
            result.is_ok(),
            "duplicates must reuse the persisted scan index before hashing candidates"
        );
    }

    #[test]
    fn parses_task_first_scan_with_optional_index_output() {
        let command = Cli::try_parse_from([
            "kept",
            "scan",
            "/tmp/kept-fixtures",
            "--output",
            "scan-index.json",
            "--json",
        ]);
        assert!(command.is_ok(), "task-first scan command must parse");
    }

    #[test]
    fn parses_task_first_find_with_filter_facts() {
        let command = Cli::try_parse_from([
            "kept",
            "find",
            "/tmp/kept-fixtures",
            "--type",
            "pdf",
            "--min-size",
            "50mb",
            "--name",
            "report",
            "--path",
            "archive",
            "--max-size",
            "100mb",
            "--after",
            "1704067200",
            "--before",
            "1735689600",
            "--index",
            "/tmp/kept-portable-scan.json",
        ]);
        assert!(command.is_ok(), "task-first find command must parse");
    }

    #[test]
    fn parses_interactive_setup_command() {
        let command = Cli::try_parse_from(["kept", "setup"]);
        assert!(command.is_ok(), "setup must be a user-facing command");
    }

    #[test]
    fn setup_creates_starter_config_once_and_preserves_user_owned_content() {
        let directory =
            std::env::temp_dir().join(format!("kept-setup-contract-{}", std::process::id()));
        let path = directory.join("config.yaml");
        std::fs::remove_dir_all(&directory).ok();

        let first = setup_user_config_at(&path).expect("setup must create starter config");
        let original = std::fs::read_to_string(&path).expect("starter config must be written");
        std::fs::write(
            &path,
            "version: 1\ndefaults: {}\nprofiles: {}\nscopes: []\n",
        )
        .expect("user edit fixture must be written");
        let second = setup_user_config_at(&path).expect("setup must not overwrite user config");
        let preserved = std::fs::read_to_string(&path).expect("user config must remain readable");

        assert!(first.created, "first setup must create config");
        assert_eq!(first.path, path);
        assert!(original.contains("config.yaml is user-owned"));
        assert!(!second.created, "second setup must not overwrite config");
        assert_eq!(
            preserved,
            "version: 1\ndefaults: {}\nprofiles: {}\nscopes: []\n"
        );
        std::fs::remove_dir_all(directory).expect("temporary setup fixture must be removed");
    }

    #[test]
    fn parses_config_edit_command() {
        let command = Cli::try_parse_from(["kept", "config", "edit"]);
        assert!(command.is_ok(), "config edit must be a user-facing command");
    }

    #[test]
    fn parses_config_fields_command() {
        let command = Cli::try_parse_from(["kept", "config", "fields"]);
        assert!(
            command.is_ok(),
            "config fields must expose supported setting keys"
        );
    }

    #[test]
    fn documented_naming_fields_can_be_set_and_unset_without_yaml_editing() {
        let mut naming = kept_core::NamingSettings::default();
        let cases = [
            ("unicode", "nfc"),
            ("case", "lower"),
            ("separator", "kebab"),
            ("extensions", "pdf,docx"),
            ("flagControlCharacters", "true"),
            ("flagTrimWhitespace", "true"),
            ("flagCaseCollisions", "true"),
            ("flagPortabilityConflicts", "true"),
            ("numbers.allow", "false"),
            ("numbers.maxDigitsPerToken", "6"),
            ("words.min", "1"),
            ("words.max", "4"),
            ("length.stem.min", "2"),
            ("length.stem.max", "80"),
            ("whitespace.trim", "true"),
            ("whitespace.collapseInternal", "false"),
            ("prefix.required", "kept"),
            ("prefix.allow", "kept,archive"),
            ("aliases.rpt", "report"),
            ("shortcuts.inv", "invoice"),
            ("variables.project", "kept"),
            ("similarity.name.threshold", "0.85"),
            ("similarity.name.caseSensitive", "true"),
            (
                "replacements",
                "- from: final\n  to: draft\n  caseSensitive: true",
            ),
            ("reposition", "- token: 2026\n  position: back"),
            ("stemRegex", "^[a-z0-9-]+$"),
        ];

        for (field, value) in cases {
            set_profile_field(&mut naming, field, value)
                .unwrap_or_else(|error| panic!("set {field} failed: {error}"));
        }
        for (field, _) in cases {
            unset_profile_field(&mut naming, field)
                .unwrap_or_else(|error| panic!("unset {field} failed: {error}"));
        }
    }

    #[test]
    fn maps_interactive_setup_choices_without_implicit_config_mutation() {
        assert!(matches!(
            setup_action_from_selection("1"),
            Some(SetupAction::EditConfig)
        ));
        assert!(matches!(
            setup_action_from_selection("0"),
            Some(SetupAction::Exit)
        ));
        assert!(setup_action_from_selection("q").is_none());
    }

    #[test]
    fn config_editor_defaults_to_nano_and_accepts_explicit_editor_path() {
        assert_eq!(
            resolve_config_editor(None),
            std::ffi::OsString::from("nano")
        );
        assert_eq!(
            resolve_config_editor(Some(std::ffi::OsString::from("/usr/bin/vim"))),
            std::ffi::OsString::from("/usr/bin/vim")
        );
    }

    #[test]
    fn doctor_run_includes_the_config_editor_diagnostic() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-editor-{}", std::process::id()));
        let path = directory.join("config.yaml");
        std::fs::remove_dir_all(&directory).ok();
        setup_user_config_at(&path).expect("fixture config must be created");

        let result = run_doctor_at(&path, false).expect("doctor must run");

        assert!(result.report.findings.iter().any(|finding| {
            matches!(
                finding.code,
                "CONFIG_EDITOR_AVAILABLE" | "CONFIG_EDITOR_MISSING"
            )
        }));
        std::fs::remove_dir_all(directory).expect("temporary doctor fixture must be removed");
    }

    #[test]
    fn doctor_reports_a_missing_editor_with_a_concrete_recovery_path() {
        let report = doctor_editor_at(std::ffi::OsString::from("kept-editor-missing-contract"));
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.code == "CONFIG_EDITOR_MISSING")
            .expect("missing editor must be diagnosed");

        assert_eq!(finding.status, DoctorStatus::Error);
        assert!(finding.remediation.contains("KEPT_EDITOR"));
    }

    #[test]
    fn doctor_reports_a_missing_config_with_its_path_and_recovery_command() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-missing-{}", std::process::id()));
        let path = directory.join("config.yaml");
        std::fs::remove_dir_all(&directory).ok();

        let report = doctor_config_at(&path);

        assert!(report.has_errors());
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.code == "CONFIG_MISSING")
            .expect("missing config must be diagnosed");
        assert!(finding.message.contains(&path.display().to_string()));
        assert_eq!(finding.remediation, "Run kept setup");
    }

    #[test]
    fn doctor_fix_creates_a_missing_config_then_reports_it_valid() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-fix-{}", std::process::id()));
        let path = directory.join("config.yaml");
        std::fs::remove_dir_all(&directory).ok();

        let repair = repair_missing_config_at(&path).expect("doctor --fix must create config");
        let report = doctor_config_at(&path);

        assert!(repair.created);
        assert!(path.is_file());
        assert!(!report.has_errors());
        std::fs::remove_dir_all(directory).expect("temporary doctor fixture must be removed");
    }

    #[test]
    fn doctor_fix_backs_up_an_invalid_config_before_restoring_a_valid_starter() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-invalid-{}", std::process::id()));
        let path = directory.join("config.yaml");
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("temporary doctor fixture must be created");
        let invalid_yaml = "version: [broken\n";
        std::fs::write(&path, invalid_yaml).expect("invalid config fixture must be written");

        let repair =
            repair_invalid_config_at(&path).expect("doctor --fix must repair invalid config");
        let report = doctor_config_at(&path);

        assert_eq!(
            std::fs::read_to_string(&repair.backup_path).expect("backup must be readable"),
            invalid_yaml
        );
        assert!(path.is_file());
        assert!(!report.has_errors());
        std::fs::remove_dir_all(directory).expect("temporary doctor fixture must be removed");
    }

    #[test]
    fn doctor_run_with_fix_resolves_a_missing_config_from_its_own_diagnostic() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-run-{}", std::process::id()));
        let path = directory.join("config.yaml");
        std::fs::remove_dir_all(&directory).ok();

        let result = run_doctor_at(&path, true).expect("doctor --fix must run");

        assert!(result.repaired);
        assert!(!doctor_config_at(&path).has_errors());
        std::fs::remove_dir_all(directory).expect("temporary doctor fixture must be removed");
    }

    #[test]
    fn parses_doctor_fix_command() {
        let command = Cli::try_parse_from(["kept", "doctor", "--fix"]);
        assert!(
            command.is_ok(),
            "doctor --fix must be a user-facing command"
        );
    }

    #[test]
    fn root_help_hides_internal_note_markers_from_users() {
        let help = Cli::command().render_help().to_string();

        assert!(!help.contains("NOTE-001"));
        assert!(help.contains("สร้าง config.yaml"));
    }

    #[test]
    fn root_help_prioritizes_task_first_commands() {
        let help = Cli::command().render_help().to_string();
        for command in [
            "scan",
            "find",
            "review",
            "duplicates",
            "search",
            "convert",
            "setup",
            "config",
            "doctor",
        ] {
            assert!(help.contains(command), "root help must show {command}");
        }
        assert!(
            !help.contains("\n  fs"),
            "root help must hide legacy fs hierarchy"
        );
        assert!(
            !help.contains("\n  registry"),
            "root help must hide legacy registry hierarchy"
        );
        assert!(
            !help
                .lines()
                .any(|line| line.split_whitespace().next() == Some("doc")),
            "root help must hide legacy doc hierarchy"
        );
        assert!(
            !help.contains("\n  policy"),
            "internal policy must not become a user-facing command"
        );
    }

    #[test]
    fn parses_task_first_search_without_registry_hierarchy() {
        let command = Cli::try_parse_from([
            "kept",
            "search",
            "registry.json",
            "รายงาน",
            "--group",
            "product_terms",
        ]);
        assert!(command.is_ok(), "root-level search command must parse");
    }

    #[test]
    fn parses_task_first_convert_without_doc_hierarchy() {
        let command = Cli::try_parse_from(["kept", "convert", "source.md", "result.json"]);
        assert!(command.is_ok(), "root-level convert command must parse");
    }

    #[test]
    fn parses_task_first_review_for_existing_index() {
        let command = Cli::try_parse_from([
            "kept",
            "review",
            "/tmp/kept-fixtures",
            "--index",
            "/tmp/kept-portable-scan.json",
        ]);
        assert!(command.is_ok(), "review must parse with an indexed root");
    }

    #[test]
    fn parses_task_first_duplicates_with_non_interactive_export() {
        let command = Cli::try_parse_from([
            "kept",
            "duplicates",
            "/tmp/kept-fixtures",
            "--index",
            "/tmp/kept-portable-scan.json",
            "--action",
            "export-plan",
            "--yes",
        ]);
        assert!(
            command.is_ok(),
            "task-first duplicates command must parse with explicit action"
        );
    }

    #[test]
    fn parses_duplicate_scan_command_with_json_output() {
        let command = Cli::try_parse_from([
            "kept",
            "fs",
            "duplicates",
            "scan",
            "/tmp/kept-fixtures",
            "--json",
        ]);
        assert!(command.is_ok(), "duplicate scan command must parse");
    }

    #[test]
    fn parses_non_interactive_duplicate_action_with_explicit_yes() {
        let command = Cli::try_parse_from([
            "kept",
            "fs",
            "duplicates",
            "scan",
            "/tmp/kept-fixtures",
            "--action",
            "export-plan",
            "--yes",
        ]);
        assert!(
            command.is_ok(),
            "non-interactive duplicate action must parse"
        );
    }

    #[test]
    fn scan_space_review_sorts_largest_top_level_nodes() {
        let index = ScanIndex {
            root: "/fixture".to_string(),
            scanned_at_unix: 1,
            total_size: 300,
            files: vec![
                FileRecord {
                    path: "small/a.txt".to_string(),
                    name: "a.txt".to_string(),
                    extension: "txt".to_string(),
                    size: 100,
                    modified_unix: 1,
                    kind: "file".to_string(),
                },
                FileRecord {
                    path: "large/b.bin".to_string(),
                    name: "b.bin".to_string(),
                    extension: "bin".to_string(),
                    size: 200,
                    modified_unix: 1,
                    kind: "file".to_string(),
                },
            ],
            issues: Vec::new(),
        };

        let summary = build_scan_space_summary(&index);

        assert_eq!(summary[0], ("large".to_string(), 200));
        assert_eq!(summary[1], ("small".to_string(), 100));
    }

    #[test]
    fn scan_review_summary_exposes_space_and_scan_issues() {
        let index = ScanIndex {
            root: "/fixture".to_string(),
            scanned_at_unix: 1,
            total_size: 128,
            files: vec![FileRecord {
                path: "report.txt".to_string(),
                name: "report.txt".to_string(),
                extension: "txt".to_string(),
                size: 128,
                modified_unix: 1,
                kind: "file".to_string(),
            }],
            issues: vec![ScanIssue {
                path: "locked".to_string(),
                operation: "read_dir".to_string(),
                message: "permission denied".to_string(),
            }],
        };

        let summary = scan_review_summary(&index);

        assert!(summary.contains("1 file(s)"));
        assert!(summary.contains("128 bytes"));
        assert!(summary.contains("1 scan issue(s)"));
        assert!(summary.contains("locked"));
    }

    #[test]
    fn maps_scan_review_menu_choices() {
        assert_eq!(
            scan_review_action_from_selection("1"),
            Some(ScanReviewAction::Space)
        );
        assert_eq!(
            scan_review_action_from_selection("2"),
            Some(ScanReviewAction::Find)
        );
        assert_eq!(
            scan_review_action_from_selection("3"),
            Some(ScanReviewAction::Duplicates)
        );
        assert_eq!(
            scan_review_action_from_selection("4"),
            Some(ScanReviewAction::Issues)
        );
        assert_eq!(
            scan_review_action_from_selection("5"),
            Some(ScanReviewAction::Naming)
        );
        assert_eq!(scan_review_action_from_selection("0"), None);
    }

    #[test]
    fn maps_numeric_duplicate_menu_choices() {
        assert!(matches!(
            duplicate_action_from_selection("1"),
            Some(DuplicateAction::Show)
        ));
        assert!(matches!(
            duplicate_action_from_selection("2"),
            Some(DuplicateAction::ExportPlan)
        ));
        assert!(duplicate_action_from_selection("0").is_none());
        assert!(duplicate_action_from_selection("q").is_none());
    }

    #[test]
    fn parses_custom_filter() {
        let filter = parse_custom_filter("name:prefix:report").unwrap();
        assert_eq!(filter.field, "name");
        assert!(matches!(filter.operator, CustomOperator::StartsWith));
        assert_eq!(filter.value, "report");
    }
}

#[cfg(test)]
mod release_metadata_tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn cli_metadata_uses_foundation_release_version() {
        assert_eq!(Cli::command().get_version(), Some("0.2.0"));
    }
}

#[cfg(test)]
mod foundation_import_compatibility_tests {
    use super::prepare_imported_registry;

    #[test]
    fn csv_import_is_migrated_before_the_current_schema_save_boundary() {
        let directory =
            std::env::temp_dir().join(format!("kept-cli-foundation-import-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        let csv = directory.join("keywords.csv");
        std::fs::write(&csv, "id,aliases\nreport,รายงาน|report\n")
            .expect("CSV fixture must be written");
        let legacy =
            kept_core::import_csv(&csv, "fixture", "Fixture").expect("CSV fixture must import");

        let current = prepare_imported_registry(legacy)
            .expect("imported CSV registry must migrate before save");

        assert_eq!(current.version, "1.2.0");
        assert!(current.foundation.is_some());
        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(test)]
mod search_policy_command_tests {
    use super::{handle_registry, prepare_imported_registry, RegistryCommands};
    use kept_core::schema::SearchPolicy;

    #[test]
    fn registry_search_rejects_invalid_user_search_policy_before_index_build() {
        let directory =
            std::env::temp_dir().join(format!("kept-cli-search-policy-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        let csv = directory.join("keywords.csv");
        let registry_path = directory.join("registry.json");
        std::fs::write(&csv, "id,aliases\nreport,report\n").expect("CSV fixture must be written");
        let legacy =
            kept_core::import_csv(&csv, "fixture", "Fixture").expect("CSV fixture must import");
        let mut registry =
            prepare_imported_registry(legacy).expect("imported CSV registry must migrate");
        registry.search_policy = Some(SearchPolicy {
            fuzzy_min_similarity: 1.01,
            ..SearchPolicy::default()
        });
        kept_core::save_registry(&registry_path, &registry)
            .expect("fixture registry must be written");

        let result = handle_registry(RegistryCommands::Search {
            path: registry_path,
            query: "rport".to_string(),
            group: None,
            json: true,
        });

        assert!(result.is_err());
        assert!(result
            .expect_err("invalid policy must fail")
            .to_string()
            .contains("INVALID_SEARCH_POLICY"));
        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(test)]
mod cli_command_topology_tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn cli_has_no_duplicate_short_options() {
        let result = std::panic::catch_unwind(|| Cli::command().debug_assert());
        assert!(
            result.is_ok(),
            "CLI command topology must have unique flags"
        );
    }
}
