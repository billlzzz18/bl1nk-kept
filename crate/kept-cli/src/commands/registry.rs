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
        #[arg(long)]
        semantic: bool,
        #[arg(long, conflicts_with = "semantic")]
        hybrid: bool,
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

pub async fn handle_registry(cmd: RegistryCommands) -> anyhow::Result<()> {
    use kept_core::{
        import_csv, load_registry, save_registry, KeywordSearch, RegistryAnalyzer, Validator,
    };

    match cmd {
        RegistryCommands::Validate { path, entry_id, group } => {
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
            semantic,
            hybrid,
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
            let mut results = search.search(&query, group.as_deref());

            // NOTE-SEM-001: rerank pipeline — เรียก semantic reranker หลัง BM25 search
            // เพื่อ re-order results ด้วย embedding similarity
            if semantic || hybrid {
                results = rerank_with_semantic(&query, results, hybrid).await?;
            }

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

/// Rerank BM25 search results using the local semantic reranker.
///
/// When `blend` is true, combines BM25 score (40%) with reranker score (60%).
/// When `blend` is false (pure semantic), uses only the reranker score.
async fn rerank_with_semantic(
    query: &str,
    results: Vec<kept_core::schema::SearchResult>,
    blend: bool,
) -> anyhow::Result<Vec<kept_core::schema::SearchResult>> {
    use kept_core::semantic::{ensure_model_available, rerank_documents, resolve};

    if results.is_empty() {
        return Ok(results);
    }

    let config = resolve(&Default::default()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let _ = ensure_model_available(&config).await;

    let documents: Vec<String> = results.iter().map(build_rerank_document).collect();

    let ranked = rerank_documents(&config, query, &documents)
        .await
        .map_err(|e| anyhow::anyhow!("semantic rerank failed: {e}"))?;

    let mut reranked: Vec<kept_core::schema::SearchResult> = ranked
        .into_iter()
        .map(|(idx, rerank_score)| {
            let mut result = results[idx].clone();
            if blend {
                // NOTE-SEM-002: weighted blend — BM25 40%, reranker 60%
                result.score = result.score * 0.4 + rerank_score * 10.0 * 0.6;
            } else {
                result.score = rerank_score * 10.0;
            }
            result.match_type = if blend { "hybrid" } else { "semantic" }.to_string();
            result.confidence = Some(result.score.min(10.0) / 10.0);
            result
        })
        .collect();

    reranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    reranked.truncate(20);
    Ok(reranked)
}

/// Build document text from a search result for the reranker input.
fn build_rerank_document(result: &kept_core::schema::SearchResult) -> String {
    let mut text = result.id.clone();
    if !result.description.is_empty() {
        text.push(' ');
        text.push_str(&result.description);
    }
    for alias in &result.aliases {
        text.push(' ');
        text.push_str(alias);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cli;
    use clap::Parser;
    use kept_core::schema::SearchResult;

    fn mock_result(id: &str, description: &str, aliases: Vec<&str>, score: f64) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            group_id: "test".to_string(),
            description: description.to_string(),
            aliases: aliases.into_iter().map(str::to_string).collect(),
            match_type: "bm25".to_string(),
            score,
            semantic_context: None,
            usage_stats: None,
            language_score: None,
            confidence: None,
        }
    }

    #[test]
    fn rerank_document_text_includes_id_description_and_aliases() {
        let result = mock_result("invoice", "monthly report", vec!["ใบแจ้งหนี้"], 5.0);
        let text = build_rerank_document(&result);
        assert_eq!(text, "invoice monthly report ใบแจ้งหนี้");
    }

    #[test]
    fn rerank_document_text_skips_empty_description() {
        let result = mock_result("hello", "", vec!["สวัสดี"], 3.0);
        let text = build_rerank_document(&result);
        assert_eq!(text, "hello สวัสดี");
    }

    #[test]
    fn rerank_document_text_handles_no_aliases() {
        let result = mock_result("report", "Business summary", vec![], 4.0);
        let text = build_rerank_document(&result);
        assert_eq!(text, "report Business summary");
    }

    #[test]
    fn semantic_flag_cli_parses() {
        let cmd = Cli::try_parse_from(["kept", "search", "registry.json", "query", "--semantic"]);
        assert!(cmd.is_ok());
    }

    #[test]
    fn hybrid_flag_cli_parses() {
        let cmd = Cli::try_parse_from(["kept", "search", "registry.json", "query", "--hybrid"]);
        assert!(cmd.is_ok());
    }

    #[test]
    fn semantic_and_hybrid_conflict() {
        let cmd = Cli::try_parse_from([
            "kept",
            "search",
            "registry.json",
            "query",
            "--semantic",
            "--hybrid",
        ]);
        assert!(cmd.is_err());
    }
}
