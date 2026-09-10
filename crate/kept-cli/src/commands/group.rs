//! Keyword registry group management command handler.

use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum GroupCommands {
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
pub enum GroupFieldCommands {
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

pub const REGISTRY_GROUP_FIELD_TYPES: &[(&str, &str)] = &[
    ("string", "ข้อความหนึ่งค่า"),
    ("enum", "ข้อความที่จำกัด values"),
    ("array", "รายการค่า; itemType เป็น string, number หรือ boolean"),
    ("object", "object JSON/YAML"),
    ("number", "ตัวเลข"),
    ("boolean", "true หรือ false"),
];

pub fn load_registry_for_group(path: &Path) -> anyhow::Result<kept_core::schema::KeywordRegistry> {
    kept_core::load_registry(path).map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub fn save_registry_for_group(
    path: &Path,
    registry: &kept_core::schema::KeywordRegistry,
) -> anyhow::Result<()> {
    kept_core::save_registry(path, registry).map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub fn group_field_schema(
    field_type: &str,
    item_type: Option<String>,
    values: Option<String>,
    required: bool,
    description: String,
) -> anyhow::Result<kept_core::schema::FieldSchema> {
    let supported = REGISTRY_GROUP_FIELD_TYPES
        .iter()
        .any(|(kind, _)| *kind == field_type);
    if !supported {
        anyhow::bail!(
            "ชนิด field ไม่ถูกต้อง '{field_type}'; ใช้ string, enum, array, object, number หรือ boolean"
        );
    }
    if field_type == "enum" && values.as_deref().unwrap_or("").trim().is_empty() {
        anyhow::bail!("ชนิด enum ต้องระบุ --values เช่น active,inactive,draft");
    }
    if field_type == "array" && item_type.as_deref().unwrap_or("").trim().is_empty() {
        anyhow::bail!("ชนิด array ต้องระบุ --item-type เช่น string, number หรือ boolean");
    }
    let parsed_values = values.map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    Ok(kept_core::schema::FieldSchema {
        field_type: field_type.to_string(),
        item_type,
        values: parsed_values,
        required: Some(required),
        description,
        ..Default::default()
    })
}

pub fn default_registry_group(
    group_id: String,
    name: String,
    description: String,
) -> kept_core::schema::KeywordGroup {
    let mut base_fields = std::collections::HashMap::new();
    base_fields.insert(
        "id".to_string(),
        kept_core::schema::FieldSchema {
            field_type: "string".to_string(),
            required: Some(true),
            description: "Unique identifier".to_string(),
            ..Default::default()
        },
    );
    base_fields.insert(
        "aliases".to_string(),
        kept_core::schema::FieldSchema {
            field_type: "array".to_string(),
            item_type: Some("string".to_string()),
            required: Some(true),
            description: "Search aliases".to_string(),
            ..Default::default()
        },
    );
    kept_core::schema::KeywordGroup {
        group_id,
        group_name: name,
        description,
        base_fields_schema: base_fields,
        custom_field_allowed: kept_core::schema::CustomFieldConfig {
            enabled: true,
            ..Default::default()
        },
        entries: Vec::new(),
        group_stats: None,
    }
}

pub fn handle_group(cmd: GroupCommands) -> anyhow::Result<()> {
    match cmd {
        GroupCommands::Types => {
            println!("Type\tDescription");
            for (kind, description) in REGISTRY_GROUP_FIELD_TYPES {
                println!("{kind}\t{description}");
            }
            Ok(())
        },
        GroupCommands::List { registry: path } => {
            let registry = load_registry_for_group(&path)?;
            println!("Index\tGroup ID\tName\tEntries\tFields\tDescription");
            for (position, group) in registry.groups.iter().enumerate() {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    position + 1,
                    group.group_id,
                    group.group_name,
                    group.entries.len(),
                    group.base_fields_schema.len(),
                    group.description
                );
            }
            Ok(())
        },
        GroupCommands::Show { registry: path, id: group_id } => {
            let registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter()
                .find(|item| item.group_id == group_id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{group_id}'"))?;
            println!("{}", serde_yaml::to_string(group)?);
            Ok(())
        },
        GroupCommands::Add {
            registry: path,
            id: group_id,
            name,
            description,
        } => {
            let mut registry = load_registry_for_group(&path)?;
            if registry
                .groups
                .iter()
                .any(|group| group.group_id == group_id)
            {
                anyhow::bail!("group '{group_id}' มีอยู่แล้ว");
            }
            registry
                .groups
                .push(default_registry_group(group_id.clone(), name, description));
            save_registry_for_group(&path, &registry)?;
            println!("Added group '{group_id}'");
            Ok(())
        },
        GroupCommands::Set {
            registry: path,
            id: group_id,
            name,
            description,
        } => {
            let mut registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter_mut()
                .find(|group| group.group_id == group_id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{group_id}'"))?;
            if let Some(name) = name {
                group.group_name = name;
            }
            if let Some(description) = description {
                group.description = description;
            }
            save_registry_for_group(&path, &registry)?;
            println!("Updated group '{group_id}'");
            Ok(())
        },
        GroupCommands::Remove { registry: path, id: group_id } => {
            let mut registry = load_registry_for_group(&path)?;
            let position = registry
                .groups
                .iter()
                .position(|group| group.group_id == group_id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{group_id}'"))?;
            registry.groups.remove(position);
            save_registry_for_group(&path, &registry)?;
            println!("Removed group '{group_id}'");
            Ok(())
        },
        GroupCommands::Move {
            registry: path,
            id: group_id,
            before,
            after,
        } => {
            let mut registry = load_registry_for_group(&path)?;
            let source_index = registry
                .groups
                .iter()
                .position(|group| group.group_id == group_id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{group_id}'"))?;
            let group = registry.groups.remove(source_index);
            let target_index = match (before, after) {
                (Some(target), None) => registry
                    .groups
                    .iter()
                    .position(|item| item.group_id == target)
                    .ok_or_else(|| anyhow::anyhow!("ไม่พบ target group '{target}'"))?,
                (None, Some(target)) => registry
                    .groups
                    .iter()
                    .position(|item| item.group_id == target)
                    .map(|index| index + 1)
                    .ok_or_else(|| anyhow::anyhow!("ไม่พบ target group '{target}'"))?,
                _ => anyhow::bail!("ต้องระบุ --before หรือ --after อย่างใดอย่างหนึ่ง"),
            };
            registry.groups.insert(target_index, group);
            save_registry_for_group(&path, &registry)?;
            println!("Moved group '{group_id}'");
            Ok(())
        },
        GroupCommands::Field { cmd } => handle_group_field(cmd),
    }
}

pub fn handle_group_field(cmd: GroupFieldCommands) -> anyhow::Result<()> {
    match cmd {
        GroupFieldCommands::List { registry: path, group } => {
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
        },
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
        },
        GroupFieldCommands::Remove {
            registry: path,
            group: group_id,
            name,
        } => {
            if name == "id" || name == "aliases" {
                anyhow::bail!("ลบ base field '{name}' ไม่ได้");
            }
            let mut registry = load_registry_for_group(&path)?;
            let group = registry
                .groups
                .iter_mut()
                .find(|item| item.group_id == group_id)
                .ok_or_else(|| anyhow::anyhow!("ไม่พบ group '{group_id}'"))?;
            if group.base_fields_schema.remove(&name).is_none() {
                anyhow::bail!("ไม่พบ field '{name}' ใน group '{group_id}'");
            }
            save_registry_for_group(&path, &registry)?;
            println!("Removed field '{name}' from group '{group_id}'");
            Ok(())
        },
    }
}
