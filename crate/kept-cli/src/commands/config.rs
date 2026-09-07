//! Config command family for viewing and modifying YAML naming settings, profiles, and scopes.

use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::helpers::{
    parse_boolean, parse_map_field, parse_string_list, parse_usize, require_absolute_user_path,
    resolve_config_editor,
};

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show supported naming config fields, types, and allowed values.
    Fields,
    /// Inspect and modify baseline naming defaults.
    Defaults {
        #[command(subcommand)]
        cmd: DefaultCommands,
    },
    /// Manage naming profiles in config.yaml.
    Profile {
        #[command(subcommand)]
        cmd: ProfileCommands,
    },
    /// Manage absolute path scopes in config.yaml.
    Scope {
        #[command(subcommand)]
        cmd: ScopeCommands,
    },
    /// Open config.yaml in the configured editor.
    Edit,
}

#[derive(Subcommand)]
pub enum DefaultCommands {
    /// Show YAML representation of defaults.naming.
    Show,
    /// Set a default naming field value.
    Set { field: String, value: String },
    /// Unset a default naming field value.
    Unset { field: String },
}

#[derive(Subcommand)]
pub enum ProfileCommands {
    /// List all profiles with descriptions and managed extensions.
    List,
    /// Show YAML representation of a specific profile.
    Show { name: String },
    /// Add a new naming profile.
    Add {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove an unreferenced naming profile.
    Remove { name: String },
    /// Set a naming field value for a profile.
    Set {
        name: String,
        field: String,
        value: String,
    },
    /// Unset a naming field on a profile to inherit from defaults.
    Unset { name: String, field: String },
}

#[derive(Subcommand)]
pub enum ScopeCommands {
    /// List scopes ordered by precedence (deepest path, then highest priority).
    List,
    /// Show YAML representation of a specific scope.
    Show { path: PathBuf },
    /// Add a new path scope mapped to an existing profile.
    Add {
        profile: String,
        path: PathBuf,
        #[arg(long, default_value_t = 0)]
        priority: i32,
        #[arg(long, default_value_t = true, value_parser = clap::value_parser!(bool))]
        recursive: bool,
    },
    /// Update profile, priority, or recursive settings for a scope.
    Set {
        path: PathBuf,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        priority: Option<i32>,
        #[arg(long, value_parser = clap::value_parser!(bool))]
        recursive: Option<bool>,
    },
    /// Remove a path scope.
    Remove { path: PathBuf },
    /// Manage absolute exception paths for a scope.
    Exception {
        #[command(subcommand)]
        cmd: ScopeExceptionCommands,
    },
    /// Set or unset scope-specific naming overrides.
    Setting {
        #[command(subcommand)]
        cmd: ScopeSettingCommands,
    },
}

#[derive(Subcommand)]
pub enum ScopeSettingCommands {
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
pub enum ScopeExceptionCommands {
    Add { scope: PathBuf, path: PathBuf },
    Remove { scope: PathBuf, path: PathBuf },
}

pub const CONFIG_FIELD_CATALOG: &[(&str, &str, &str)] = &[
    ("unicode", "enum (nfc)", "Unicode normalization form"),
    ("case", "enum (lower, upper, preserve)", "target casing"),
    (
        "separator",
        "enum (kebab, snake, preserve)",
        "word separator",
    ),
    (
        "extensions",
        "comma-separated list",
        "managed file extensions",
    ),
    (
        "flagControlCharacters",
        "boolean",
        "flag ASCII/Unicode control characters",
    ),
    (
        "flagTrimWhitespace",
        "boolean",
        "flag leading/trailing whitespace",
    ),
    (
        "flagCaseCollisions",
        "boolean",
        "flag case-only duplicate collisions",
    ),
    (
        "flagPortabilityConflicts",
        "boolean",
        "flag Windows/POSIX unsafe filenames",
    ),
    ("numbers.allow", "boolean", "allow numerals in stem"),
    (
        "numbers.maxDigitsPerToken",
        "integer >= 0",
        "max digits in numeric tokens",
    ),
    ("words.min", "integer >= 0", "minimum word count"),
    ("words.max", "integer >= 0", "maximum word count"),
    (
        "length.stem.min",
        "integer >= 0",
        "minimum stem character count",
    ),
    (
        "length.stem.max",
        "integer >= 0",
        "maximum stem character count",
    ),
    (
        "whitespace.trim",
        "boolean",
        "trim leading/trailing whitespace",
    ),
    (
        "whitespace.collapseInternal",
        "boolean",
        "collapse multiple whitespace characters",
    ),
    ("prefix.required", "string", "required filename prefix"),
    (
        "prefix.allow",
        "comma-separated list",
        "allowed filename prefixes",
    ),
    (
        "similarity.name.threshold",
        "number 0.0..=1.0",
        "duplicate name threshold",
    ),
    (
        "similarity.name.caseSensitive",
        "boolean",
        "case sensitivity for name similarity",
    ),
    ("aliases.<name>", "string", "expand token alias value"),
    (
        "shortcuts.<token>",
        "string",
        "expand filename token shortcut",
    ),
    ("variables.<name>", "string", "resolve template variable"),
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

pub fn config_field_default_value(field: &str, naming: &kept_core::NamingSettings) -> String {
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
        "replacements" => serde_yaml::to_string(&naming.replacements)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        "reposition" => serde_yaml::to_string(&naming.reposition)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        "stemRegex" => naming.stem_regex.clone().unwrap_or_default(),
        _ if field.starts_with("aliases.") => {
            let key = field.trim_start_matches("aliases.");
            naming.aliases.get(key).cloned().unwrap_or_default()
        }
        _ if field.starts_with("shortcuts.") => {
            let key = field.trim_start_matches("shortcuts.");
            naming.shortcuts.get(key).cloned().unwrap_or_default()
        }
        _ if field.starts_with("variables.") => {
            let key = field.trim_start_matches("variables.");
            naming.variables.get(key).cloned().unwrap_or_default()
        }
        _ => String::new(),
    }
}

pub fn print_config_field_catalog(naming: &kept_core::NamingSettings) {
    println!("Field\tType\tDefault value\tDescription");
    for (field, field_type, description) in CONFIG_FIELD_CATALOG {
        let default_value = config_field_default_value(field, naming);
        println!("{field}\t{field_type}\t{default_value}\t{description}");
    }
}

pub fn handle_config(cmd: Option<ConfigCommands>) -> anyhow::Result<()> {
    let path = kept_core::default_user_config_path()?;
    match cmd {
        None => {
            let config = load_config_for_mutation(&path)?;
            println!("Config: '{}'", path.display());
            println!("Version: {}", config.version);
            println!(
                "Defaults: unicode={}, case={}, separator={}, extensions={}",
                config.defaults.naming.unicode.as_deref().unwrap_or("-"),
                config.defaults.naming.case.as_deref().unwrap_or("-"),
                config.defaults.naming.separator.as_deref().unwrap_or("-"),
                config.defaults.naming.extensions.join(",")
            );
            println!("Profiles: {}", config.profiles.len());
            for (name, profile) in &config.profiles {
                let description = profile.description.as_deref().unwrap_or("");
                println!(
                    "  - {name}: {description} (extensions: {})",
                    profile.naming.extensions.join(",")
                );
            }
            println!("Scopes: {}", config.scopes.len());
            for scope in &config.scopes {
                println!(
                    "  - {} -> {} (priority: {}, recursive: {})",
                    scope.path.display(),
                    scope.profile,
                    scope.priority,
                    scope.recursive
                );
            }
            println!("\nNext commands:");
            println!("  kept config defaults show / set <field> <value>");
            println!("  kept config profile list / add <name> / set <name> <field> <value>");
            println!("  kept config scope list / add <profile> <absolute-path>");
            println!("  kept config edit");
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
            let editor = resolve_config_editor(None);
            if !path.is_file() {
                anyhow::bail!(
                    "ไม่พบ config ที่ '{}'; รัน kept setup ก่อนเปิดแก้ไข",
                    path.display()
                );
            }
            let status = Command::new(&editor).arg(&path).status().map_err(|error| {
                anyhow::anyhow!(
                    "เปิด editor '{}' ไม่ได้: {error}; ติดตั้ง editor หรือตั้ง KEPT_EDITOR เช่น export KEPT_EDITOR=nano",
                    Path::new(&editor).display()
                )
            })?;
            if !status.success() {
                anyhow::bail!(
                    "editor '{}' ปิดด้วย error code สำหรับ {}",
                    Path::new(&editor).display(),
                    path.display()
                );
            }
            Ok(())
        }
    }
}

pub fn load_config_for_mutation(path: &Path) -> anyhow::Result<kept_core::UserConfig> {
    kept_core::load_user_config(path).map_err(|error| {
        anyhow::anyhow!(
            "เปิด config ไม่ได้ที่ '{}': {error}; รัน kept setup เพื่อสร้างหรือ kept doctor --fix เพื่อกู้ไฟล์ที่เสีย",
            path.display()
        )
    })
}

pub fn handle_config_defaults(path: &Path, cmd: DefaultCommands) -> anyhow::Result<()> {
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

pub fn handle_config_profile(path: &Path, cmd: ProfileCommands) -> anyhow::Result<()> {
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
                anyhow::bail!("profile '{name}' มีอยู่แล้ว");
            }
            config.profiles.insert(
                name.clone(),
                kept_core::NamingProfile {
                    description,
                    naming: kept_core::NamingSettings::default(),
                },
            );
            kept_core::save_user_config(path, &config)?;
            println!("Added profile '{name}'");
            Ok(())
        }
        ProfileCommands::Remove { name } => {
            let has_scope = config.scopes.iter().any(|scope| scope.profile == name);
            if has_scope {
                anyhow::bail!("ลบ profile '{name}' ไม่ได้เพราะมี scope ใช้งานอยู่");
            }
            if config.profiles.remove(&name).is_none() {
                anyhow::bail!("ไม่พบ profile '{name}'");
            }
            kept_core::save_user_config(path, &config)?;
            println!("Removed profile '{name}'");
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

pub fn find_scope_index(config: &kept_core::UserConfig, path: &Path) -> anyhow::Result<usize> {
    config
        .scopes
        .iter()
        .position(|scope| scope.path == path)
        .ok_or_else(|| anyhow::anyhow!("ไม่พบ scope '{}'", path.display()))
}

pub fn handle_config_scope(path: &Path, cmd: ScopeCommands) -> anyhow::Result<()> {
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
                anyhow::bail!("ไม่พบ profile '{profile}' ใน config");
            }
            if config.scopes.iter().any(|scope| scope.path == scope_path) {
                anyhow::bail!("มี scope path '{}' อยู่แล้ว", scope_path.display());
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
            let index = find_scope_index(&config, &scope_path)?;
            let scope = &mut config.scopes[index];
            if let Some(profile) = profile {
                if !config.profiles.contains_key(&profile) {
                    anyhow::bail!("ไม่พบ profile '{profile}' ใน config");
                }
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

pub fn set_profile_field(
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
                anyhow::anyhow!(
                    "replacements YAML ไม่ถูกต้อง: {error}; ใช้รายการ {{from: ..., to: ...}}"
                )
            })?
        }
        "reposition" => {
            naming.reposition = serde_yaml::from_str(value).map_err(|error| {
                anyhow::anyhow!(
                "reposition YAML ไม่ถูกต้อง: {error}; ใช้รายการ {{token: ..., position: front/back}}"
            )
            })?
        }
        "stemRegex" => {
            naming.stem_regex = Some(value.to_string());
        }
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

pub fn unset_profile_field(
    naming: &mut kept_core::NamingSettings,
    field: &str,
) -> anyhow::Result<()> {
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
            if let Some(range) = naming.length.stem.as_mut() {
                range.min = None;
            }
        }
        "length.stem.max" => {
            if let Some(range) = naming.length.stem.as_mut() {
                range.max = None;
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
