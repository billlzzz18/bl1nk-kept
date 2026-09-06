//! Local semantic-model connection: OpenAI-compatible embedding and rerank endpoints.
//!
//! Model selection is by model ID; the provider is selected by endpoint URL.
//! Defaults target a local Ollama-compatible server that serves both an
//! embedding model and a reranker.

use serde::{Deserialize, Serialize};

pub const DEFAULT_SEMANTIC_ENDPOINT: &str = "http://127.0.0.1:11434/v1";
pub const DEFAULT_EMBEDDING_MODEL_ID: &str = "bge-m3";
pub const DEFAULT_RERANK_MODEL_ID: &str = "bge-reranker-v2-m3";

#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    #[error("invalid semantic endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("unsupported provider \"{0}\": supported providers are ollama (default) and local")]
    UnsupportedProvider(String),
    #[error(
        "cannot reach the {provider} provider at {endpoint}: is it installed and running? for ollama run `ollama serve`, or point KEPT_SEMANTIC_ENDPOINT at another supported provider (supported: ollama, jina)"
    )]
    ProviderUnavailable { provider: String, endpoint: String },
    #[error(
        "jina provider requires an API key: set the JINA_API_KEY environment variable or `defaults.semantic.apiKey` in config.yaml"
    )]
    MissingJinaApiKey,
    #[error("embedding or rerank model id must not be empty")]
    EmptyModelId,
    #[error("request failed: {0}")]
    Request(String),
    #[error("unexpected response from provider: {0}")]
    Response(String),
}

// NOTE-001: provider ที่รองรับมีแค่ ollama (local default) กับ jina (hosted, มีทั้ง embeddings และ rerank)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticProvider {
    Ollama,
    Local,
}

impl std::fmt::Display for SemanticProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticProvider::Ollama => write!(formatter, "ollama"),
            SemanticProvider::Local => write!(formatter, "local"),
        }
    }
}

impl SemanticProvider {
    pub const SUPPORTED: &'static str = "ollama, local";

    pub fn parse(value: &str) -> Result<Self, SemanticError> {
        match value.trim().to_lowercase().as_str() {
            "ollama" | "" => Ok(SemanticProvider::Ollama),
            "local" => Ok(SemanticProvider::Local),
            other => Err(SemanticError::UnsupportedProvider(other.to_string())),
        }
    }

    pub fn default_endpoint(self) -> &'static str {
        DEFAULT_SEMANTIC_ENDPOINT
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSemanticConfig {
    pub provider: SemanticProvider,
    pub endpoint: String,
    pub embedding_model_id: String,
    pub rerank_model_id: String,
    /// Resolved from `JINA_API_KEY` or `defaults.semantic.apiKey`; sent as a
    /// bearer token so YAML-configured keys are not silently dropped.
    pub api_key: Option<String>,
}

/// Apply built-in defaults to the user-owned config without rewriting their YAML.
///
/// Precedence: `KEPT_*` environment variables override config values, which
/// override the built-in defaults — mirroring how MCP servers take settings
/// through `environment`.
pub fn resolve(
    settings: &crate::policy::SemanticSearchSettings,
) -> Result<ResolvedSemanticConfig, SemanticError> {
    let from_env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
    let provider = SemanticProvider::parse(
        &from_env("KEPT_SEMANTIC_PROVIDER")
            .or_else(|| settings.provider.clone())
            .unwrap_or_default(),
    )?;
    let api_key = settings.api_key.clone();
    let endpoint = from_env("KEPT_SEMANTIC_ENDPOINT")
        .or_else(|| settings.endpoint.clone())
        .unwrap_or_else(|| provider.default_endpoint().to_string());
    let config = ResolvedSemanticConfig {
        provider,
        endpoint,
        api_key,
        embedding_model_id: from_env("KEPT_EMBEDDING_MODEL_ID")
            .or_else(|| settings.embedding_model_id.clone())
            .unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL_ID.to_string()),
        rerank_model_id: from_env("KEPT_RERANK_MODEL_ID")
            .or_else(|| settings.rerank_model_id.clone())
            .unwrap_or_else(|| DEFAULT_RERANK_MODEL_ID.to_string()),
    };
    validate_resolved(&config)?;
    Ok(config)
}

fn validate_resolved(config: &ResolvedSemanticConfig) -> Result<(), SemanticError> {
    if !(config.endpoint.starts_with("http://") || config.endpoint.starts_with("https://")) {
        return Err(SemanticError::InvalidEndpoint(config.endpoint.clone()));
    }
    if config.embedding_model_id.trim().is_empty() || config.rerank_model_id.trim().is_empty() {
        return Err(SemanticError::EmptyModelId);
    }
    Ok(())
}

pub fn validate_semantic_settings(
    settings: &crate::policy::SemanticSearchSettings,
) -> Result<(), crate::policy::PolicyError> {
    if let Some(endpoint) = &settings.endpoint {
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
            return Err(crate::policy::PolicyError::InvalidConfig(format!(
                "semantic.endpoint must be an http(s) URL: {endpoint}"
            )));
        }
    }
    let empty = |value: Option<&String>| value.is_some_and(|text| text.trim().is_empty());
    if empty(settings.embedding_model_id.as_ref()) || empty(settings.rerank_model_id.as_ref()) {
        return Err(crate::policy::PolicyError::InvalidConfig(
            "semantic model ids must not be empty".to_string(),
        ));
    }
    Ok(())
}

#[derive(Serialize)]
struct EmbeddingsRequestBody<'a> {
    model: &'a str,
    input: &'a [String],
}

pub fn embeddings_endpoint(endpoint: &str) -> String {
    format!("{}/embeddings", endpoint.trim_end_matches('/'))
}

pub fn embeddings_request_body(model_id: &str, inputs: &[String]) -> serde_json::Value {
    serde_json::to_value(EmbeddingsRequestBody {
        model: model_id,
        input: inputs,
    })
    .expect("embeddings request serializes")
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

pub fn parse_embeddings_response(body: &[u8]) -> Result<Vec<Vec<f32>>, SemanticError> {
    let parsed: EmbeddingsResponse = serde_json::from_slice(body)
        .map_err(|error| SemanticError::Response(format!("embeddings payload: {error}")))?;
    Ok(parsed.data.into_iter().map(|item| item.embedding).collect())
}

#[derive(Serialize)]
struct RerankRequestBody<'a> {
    model: &'a str,
    query: &'a str,
    documents: &'a [String],
}

pub fn rerank_endpoint(endpoint: &str) -> String {
    format!("{}/rerank", endpoint.trim_end_matches('/'))
}

pub fn rerank_request_body(model_id: &str, query: &str, documents: &[String]) -> serde_json::Value {
    serde_json::to_value(RerankRequestBody {
        model: model_id,
        query,
        documents,
    })
    .expect("rerank request serializes")
}

#[derive(Deserialize)]
struct RerankResponse {
    results: Vec<RerankResult>,
}

#[derive(Deserialize)]
struct RerankResult {
    index: usize,
    relevance_score: f64,
}

/// Parse a rerank response into `(document_index, score)` pairs sorted by descending score.
pub fn parse_rerank_response(body: &[u8]) -> Result<Vec<(usize, f64)>, SemanticError> {
    let parsed: RerankResponse = serde_json::from_slice(body)
        .map_err(|error| SemanticError::Response(format!("rerank payload: {error}")))?;
    let mut scores: Vec<(usize, f64)> = parsed
        .results
        .into_iter()
        .map(|result| (result.index, result.relevance_score))
        .collect();
    scores.sort_by(|left, right| right.1.total_cmp(&left.1));
    Ok(scores)
}

async fn post_json(
    config: &ResolvedSemanticConfig,
    url: String,
    payload: &serde_json::Value,
) -> Result<Vec<u8>, SemanticError> {
    let mut request = reqwest::Client::new().post(url).json(payload);
    if let Some(key) = &config.api_key {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(|error| {
        if error.is_connect() {
            SemanticError::ProviderUnavailable {
                provider: config.provider.to_string(),
                endpoint: config.endpoint.clone(),
            }
        } else {
            SemanticError::Request(error.to_string())
        }
    })?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|error| SemanticError::Request(error.to_string()))?;
    if !status.is_success() {
        return Err(SemanticError::Response(format!(
            "HTTP {status}: {}",
            String::from_utf8_lossy(&body)
        )));
    }
    Ok(body.to_vec())
}

/// Embed texts through an OpenAI-compatible `/embeddings` endpoint.
pub async fn embed_texts(
    config: &ResolvedSemanticConfig,
    inputs: &[String],
) -> Result<Vec<Vec<f32>>, SemanticError> {
    let body = post_json(
        config,
        embeddings_endpoint(&config.endpoint),
        &embeddings_request_body(&config.embedding_model_id, inputs),
    )
    .await?;
    parse_embeddings_response(&body)
}

/// Rerank documents against a query; returns `(index, score)` by descending score.
pub async fn rerank_documents(
    config: &ResolvedSemanticConfig,
    query: &str,
    documents: &[String],
) -> Result<Vec<(usize, f64)>, SemanticError> {
    let body = post_json(
        config,
        rerank_endpoint(&config.endpoint),
        &rerank_request_body(&config.rerank_model_id, query, documents),
    )
    .await?;
    parse_rerank_response(&body)
}

/// Ollama API origin for an OpenAI-compatible endpoint (`.../v1` -> server root).
pub fn ollama_origin(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed).to_string()
}

#[derive(Deserialize)]
struct OllamaTags {
    #[serde(default)]
    models: Vec<OllamaTagModel>,
}

#[derive(Deserialize)]
struct OllamaTagModel {
    name: String,
}

pub fn installed_ollama_models(tags_body: &[u8]) -> Result<Vec<String>, SemanticError> {
    let parsed: OllamaTags = serde_json::from_slice(tags_body)
        .map_err(|error| SemanticError::Response(format!("ollama tags payload: {error}")))?;
    Ok(parsed.models.into_iter().map(|model| model.name).collect())
}

pub fn needs_ollama_pull(installed: &[String], model_id: &str) -> bool {
    !installed
        .iter()
        .any(|name| name == model_id || name.split(':').next() == Some(model_id))
}

async fn get_bytes(url: &str) -> Result<reqwest::Response, SemanticError> {
    reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|error| SemanticError::Request(error.to_string()))
}

// NOTE-002: โปรแกรมต้องจัดการ pull เองเมื่อ provider เป็น Ollama; ห้ามโยนให้ผู้ใช้ไปรัน `ollama pull` เอง
///
/// Returns `Ok(true)` when a pull was performed, `Ok(false)` when the model was
/// already installed or the provider does not support local pulls (jina).
pub async fn ensure_model_available(
    config: &ResolvedSemanticConfig,
) -> Result<bool, SemanticError> {
    if config.provider != SemanticProvider::Ollama {
        return Ok(false);
    }
    let endpoint = config.endpoint.clone();
    let model_id = config.embedding_model_id.clone();
    let origin = ollama_origin(&endpoint);
    let tags = match get_bytes(&format!("{origin}/api/tags")).await {
        Ok(response) if response.status().is_success() => response.bytes().await,
        _ => return Ok(false),
    };
    let body = tags.map_err(|error| SemanticError::Request(error.to_string()))?;
    let installed = installed_ollama_models(&body)?;
    if !needs_ollama_pull(&installed, &model_id) {
        return Ok(false);
    }
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{origin}/api/pull"))
        .json(&serde_json::json!({"model": model_id, "stream": false}))
        .send()
        .await
        .map_err(|error| {
            if error.is_connect() {
                SemanticError::ProviderUnavailable {
                    provider: config.provider.to_string(),
                    endpoint: config.endpoint.clone(),
                }
            } else {
                SemanticError::Request(error.to_string())
            }
        })?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|error| SemanticError::Request(error.to_string()))?;
    if !status.is_success() {
        return Err(SemanticError::Response(format!(
            "HTTP {status}: {}",
            String::from_utf8_lossy(&body)
        )));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env() -> std::sync::MutexGuard<'static, ()> {
        let guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            std::env::remove_var("KEPT_SEMANTIC_PROVIDER");
            std::env::remove_var("KEPT_SEMANTIC_ENDPOINT");
            std::env::remove_var("KEPT_EMBEDDING_MODEL_ID");
            std::env::remove_var("KEPT_RERANK_MODEL_ID");
            std::env::remove_var("JINA_API_KEY");
        }
        guard
    }

    #[test]
    fn defaults_target_a_local_provider_with_embedding_and_rerank_ids() {
        let _guard = with_clean_env();
        let resolved = resolve(&Default::default()).unwrap();
        assert_eq!(resolved.endpoint, "http://127.0.0.1:11434/v1");
        assert_eq!(resolved.embedding_model_id, "bge-m3");
        assert_eq!(resolved.rerank_model_id, "bge-reranker-v2-m3");
    }

    #[test]
    fn explicit_model_ids_override_defaults_without_touching_other_fields() {
        let _guard = with_clean_env();
        let resolved = resolve(&crate::policy::SemanticSearchSettings {
            embedding_model_id: Some("nomic-embed-text".to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(resolved.embedding_model_id, "nomic-embed-text");
        assert_eq!(resolved.rerank_model_id, DEFAULT_RERANK_MODEL_ID);
    }

    #[test]
    fn rejects_non_http_endpoints_and_empty_model_ids() {
        let _guard = with_clean_env();
        assert!(matches!(
            resolve(&crate::policy::SemanticSearchSettings {
                endpoint: Some("ftp://x".into()),
                ..Default::default()
            }),
            Err(SemanticError::InvalidEndpoint(_))
        ));
        assert!(matches!(
            resolve(&crate::policy::SemanticSearchSettings {
                embedding_model_id: Some("  ".into()),
                ..Default::default()
            }),
            Err(SemanticError::EmptyModelId)
        ));
    }

    #[test]
    fn embeddings_request_targets_embeddings_path_with_model_and_inputs() {
        assert_eq!(
            embeddings_endpoint("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434/v1/embeddings"
        );
        let body = embeddings_request_body("bge-m3", &["first".to_string(), "second".to_string()]);
        assert_eq!(body["model"], "bge-m3");
        assert_eq!(body["input"][1], "second");
    }

    #[test]
    fn parses_openai_style_embeddings_payload_in_order() {
        let vectors = parse_embeddings_response(
            br#"{"data":[{"embedding":[1.0,0.0]},{"embedding":[0.5,0.5]}]}"#,
        )
        .unwrap();
        assert_eq!(vectors.len(), 2);
        assert_eq!(vectors[1], vec![0.5, 0.5]);
    }

    #[test]
    fn rerank_request_targets_rerank_path_and_parses_scores_sorted_by_relevance() {
        assert_eq!(
            rerank_endpoint("http://127.0.0.1:11434/v1/"),
            "http://127.0.0.1:11434/v1/rerank"
        );
        let body = rerank_request_body("bge-reranker-v2-m3", "invoice", &["a".into(), "b".into()]);
        assert_eq!(body["query"], "invoice");
        assert_eq!(body["documents"][1], "b");

        let ranked = parse_rerank_response(
            br#"{"results":[{"index":1,"relevance_score":0.4},{"index":0,"relevance_score":0.9}]}"#,
        )
        .unwrap();
        assert_eq!(ranked, vec![(0, 0.9), (1, 0.4)]);
    }

    #[test]
    fn env_overrides_take_precedence_over_config_and_defaults() {
        let _guard = with_clean_env();
        unsafe {
            std::env::set_var("KEPT_SEMANTIC_ENDPOINT", "http://127.0.0.1:20128/v1");
            std::env::set_var("KEPT_EMBEDDING_MODEL_ID", "nomic-embed-text");
        }
        let resolved = resolve(&crate::policy::SemanticSearchSettings {
            endpoint: Some("http://ignored.example:1/v1".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(resolved.endpoint, "http://127.0.0.1:20128/v1");
        assert_eq!(resolved.embedding_model_id, "nomic-embed-text");
        assert_eq!(resolved.rerank_model_id, DEFAULT_RERANK_MODEL_ID);
    }

    #[test]
    fn ollama_pull_is_attempted_only_when_the_model_is_missing_from_tags() {
        struct Case<'a> {
            tags: &'a str,
            expect_pull: bool,
        }
        let cases = [
            Case {
                tags: r#"{"models":[{"name":"bge-m3"},{"name":"llama3"}]}"#,
                expect_pull: false,
            },
            Case {
                tags: r#"{"models":[{"name":"llama3"}]}"#,
                expect_pull: true,
            },
        ];
        for case in cases {
            let installed = installed_ollama_models(case.tags.as_bytes()).unwrap();
            assert_eq!(
                needs_ollama_pull(&installed, "bge-m3"),
                case.expect_pull,
                "tags={}",
                case.tags
            );
        }
        assert_eq!(
            ollama_origin("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434"
        );
    }

    #[test]
    fn provider_defaults_to_ollama_and_local_resolves_endpoint() {
        let _guard = with_clean_env();
        let ollama = resolve(&Default::default()).unwrap();
        assert_eq!(ollama.provider, SemanticProvider::Ollama);
        assert_eq!(ollama.endpoint, DEFAULT_SEMANTIC_ENDPOINT);

        let local = resolve(&crate::policy::SemanticSearchSettings {
            provider: Some("local".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(local.provider, SemanticProvider::Local);
        assert_eq!(local.endpoint, DEFAULT_SEMANTIC_ENDPOINT);
    }

    #[test]
    fn unknown_provider_names_are_rejected_with_the_supported_list() {
        let error = resolve(&crate::policy::SemanticSearchSettings {
            provider: Some("openai".into()),
            ..Default::default()
        })
        .unwrap_err();
        assert!(
            error.to_string().contains("ollama") && error.to_string().contains("local"),
            "error must name the supported providers: {error}"
        );
    }

    #[test]
    fn user_config_semantic_section_validates_endpoint_and_model_ids() {
        use crate::policy::{SemanticSearchSettings, UserConfig};
        let mut config = UserConfig::starter();
        config.defaults.semantic = SemanticSearchSettings {
            endpoint: Some("notaurl".into()),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let mut config = UserConfig::starter();
        config.defaults.semantic = SemanticSearchSettings {
            embedding_model_id: Some("bge-m3".into()),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }
}
