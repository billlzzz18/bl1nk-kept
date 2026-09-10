//! Filesystem Query Language (FQL) Parser and Compiler.
//!
//! Compiles concise query expressions like `ext:pdf size>50MB name:report`
//! or `dup:content` into typed [`FilterSet`] and execution plans.

use super::filter::{FileFilter, FilterSet};
use serde::{Deserialize, Serialize};

/// AST token or condition in the query expression
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryClause {
    Extension {
        value: String,
    },
    NameContains {
        value: String,
    },
    PathContains {
        value: String,
    },
    MinSize {
        bytes: u64,
    },
    MaxSize {
        bytes: u64,
    },
    DuplicateContent,
    DuplicateName,
    Kind {
        value: String,
    },
    ModifiedAfter {
        unix: u64,
    },
    ModifiedBefore {
        unix: u64,
    },
    Custom {
        field: String,
        op: String,
        value: String,
    },
}

/// Compilation result with human-readable explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub raw_query: String,
    pub clauses: Vec<QueryClause>,
    pub explanation: Vec<String>,
}

/// Parse human-readable byte sizes (e.g. "50MB", "1.5GB", "500KB", "1024")
pub fn parse_size_to_bytes(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (num_part, unit) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
        (&s[..pos], &s[pos..])
    } else {
        (s, "B")
    };

    let num: f64 = num_part
        .trim()
        .parse()
        .map_err(|_| format!("Invalid number in size: '{}'", s))?;

    let multiplier: u64 = match unit.to_uppercase().as_str() {
        "B" | "BYTES" => 1,
        "K" | "KB" => 1024,
        "M" | "MB" => 1024 * 1024,
        "G" | "GB" => 1024 * 1024 * 1024,
        "T" | "TB" => 1024 * 1024 * 1024 * 1024,
        _ => return Err(format!("Unknown size unit '{}' in '{}'", unit, s)),
    };

    Ok((num * multiplier as f64) as u64)
}

/// Parse and compile a query string into a [`FilterSet`] and [`QueryPlan`]
pub fn compile_query(query: &str) -> Result<(FilterSet, QueryPlan), String> {
    let mut filter_set = FilterSet::default();
    let mut clauses = Vec::new();
    let mut explanation = Vec::new();

    let tokens: Vec<&str> = query.split_whitespace().collect();

    for token in tokens {
        if token.is_empty() {
            continue;
        }

        if let Some(ext) = token.strip_prefix("ext:") {
            let clean_ext = ext.trim_start_matches('.').to_string();
            filter_set
                .all
                .push(FileFilter::Extension(clean_ext.clone()));
            clauses.push(QueryClause::Extension { value: clean_ext.clone() });
            explanation.push(format!("Filter by extension matching '.{}'", clean_ext));
            continue;
        }

        if let Some(name) = token.strip_prefix("name:") {
            filter_set
                .all
                .push(FileFilter::NameContains(name.to_string()));
            clauses.push(QueryClause::NameContains { value: name.to_string() });
            explanation.push(format!("Filter by file name containing '{}'", name));
            continue;
        }

        if let Some(path) = token.strip_prefix("path:") {
            filter_set
                .all
                .push(FileFilter::PathContains(path.to_string()));
            clauses.push(QueryClause::PathContains { value: path.to_string() });
            explanation.push(format!("Filter by path containing '{}'", path));
            continue;
        }

        if let Some(kind) = token.strip_prefix("kind:") {
            filter_set.all.push(FileFilter::Kind(kind.to_string()));
            clauses.push(QueryClause::Kind { value: kind.to_string() });
            explanation.push(format!("Filter by file kind '{}'", kind));
            continue;
        }

        if let Some(dup_type) = token.strip_prefix("dup:") {
            match dup_type.to_lowercase().as_str() {
                "content" | "exact" => {
                    clauses.push(QueryClause::DuplicateContent);
                    explanation.push("Filter files that have exact content duplicates".to_string());
                }
                "name" | "same_name" => {
                    clauses.push(QueryClause::DuplicateName);
                    explanation.push("Filter files that have matching names".to_string());
                }
                _ => {
                    return Err(format!(
                        "Unknown duplicate filter type: 'dup:{}' (valid: content, name)",
                        dup_type
                    ))
                }
            }
            continue;
        }

        if let Some(size_str) = token.strip_prefix("size>") {
            let bytes = parse_size_to_bytes(size_str)?;
            filter_set.all.push(FileFilter::MinSize(bytes));
            clauses.push(QueryClause::MinSize { bytes });
            explanation.push(format!("Filter files with size >= {} bytes ({})", bytes, size_str));
            continue;
        }

        if let Some(size_str) = token.strip_prefix("size>=") {
            let bytes = parse_size_to_bytes(size_str)?;
            filter_set.all.push(FileFilter::MinSize(bytes));
            clauses.push(QueryClause::MinSize { bytes });
            explanation.push(format!("Filter files with size >= {} bytes ({})", bytes, size_str));
            continue;
        }

        if let Some(size_str) = token.strip_prefix("size<") {
            let bytes = parse_size_to_bytes(size_str)?;
            filter_set.all.push(FileFilter::MaxSize(bytes));
            clauses.push(QueryClause::MaxSize { bytes });
            explanation.push(format!("Filter files with size <= {} bytes ({})", bytes, size_str));
            continue;
        }

        if let Some(size_str) = token.strip_prefix("size<=") {
            let bytes = parse_size_to_bytes(size_str)?;
            filter_set.all.push(FileFilter::MaxSize(bytes));
            clauses.push(QueryClause::MaxSize { bytes });
            explanation.push(format!("Filter files with size <= {} bytes ({})", bytes, size_str));
            continue;
        }

        if let Some(after_str) = token.strip_prefix("after:") {
            let unix = after_str
                .parse::<u64>()
                .map_err(|_| format!("Invalid unix timestamp in after: '{}'", after_str))?;
            filter_set.all.push(FileFilter::ModifiedAfter(unix));
            clauses.push(QueryClause::ModifiedAfter { unix });
            explanation.push(format!("Filter files modified after timestamp {}", unix));
            continue;
        }

        if let Some(before_str) = token.strip_prefix("before:") {
            let unix = before_str
                .parse::<u64>()
                .map_err(|_| format!("Invalid unix timestamp in before: '{}'", before_str))?;
            filter_set.all.push(FileFilter::ModifiedBefore(unix));
            clauses.push(QueryClause::ModifiedBefore { unix });
            explanation.push(format!("Filter files modified before timestamp {}", unix));
            continue;
        }

        // Bare keyword matches name
        filter_set
            .all
            .push(FileFilter::NameContains(token.to_string()));
        clauses.push(QueryClause::NameContains { value: token.to_string() });
        explanation.push(format!("Filter files containing '{}' in name", token));
    }

    let plan = QueryPlan {
        raw_query: query.to_string(),
        clauses,
        explanation,
    };

    Ok((filter_set, plan))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_query_basic() {
        let (filters, plan) = compile_query("ext:pdf size>50MB dup:content report").unwrap();
        assert_eq!(filters.all.len(), 3); // ext:pdf, size>50MB, report
        assert_eq!(plan.clauses.len(), 4);
        assert!(plan.explanation.iter().any(|e| e.contains("52428800")));
    }

    #[test]
    fn test_parse_size_units() {
        assert_eq!(parse_size_to_bytes("1024").unwrap(), 1024);
        assert_eq!(parse_size_to_bytes("1KB").unwrap(), 1024);
        assert_eq!(parse_size_to_bytes("50MB").unwrap(), 50 * 1024 * 1024);
        assert_eq!(parse_size_to_bytes("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
    }
}
