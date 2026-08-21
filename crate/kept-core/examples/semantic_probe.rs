//! Probe the configured local semantic provider: resolve defaults, then try
//! embedding and reranking to verify the endpoint answers correctly.

use kept_core::policy::{SemanticSearchSettings, UserConfig};
use kept_core::semantic;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = kept_core::policy::default_user_config_path()?;
    let settings = if config_path.exists() {
        let config: UserConfig = serde_yaml::from_str(&std::fs::read_to_string(&config_path)?)?;
        config.defaults.semantic
    } else {
        SemanticSearchSettings::default()
    };
    let resolved = semantic::resolve(&settings)?;
    println!(
        "endpoint={} embeddingModelId={} rerankModelId={}",
        resolved.endpoint, resolved.embedding_model_id, resolved.rerank_model_id
    );

    match semantic::ensure_model_available(&resolved).await {
        Ok(true) => println!("pull=pulled {}", resolved.embedding_model_id),
        Ok(false) => println!("pull=already-installed-or-not-ollama"),
        Err(error) => println!("pull=failed ({error})"),
    }

    let embed = semantic::embed_texts(&resolved, &["probe".to_string()]).await;
    match embed {
        Ok(vectors) => println!(
            "embeddings=ok dimensions={} count={}",
            vectors[0].len(),
            vectors.len()
        ),
        Err(error) => println!("embeddings=unavailable ({error})"),
    }

    let ranked = semantic::rerank_documents(
        &resolved,
        "report",
        &["invoice".to_string(), "report".to_string()],
    )
    .await;
    match ranked {
        Ok(scores) => println!("rerank=ok scores={scores:?}"),
        Err(error) => println!("rerank=unavailable ({error})"),
    }
    Ok(())
}
