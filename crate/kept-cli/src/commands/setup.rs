//! Setup command handler.

use dialoguer::Select;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupAction {
    EditConfig,
    Exit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetupResult {
    Created(PathBuf),
    AlreadyExists(PathBuf),
}

pub fn handle_setup() -> anyhow::Result<()> {
    let path = kept_core::default_user_config_path()?;
    match setup_user_config_at(&path)? {
        SetupResult::Created(path) => {
            println!("สร้าง starter config สำเร็จ: '{}'", path.display());
            println!("คำแนะนำ: รัน kept config เพื่อดูค่า หรือ kept doctor เพื่อตรวจสอบ");
        }
        SetupResult::AlreadyExists(path) => {
            println!("มี config อยู่แล้ว: '{}'", path.display());
            println!("คำแนะนำ: ใช้ kept config defaults/profile/scope เพื่อจัดการค่า");
        }
    }

    if crate::helpers::is_interactive_terminal() {
        let choices = ["1. เปิดแก้ไข config.yaml ทันที", "0. ข้ามขั้นตอนแก้ไข"];
        let selected = Select::new()
            .with_prompt("เลือกขั้นตอนถัดไป (ใช้ ↑/↓ หรือ j/k แล้วกด Enter)")
            .items(choices)
            .default(0)
            .interact_opt()?;
        if let Some(SetupAction::EditConfig) = selected.and_then(setup_action_from_selection) {
            super::config::handle_config(Some(super::config::ConfigCommands::Edit))?;
        }
    }

    Ok(())
}

pub fn setup_user_config_at(path: &Path) -> anyhow::Result<SetupResult> {
    if path.is_file() {
        return Ok(SetupResult::AlreadyExists(path.to_path_buf()));
    }
    kept_core::create_user_config_if_missing(path)?;
    Ok(SetupResult::Created(path.to_path_buf()))
}

pub fn setup_action_from_selection(position: usize) -> Option<SetupAction> {
    match position {
        0 => Some(SetupAction::EditConfig),
        _ => Some(SetupAction::Exit),
    }
}
