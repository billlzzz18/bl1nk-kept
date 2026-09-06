use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "scheme", content = "path", rename_all = "snake_case")]
pub enum Target {
    File(String),
    Symbol(String),
    Search(String),
    Context(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetParseError {
    #[error("Missing scheme delimiter '://'")]
    MissingDelimiter,
    #[error("Unsupported target scheme '{0}'")]
    UnsupportedScheme(String),
    #[error("Empty target path")]
    EmptyPath,
}

impl Target {
    pub fn parse(s: &str) -> Result<Self, TargetParseError> {
        let (scheme, path) = s
            .split_once("://")
            .ok_or(TargetParseError::MissingDelimiter)?;

        if path.trim().is_empty() {
            return Err(TargetParseError::EmptyPath);
        }

        match scheme {
            "file" => Ok(Target::File(path.to_string())),
            "symbol" => Ok(Target::Symbol(path.to_string())),
            "search" => Ok(Target::Search(path.to_string())),
            "context" => Ok(Target::Context(path.to_string())),
            other => Err(TargetParseError::UnsupportedScheme(other.to_string())),
        }
    }

    pub fn scheme(&self) -> &'static str {
        match self {
            Target::File(_) => "file",
            Target::Symbol(_) => "symbol",
            Target::Search(_) => "search",
            Target::Context(_) => "context",
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Target::File(p) | Target::Symbol(p) | Target::Search(p) | Target::Context(p) => p,
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme(), self.path())
    }
}

impl FromStr for Target {
    type Err = TargetParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
