use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod commands;
pub mod corpus;
pub mod evidence;
pub mod helpers;

use commands::*;
use corpus::{handle_corpus, CorpusCommands};
use evidence::{handle_evidence, EvidenceCommands};

#[derive(Parser)]
#[command(
    name = "kept",
    version = env!("CARGO_PKG_VERSION"),
    about = "Unified CLI for Keywords, Filesystem Analysis, and Document Sync"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
    /// Find files from the latest scan index using simple filter facts or query expression.
    Find {
        /// Root directory (defaults to current directory if omitted)
        #[arg(default_value = ".")]
        root: PathBuf,
        /// Query string in Filesystem Query Language (e.g. "ext:pdf size>50MB dup:content")
        #[arg(short = 'q', long)]
        query: Option<String>,
        /// Explain query plan without executing
        #[arg(long)]
        explain: bool,
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
        /// Rerank BM25 results using a local semantic reranker.
        #[arg(long)]
        semantic: bool,
        /// Combine BM25 and semantic reranker scores (weighted blend).
        #[arg(long, conflicts_with = "semantic")]
        hybrid: bool,
    },
    /// Inspect and manage keyword registry groups.
    Group {
        #[command(subcommand)]
        cmd: GroupCommands,
    },
    /// Convert a document between supported offline formats.
    Convert { input: PathBuf, output: PathBuf },
    /// Create user config.yaml if missing and launch interactive setup workflow.
    Setup,
    /// View or modify user config.yaml and naming rules.
    Config {
        #[command(subcommand)]
        cmd: Option<ConfigCommands>,
    },
    /// Inspect configuration and runtime environment health.
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
            query,
            explain,
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
            query,
            explain,
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
            semantic,
            hybrid,
        } => handle_registry(RegistryCommands::Search {
            path: registry,
            query,
            group,
            json,
            semantic,
            hybrid,
        })
        .await?,
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
        Commands::Registry { cmd } => handle_registry(cmd).await?,
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

#[cfg(test)]
mod tests {
    use super::commands::config::*;
    use super::commands::doctor::*;
    use super::commands::duplicates::*;
    use super::commands::fs::*;
    use super::commands::scan::*;
    use super::commands::setup::*;
    use super::helpers::*;
    use super::Cli;
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
            query: None,
            explain: false,
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

        assert!(result.is_ok(), "find must query the snapshot");
        if let Some(bytes) = previous {
            std::fs::write(&snapshot_path, bytes).ok();
        } else {
            std::fs::remove_file(&snapshot_path).ok();
        }
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
    }

    #[test]
    fn task_first_find_with_fql_query_and_explain() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root = std::env::temp_dir().join(format!("kept-task-fql-{}", std::process::id()));
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

        let explain_result = handle_task_find(FindRequest {
            root: root.clone(),
            query: Some("ext:pdf size>1B report".to_string()),
            explain: true,
            file_type: None,
            name: None,
            path_contains: None,
            min_size: None,
            max_size: None,
            after: None,
            before: None,
            index: None,
            json: true,
        });
        assert!(explain_result.is_ok(), "explain query plan must succeed");

        let find_result = handle_task_find(FindRequest {
            root: root.clone(),
            query: Some("ext:pdf size>1B report".to_string()),
            explain: false,
            file_type: None,
            name: None,
            path_contains: None,
            min_size: None,
            max_size: None,
            after: None,
            before: None,
            index: None,
            json: true,
        });
        assert!(find_result.is_ok(), "query execution must succeed");

        if let Some(bytes) = previous {
            std::fs::write(&snapshot_path, bytes).ok();
        } else {
            std::fs::remove_file(&snapshot_path).ok();
        }
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
    }

    #[test]
    fn task_first_find_uses_user_selected_portable_index() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root = std::env::temp_dir().join(format!(
            "kept-task-find-portable-root-{}",
            std::process::id()
        ));
        let index_dir = std::env::temp_dir().join(format!(
            "kept-task-find-portable-index-{}",
            std::process::id()
        ));
        let snapshot_path = index_dir.join("portable.json");
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        std::fs::create_dir_all(&index_dir).expect("fixture index dir must be created");
        std::fs::write(root.join("report.pdf"), "fixture").expect("fixture file must be written");
        let options = ScanOptions::default();
        let index = scan_directory(&root, &options).expect("fixture must scan");
        std::fs::write(
            &snapshot_path,
            serde_json::to_string(&create_persistent_snapshot(index, &options))
                .expect("snapshot must serialize"),
        )
        .expect("snapshot must be written");

        let result = handle_task_find(FindRequest {
            root: root.clone(),
            query: None,
            explain: false,
            file_type: Some("pdf".to_string()),
            name: Some("report".to_string()),
            path_contains: None,
            min_size: None,
            max_size: None,
            after: None,
            before: None,
            index: Some(snapshot_path.clone()),
            json: true,
        });

        assert!(
            result.is_ok(),
            "find must query the user-selected portable index"
        );
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
        std::fs::remove_dir_all(index_dir).expect("fixture index dir must be removed");
    }

    #[test]
    fn task_first_review_reads_the_existing_scan_index() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root = std::env::temp_dir().join(format!("kept-task-review-{}", std::process::id()));
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

        let result = handle_task_review(root.clone(), None, false);

        assert!(
            result.is_ok(),
            "review must read the persistent index non-interactively"
        );
        if let Some(bytes) = previous {
            std::fs::write(&snapshot_path, bytes).ok();
        } else {
            std::fs::remove_file(&snapshot_path).ok();
        }
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
    }

    #[test]
    fn review_renders_read_only_naming_findings_from_existing_config_and_index() {
        let root = std::env::temp_dir().join(format!(
            "kept-task-review-naming-root-{}",
            std::process::id()
        ));
        let config_path = std::env::temp_dir().join(format!(
            "kept-review-naming-config-{}.yaml",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        let sample = root.join("My Documents Report.TXT");
        std::fs::write(&sample, "content").expect("fixture file must be written");

        let mut config = kept_core::UserConfig::starter();
        config.scopes.push(kept_core::NamingScope {
            path: root.clone(),
            profile: "generic".to_string(),
            recursive: true,
            priority: 0,
            exceptions: Vec::new(),
            overrides: kept_core::policy::ScopeOverrides {
                naming: kept_core::NamingSettings {
                    case: Some("lower".to_string()),
                    separator: Some("kebab".to_string()),
                    whitespace: kept_core::policy::WhitespaceSettings {
                        collapse_internal: Some(true),
                        trim: Some(true),
                    },
                    ..Default::default()
                },
            },
        });
        std::fs::write(
            &config_path,
            serde_yaml::to_string(&config).expect("config must serialize"),
        )
        .expect("config must be written");

        let index = kept_core::scan_directory(&root, &kept_core::ScanOptions::default())
            .expect("root must scan");
        let summary = naming_review_summary_at(&index, &config_path);

        assert!(summary.contains("Naming policy: 1 finding(s)"));
        assert!(summary.contains("My Documents Report.TXT"));
        assert!(summary.contains("proposed: my-documents-report.txt"));
        assert!(sample.is_file(), "review must never rename scanned files");

        std::fs::remove_file(&config_path).expect("config must be removed");
        std::fs::remove_dir_all(&root).expect("root must be removed");
    }

    #[test]
    fn task_first_duplicates_reads_the_existing_scan_index() {
        use kept_core::{create_persistent_snapshot, scan_directory, ScanOptions};

        let root =
            std::env::temp_dir().join(format!("kept-task-duplicates-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture root must be created");
        std::fs::write(root.join("a.txt"), "duplicate payload")
            .expect("first fixture file must be written");
        std::fs::write(root.join("b.txt"), "duplicate payload")
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

        assert!(
            result.is_ok(),
            "duplicates must read the persistent index and verify content"
        );
        if let Some(bytes) = previous {
            std::fs::write(&snapshot_path, bytes).ok();
        } else {
            std::fs::remove_file(&snapshot_path).ok();
        }
        std::fs::remove_dir_all(root).expect("fixture root must be removed");
    }

    #[test]
    fn parses_custom_filter() {
        let filter = parse_custom_filter("name:contains:report").unwrap();
        assert_eq!(filter.field, "name");
        assert!(matches!(filter.operator, CustomOperator::Contains));
        assert_eq!(filter.value, "report");

        let filter = parse_custom_filter("size:ge:1048576").unwrap();
        assert_eq!(filter.field, "size");
        assert!(matches!(filter.operator, CustomOperator::GreaterOrEqual));
        assert_eq!(filter.value, "1048576");
    }

    #[test]
    fn parses_task_first_scan_with_optional_index_output() {
        let command = Cli::try_parse_from([
            "kept",
            "scan",
            "/tmp/kept-fixtures",
            "--output",
            "/tmp/kept-portable-scan.json",
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
            "--name",
            "invoice",
            "--min-size",
            "10mb",
            "--json",
        ]);
        assert!(command.is_ok(), "task-first find command must parse");
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
            "duplicate scan with explicit yes must parse"
        );
    }

    #[test]
    fn scan_space_review_sorts_largest_top_level_nodes() {
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: 40,
            files: vec![
                FileRecord {
                    path: "small/a.txt".into(),
                    name: "a.txt".into(),
                    extension: "txt".into(),
                    size: 10,
                    modified_unix: 0,
                    kind: "file".into(),
                    is_binary: None,
                    git_status: None,
                },
                FileRecord {
                    path: "large/b.txt".into(),
                    name: "b.txt".into(),
                    extension: "txt".into(),
                    size: 30,
                    modified_unix: 0,
                    kind: "file".into(),
                    is_binary: None,
                    git_status: None,
                },
            ],
            issues: Vec::new(),
        };

        let summary = build_scan_space_summary(&index);
        assert_eq!(summary[0], ("large".to_string(), 30));
        assert_eq!(summary[1], ("small".to_string(), 10));
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
                is_binary: None,
                git_status: None,
            }],
            issues: vec![ScanIssue::new(
                "locked".to_string(),
                "read_dir".to_string(),
                "permission denied".to_string(),
            )],
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
            Some(ScanReviewAction::Integrity)
        );
        assert_eq!(
            scan_review_action_from_selection("5"),
            Some(ScanReviewAction::Issues)
        );
        assert_eq!(
            scan_review_action_from_selection("6"),
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
        assert_eq!(duplicate_action_from_selection("0"), None);
    }

    #[test]
    fn parses_interactive_setup_command() {
        let command = Cli::try_parse_from(["kept", "setup"]);
        assert!(command.is_ok(), "setup command must parse");
    }

    #[test]
    fn setup_creates_starter_config_once_and_preserves_user_owned_content() {
        let directory =
            std::env::temp_dir().join(format!("kept-setup-test-{}", std::process::id()));
        let config_path = directory.join("config.yaml");
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");

        let first = setup_user_config_at(&config_path).expect("initial setup must succeed");
        assert_eq!(first, SetupResult::Created(config_path.clone()));
        assert!(config_path.is_file());

        let initial_content =
            std::fs::read_to_string(&config_path).expect("config must be readable");
        assert!(initial_content.contains("version: 1"));

        std::fs::write(&config_path, "custom: value\n").expect("user modification must succeed");
        let second = setup_user_config_at(&config_path).expect("subsequent setup must succeed");
        assert_eq!(second, SetupResult::AlreadyExists(config_path.clone()));
        assert_eq!(
            std::fs::read_to_string(&config_path).expect("config must stay modified"),
            "custom: value\n"
        );

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn parses_config_edit_command() {
        let command = Cli::try_parse_from(["kept", "config", "edit"]);
        assert!(command.is_ok(), "config edit command must parse");
    }

    #[test]
    fn parses_config_fields_command() {
        let command = Cli::try_parse_from(["kept", "config", "fields"]);
        assert!(command.is_ok(), "config fields command must parse");
    }

    #[test]
    fn documented_naming_fields_can_be_set_and_unset_without_yaml_editing() {
        let mut naming = kept_core::NamingSettings::default();

        set_profile_field(&mut naming, "unicode", "nfc").expect("unicode must set");
        set_profile_field(&mut naming, "case", "lower").expect("case must set");
        set_profile_field(&mut naming, "separator", "kebab").expect("separator must set");
        set_profile_field(&mut naming, "extensions", "md,txt").expect("extensions must set");
        set_profile_field(&mut naming, "flagControlCharacters", "true")
            .expect("flagControlCharacters must set");
        set_profile_field(&mut naming, "numbers.allow", "false").expect("numbers.allow must set");
        set_profile_field(&mut naming, "numbers.maxDigitsPerToken", "4")
            .expect("maxDigitsPerToken must set");
        set_profile_field(&mut naming, "words.min", "2").expect("words.min must set");
        set_profile_field(&mut naming, "length.stem.max", "64").expect("length.stem.max must set");
        set_profile_field(&mut naming, "whitespace.trim", "true")
            .expect("whitespace.trim must set");
        set_profile_field(&mut naming, "prefix.required", "DOC-")
            .expect("prefix.required must set");
        set_profile_field(&mut naming, "prefix.allow", "DOC-,NOTE-")
            .expect("prefix.allow must set");
        set_profile_field(&mut naming, "similarity.name.threshold", "0.95")
            .expect("similarity threshold must set");
        set_profile_field(&mut naming, "similarity.name.caseSensitive", "true")
            .expect("similarity caseSensitive must set");
        set_profile_field(&mut naming, "stemRegex", r"^draft-.*").expect("stemRegex must set");
        set_profile_field(&mut naming, "aliases.doc", "document").expect("alias must set");
        set_profile_field(&mut naming, "shortcuts.wip", "work-in-progress")
            .expect("shortcut must set");
        set_profile_field(&mut naming, "variables.year", "2026").expect("variable must set");
        set_profile_field(
            &mut naming,
            "replacements",
            "- from: old\n  to: new\n  caseSensitive: false",
        )
        .expect("replacements must set");
        set_profile_field(&mut naming, "reposition", "- token: tag\n  position: front")
            .expect("reposition must set");

        assert_eq!(config_field_default_value("unicode", &naming), "nfc");
        assert_eq!(config_field_default_value("case", &naming), "lower");
        assert_eq!(config_field_default_value("separator", &naming), "kebab");
        assert_eq!(config_field_default_value("extensions", &naming), "md,txt");
        assert_eq!(
            config_field_default_value("flagControlCharacters", &naming),
            "true"
        );
        assert_eq!(
            config_field_default_value("numbers.allow", &naming),
            "false"
        );
        assert_eq!(
            config_field_default_value("numbers.maxDigitsPerToken", &naming),
            "4"
        );
        assert_eq!(config_field_default_value("words.min", &naming), "2");
        assert_eq!(config_field_default_value("length.stem.max", &naming), "64");
        assert_eq!(
            config_field_default_value("whitespace.trim", &naming),
            "true"
        );
        assert_eq!(
            config_field_default_value("prefix.required", &naming),
            "DOC-"
        );
        assert_eq!(
            config_field_default_value("prefix.allow", &naming),
            "DOC-,NOTE-"
        );
        assert_eq!(
            config_field_default_value("similarity.name.threshold", &naming),
            "0.95"
        );
        assert_eq!(
            config_field_default_value("similarity.name.caseSensitive", &naming),
            "true"
        );
        assert_eq!(
            config_field_default_value("stemRegex", &naming),
            r"^draft-.*"
        );
        assert_eq!(
            config_field_default_value("aliases.doc", &naming),
            "document"
        );
        assert_eq!(
            config_field_default_value("shortcuts.wip", &naming),
            "work-in-progress"
        );
        assert_eq!(
            config_field_default_value("variables.year", &naming),
            "2026"
        );

        unset_profile_field(&mut naming, "unicode").expect("unicode must unset");
        unset_profile_field(&mut naming, "numbers.maxDigitsPerToken")
            .expect("maxDigitsPerToken must unset");
        unset_profile_field(&mut naming, "aliases.doc").expect("alias must unset");

        assert_eq!(config_field_default_value("unicode", &naming), "");
        assert_eq!(
            config_field_default_value("numbers.maxDigitsPerToken", &naming),
            ""
        );
        assert_eq!(config_field_default_value("aliases.doc", &naming), "");
    }

    #[test]
    fn maps_interactive_setup_choices_without_implicit_config_mutation() {
        assert_eq!(
            setup_action_from_selection(0),
            Some(SetupAction::EditConfig)
        );
        assert_eq!(setup_action_from_selection(1), Some(SetupAction::Exit));
    }

    #[test]
    fn config_editor_defaults_to_nano_and_accepts_explicit_editor_path() {
        let editor = resolve_config_editor(None);
        assert!(!editor.as_os_str().is_empty());
        if let Some(configured) = std::env::var_os("KEPT_EDITOR") {
            assert_eq!(editor, configured);
        }
        assert_eq!(
            resolve_config_editor(Some("/usr/bin/custom-editor".into())),
            std::ffi::OsString::from("/usr/bin/custom-editor")
        );
    }

    #[test]
    fn doctor_run_includes_the_config_editor_diagnostic() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-editor-test-{}", std::process::id()));
        let config_path = directory.join("config.yaml");
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        kept_core::create_user_config_if_missing(&config_path)
            .expect("starter config must be created");

        let result = run_doctor_at(
            &config_path,
            false,
            Some("/path/to/definitely/missing/editor".into()),
        )
        .expect("doctor run must complete");

        assert!(result
            .report
            .findings
            .iter()
            .any(|finding| finding.code == "CONFIG_VALID"));
        assert!(result
            .report
            .findings
            .iter()
            .any(|finding| finding.code == "CONFIG_EDITOR_MISSING"));
        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn doctor_reports_a_missing_editor_with_a_concrete_recovery_path() {
        let report = doctor_editor_at("/path/to/definitely/missing/editor".into());
        let finding = &report.findings[0];

        assert_eq!(finding.code, "CONFIG_EDITOR_MISSING");
        assert_eq!(finding.status, DoctorStatus::Error);
        assert!(finding.message.contains("missing/editor"));
        assert!(finding.remediation.contains("KEPT_EDITOR"));
    }

    #[test]
    fn doctor_reports_a_missing_config_with_its_path_and_recovery_command() {
        let path = std::path::PathBuf::from("/definitely/missing/config.yaml");
        let report = doctor_config_at(&path);
        let finding = &report.findings[0];

        assert_eq!(finding.code, "CONFIG_MISSING");
        assert_eq!(finding.status, DoctorStatus::Error);
        assert!(finding.message.contains(&path.display().to_string()));
        assert!(finding.remediation.contains("kept setup"));
    }

    #[test]
    fn doctor_fix_creates_a_missing_config_then_reports_it_valid() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-fix-test-{}", std::process::id()));
        let config_path = directory.join("config.yaml");
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");

        let initial = doctor_config_at(&config_path);
        assert_eq!(initial.findings[0].code, "CONFIG_MISSING");

        let repair = repair_missing_config_at(&config_path).expect("missing config must be fixed");
        assert_eq!(repair, SetupResult::Created(config_path.clone()));

        let recovered = doctor_config_at(&config_path);
        assert_eq!(recovered.findings[0].code, "CONFIG_VALID");
        assert!(config_path.is_file());

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn doctor_fix_backs_up_an_invalid_config_before_restoring_a_valid_starter() {
        let directory = std::env::temp_dir().join(format!(
            "kept-doctor-backup-invalid-test-{}",
            std::process::id()
        ));
        let config_path = directory.join("config.yaml");
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        std::fs::write(&config_path, "invalid: yaml: content: [unclosed\n")
            .expect("invalid config fixture must be written");

        let initial = doctor_config_at(&config_path);
        assert_eq!(initial.findings[0].code, "CONFIG_INVALID");

        let repair =
            repair_invalid_config_at(&config_path).expect("invalid config must be repaired");
        assert!(repair.backup_path.is_file());
        assert_eq!(
            std::fs::read_to_string(&repair.backup_path).expect("backup must be readable"),
            "invalid: yaml: content: [unclosed\n"
        );

        let recovered = doctor_config_at(&config_path);
        assert_eq!(recovered.findings[0].code, "CONFIG_VALID");
        assert!(config_path.is_file());

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }

    #[test]
    fn doctor_run_with_fix_resolves_a_missing_config_from_its_own_diagnostic() {
        let directory =
            std::env::temp_dir().join(format!("kept-doctor-run-fix-test-{}", std::process::id()));
        let config_path = directory.join("config.yaml");
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");

        let result = run_doctor_at(&config_path, true, None).expect("doctor fix run must succeed");
        assert!(result.fixed);
        assert!(result
            .report
            .findings
            .iter()
            .any(|finding| finding.code == "CONFIG_VALID"));
        assert!(config_path.is_file());

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
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
        assert!(help.contains("config.yaml"));
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
}

#[cfg(test)]
mod release_metadata_tests {
    use clap::CommandFactory;

    #[test]
    fn cli_metadata_uses_foundation_release_version() {
        assert_eq!(
            super::Cli::command().get_version().unwrap_or_default(),
            "0.3.1"
        );
    }
}

#[cfg(test)]
mod foundation_import_compatibility_tests {
    use super::commands::group::default_registry_group;
    use super::commands::registry::prepare_imported_registry;
    use kept_core::schema::{
        CustomFieldConfig, FieldSchema, KeywordGroup, KeywordRegistry, Metadata,
    };

    #[test]
    fn csv_import_is_migrated_before_the_current_schema_save_boundary() {
        let mut base_fields = std::collections::HashMap::new();
        base_fields.insert(
            "id".to_string(),
            FieldSchema {
                field_type: "string".to_string(),
                required: Some(true),
                description: "Unique identifier".to_string(),
                ..Default::default()
            },
        );
        base_fields.insert(
            "aliases".to_string(),
            FieldSchema {
                field_type: "array".to_string(),
                item_type: Some("string".to_string()),
                required: Some(true),
                description: "Search aliases".to_string(),
                ..Default::default()
            },
        );

        let legacy = KeywordRegistry {
            version: "1.1.0".to_string(),
            metadata: Metadata {
                last_updated: "2026-08-20T00:00:00Z".to_string(),
                description: "Legacy fixture".to_string(),
                owner: "tester".to_string(),
                ..Default::default()
            },
            groups: vec![KeywordGroup {
                group_id: "terms".to_string(),
                group_name: "Terms".to_string(),
                description: "Legacy group".to_string(),
                base_fields_schema: base_fields,
                custom_field_allowed: CustomFieldConfig {
                    enabled: true,
                    ..Default::default()
                },
                entries: vec![serde_json::json!({
                    "id": "term_one",
                    "aliases": ["term", "หนึ่ง"],
                })],
                group_stats: None,
            }],
            validation: Default::default(),
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        };

        let prepared = prepare_imported_registry(legacy).expect("registry must prepare");
        assert_eq!(prepared.version, "1.2.0");
        assert!(prepared.foundation.is_some());

        let starter_group = default_registry_group(
            "starter".to_string(),
            "Starter".to_string(),
            "Starter group".to_string(),
        );
        assert_eq!(starter_group.group_id, "starter");
        assert_eq!(starter_group.base_fields_schema.len(), 2);
    }
}

#[cfg(test)]
mod search_policy_command_tests {
    use super::Cli;
    use clap::Parser;

    #[tokio::test]
    async fn registry_search_rejects_invalid_user_search_policy_before_index_build() {
        let registry = serde_json::json!({
            "version": "1.2.0",
            "metadata": {
                "lastUpdated": "2026-08-20T00:00:00Z",
                "description": "Invalid policy",
                "owner": "tester"
            },
            "groups": [],
            "validation": {
                "rules": {
                    "aliasMinLength": 1,
                    "aliasMaxLength": 255,
                    "descriptionMinLength": 0,
                    "descriptionMaxLength": 10000,
                    "customFieldPerEntry": 50,
                    "requiredBaseFields": ["id", "aliases"]
                },
                "errorMessages": {}
            },
            "synonymSets": [],
            "searchPolicy": {
                "fuzzyMinSimilarity": 1.2
            },
            "foundation": {
                "schemaVersion": "1.2.0",
                "normalization": {
                    "encoding": "utf-8",
                    "unicodeForm": "nfc",
                    "whitespacePolicy": "collapse",
                    "casePolicy": "lowercase_latin",
                    "scriptPolicy": "preserve_non_latin"
                },
                "glossaryTerms": [],
                "provenanceRecords": [],
                "regexRules": [],
                "createdAt": "2026-08-20T00:00:00Z",
                "updatedAt": "2026-08-20T00:00:00Z"
            }
        });

        let directory =
            std::env::temp_dir().join(format!("kept-search-policy-cli-{}", std::process::id()));
        let registry_path = directory.join("invalid-policy-registry.json");
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        std::fs::write(
            &registry_path,
            serde_json::to_string_pretty(&registry).expect("registry fixture must serialize"),
        )
        .expect("registry fixture must be written");

        let command = Cli::try_parse_from([
            "kept",
            "search",
            registry_path.to_str().expect("valid path string"),
            "query",
        ]);
        assert!(command.is_ok(), "search command syntax is valid");

        let result = super::commands::registry::handle_registry(
            super::commands::registry::RegistryCommands::Search {
                path: registry_path,
                query: "query".to_string(),
                group: None,
                json: true,
                semantic: false,
                hybrid: false,
            },
        )
        .await;
        assert!(
            result.is_err(),
            "invalid search policy must be rejected before searching"
        );
        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains("fuzzyMinSimilarity")
                || error_message.contains("fuzzy_min_similarity"),
            "unexpected policy error: {error_message}"
        );

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(test)]
mod cli_command_topology_tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn cli_has_no_duplicate_short_options() {
        let app = Cli::command();
        let mut seen = std::collections::HashSet::new();
        for arg in app.get_arguments() {
            if let Some(short) = arg.get_short() {
                assert!(
                    seen.insert(short),
                    "duplicate short option -{short} on argument {}",
                    arg.get_id()
                );
            }
        }
    }

    #[test]
    fn duplicates_show_json_contract_emits_single_valid_json() {
        let temp_dir = std::env::temp_dir().join(format!("kept_dup_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("temp dir created");
        std::fs::write(temp_dir.join("f1.txt"), b"duplicate content").unwrap();
        std::fs::write(temp_dir.join("f2.txt"), b"duplicate content").unwrap();

        let index =
            kept_core::scan_directory(&temp_dir, &kept_core::ScanOptions::default()).unwrap();
        let snapshot =
            kept_core::create_persistent_snapshot(index, &kept_core::ScanOptions::default());
        let index_path = temp_dir.join("index.json");
        std::fs::write(&index_path, serde_json::to_string(&snapshot).unwrap()).unwrap();

        let result = crate::commands::duplicates::handle_task_duplicates(
            temp_dir.clone(),
            Some(index_path),
            true,
            Some(crate::commands::duplicates::DuplicateAction::Show),
            true,
        );
        assert!(result.is_ok(), "duplicates task must succeed");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
