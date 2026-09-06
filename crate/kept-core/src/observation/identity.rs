use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Revision {
    pub modified_timestamp: u64,
    pub revision_token: Option<String>,
}

impl Revision {
    pub fn new(modified_timestamp: u64, revision_token: Option<String>) -> Self {
        Self {
            modified_timestamp,
            revision_token,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentIdentity {
    pub hash_algorithm: String,
    pub digest: String,
}

impl ContentIdentity {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = format!("{:x}", hasher.finalize());
        Self {
            hash_algorithm: "sha256".to_string(),
            digest: hash,
        }
    }

    pub fn hash(&self) -> &str {
        &self.digest
    }
}
