//! Legacy registry search, analyze, and CSV import command handlers.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum RegistryCommands {
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

pub fn prepare_imported_registry(
    registry: kept_core::schema::KeywordRegistry,
) -> anyhow::Result<kept_core::schema::KeywordRegistry> {
    kept_core::migrate_registry(registry).map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub fn handle_registry(cmd: RegistryCommands) -> anyhow::Result<()> {
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
            let mut failed = false;

            if let Some(id) = entry_id {
                let errors = validator.validate_entry_by_id(&id);
                if errors.is_empty() {
                    println!("Entry {} is valid!", id);
                } else {
                    println!("Entry {} has errors:", id);
                    for e in errors {
                        println!("- {}", e);
                    }
                    failed = true;
                }
            } else if let Some(grp) = group {
                let errors = validator.validate_group(&grp);
                if errors.is_empty() {
                    println!("Group {} is valid!", grp);
                } else {
                    println!("Group {} has errors:", grp);
                    for e in errors {
                        println!("- {}", e);
                    }
                    failed = true;
                }
            } else {
                let errors = validator.validate_all();
                if errors.is_empty() {
                    println!("Registry is valid!");
                } else {
                    println!("Registry has errors:");
                    for e in errors {
                        println!("- {}", e);
                    }
                    failed = true;
                }
            }

            if failed {
                anyhow::bail!("validation failed")
            }
        }
        RegistryCommands::Search {
            path,
            query,
            group,
            json,
        } => {
            let registry = load_registry(path).map_err(|error| anyhow::anyhow!("{error}"))?;
            let validator = Validator::new(registry.clone());
            validator.validate_search_policy().map_err(|errors| {
                let msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                anyhow::anyhow!("{msg}")
            })?;
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
