use super::identity::{ContentIdentity, Revision};
use super::target::Target;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    File,
    Grep,
    Fuzzy,
    Glob,
    Git,
    Diff,
    TreeSitter,
    Parser,
    Fts,
    Bm25,
    Vector,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub kind: SourceKind,
    pub adapter: String,
    pub target: Target,
    pub revision: Revision,
    pub identity: ContentIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceEvent {
    Created,
    Modified,
    Removed,
    Renamed { old_target: Target },
    GitChanged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructureOutlineItem {
    pub name: String,
    pub kind: String,
    pub range: Option<(usize, usize)>,
    pub children: Vec<StructureOutlineItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructurePayload {
    pub outline: Vec<StructureOutlineItem>,
    pub symbols: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub source: Source,
    pub target: Target,
    pub revision: Revision,
    pub event: Option<ResourceEvent>,
    pub structure: Option<StructurePayload>,
    pub evidence: Vec<Evidence>,
    pub content: Option<String>,
    pub metadata: serde_json::Value,
    pub provenance: Provenance,
}

impl Observation {
    /// Ergonomic helper returning the observation acquisition timestamp recorded in provenance
    pub fn timestamp(&self) -> u64 {
        self.provenance.timestamp
    }
}
