use super::identity::{ContentIdentity, Revision};
use super::target::Target;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub adapter: String,
    pub target: Target,
    pub revision: Revision,
    pub identity: ContentIdentity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub source: Source,
    pub facts: serde_json::Value,
    pub confidence: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub actor: String,
    pub session_id: Option<String>,
    pub input_digest: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub source: Source,
    pub content: Option<String>,
    pub metadata: serde_json::Value,
    pub timestamp: u64,
}
