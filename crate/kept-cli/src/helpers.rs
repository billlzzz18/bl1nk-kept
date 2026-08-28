//! Shared CLI helpers for terminal interaction, size parsing, and path resolution.

use dialoguer::Input;
use std::ffi::{OsStr, OsString};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

pub fn is_interactive_terminal() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn confirm_action(description: &str) -> anyhow::Result<bool> {
    print!("{description} ใช่หรือไม่? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

pub fn prompt_optional(prompt: &str) -> anyhow::Result<Option<String>> {
    let value: String = Input::new()
        .with_prompt(prompt)
        .allow_empty(true)
        .interact_text()?;
    Ok((!value.trim().is_empty()).then_some(value))
}

pub fn require_absolute_user_path(path: &Path, label: &str) -> anyhow::Result<()> {
    if path.is_absolute() {
        Ok(())
    } else {
        anyhow::bail!("{label} ต้องเป็น absolute path ที่ผู้ใช้ระบุ: {}", path.display())
    }
}

pub fn default_scan_snapshot_path(root: &str) -> PathBuf {
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

pub fn resolve_config_editor(explicit_editor: Option<OsString>) -> OsString {
    explicit_editor
        .or_else(|| std::env::var_os("KEPT_EDITOR"))
        .unwrap_or_else(|| OsString::from("nano"))
}

pub fn editor_is_available(editor: &OsStr) -> bool {
    let editor_path = Path::new(editor);
    if editor_path.is_absolute() || editor_path.components().count() > 1 {
        return editor_path.is_file();
    }
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| directory.join(editor_path).is_file())
    })
}

pub fn parse_human_size(value: &str) -> anyhow::Result<u64> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("ขนาดไฟล์ต้องไม่ว่าง");
    }
    let lower = value.to_lowercase();
    if let Some(number) = lower.strip_suffix("gb").or_else(|| lower.strip_suffix('g')) {
        return parse_size_multiplier(number, 1024 * 1024 * 1024);
    }
    if let Some(number) = lower.strip_suffix("mb").or_else(|| lower.strip_suffix('m')) {
        return parse_size_multiplier(number, 1024 * 1024);
    }
    if let Some(number) = lower.strip_suffix("kb").or_else(|| lower.strip_suffix('k')) {
        return parse_size_multiplier(number, 1024);
    }
    if let Some(number) = lower.strip_suffix('b') {
        return parse_size_multiplier(number, 1);
    }
    value
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("ขนาดไฟล์ไม่ถูกต้อง '{value}'; ใช้เช่น 10mb, 512kb หรือจำนวน bytes"))
}

fn parse_size_multiplier(value: &str, multiplier: u64) -> anyhow::Result<u64> {
    let base = value
        .trim()
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("ตัวเลขขนาดไฟล์ไม่ถูกต้อง '{value}'"))?;
    Ok(base.saturating_mul(multiplier))
}

pub fn parse_boolean(value: &str, field: &str) -> anyhow::Result<bool> {
    value
        .parse::<bool>()
        .map_err(|_| anyhow::anyhow!("{field} ต้องเป็น true หรือ false"))
}

pub fn parse_usize(value: &str, field: &str) -> anyhow::Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("{field} ต้องเป็น integer ตั้งแต่ 0"))
}

pub fn parse_string_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn parse_map_field<'a>(field: &'a str, prefix: &str) -> anyhow::Result<&'a str> {
    field
        .strip_prefix(prefix)
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("field ต้องอยู่ในรูปแบบ {prefix}<name>"))
}
