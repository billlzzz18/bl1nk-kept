//! Treemap and Filter representations for scanned indexes.

use super::types::{FileRecord, ScanIndex};
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TreemapNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub kind: String,
    pub children: Vec<TreemapNode>,
}

#[derive(Debug, Clone)]
pub enum FileFilter {
    Extension(String),
    PathContains(String),
    NameContains(String),
    MinSize(u64),
    MaxSize(u64),
    ModifiedAfter(u64),
    ModifiedBefore(u64),
    Kind(String),
}

#[derive(Debug, Clone)]
pub enum CustomOperator {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterOrEqual,
    LessOrEqual,
}

#[derive(Debug, Clone)]
pub struct CustomFilter {
    pub field: String,
    pub operator: CustomOperator,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct FilterSet {
    pub all: Vec<FileFilter>,
    pub any: Vec<FileFilter>,
    pub custom: Vec<CustomFilter>,
}

pub fn build_treemap(index: &ScanIndex) -> TreemapNode {
    let mut root = TreemapNode {
        name: index.root.clone(),
        path: index.root.clone(),
        size: index.total_size,
        kind: "directory".to_string(),
        children: Vec::new(),
    };

    let mut directories = BTreeMap::<String, TreemapNode>::new();
    for file in &index.files {
        let top_level = file
            .path
            .split(['/', '\\'])
            .next()
            .unwrap_or(&file.path)
            .to_string();
        let entry = directories
            .entry(top_level.clone())
            .or_insert_with(|| TreemapNode {
                name: top_level.clone(),
                path: top_level,
                size: 0,
                kind: "directory".to_string(),
                children: Vec::new(),
            });
        entry.children.push(TreemapNode {
            name: file.name.clone(),
            path: file.path.clone(),
            size: file.size,
            kind: file.kind.clone(),
            children: Vec::new(),
        });
    }

    for node in directories.values_mut() {
        node.size = node.children.iter().map(|child| child.size).sum();
    }
    let mut nodes = directories.into_values().collect::<Vec<_>>();
    nodes.sort_by_key(|node| std::cmp::Reverse(node.size));
    root.children.extend(nodes);
    root.children
        .sort_by_key(|node| std::cmp::Reverse(node.size));
    root
}

pub fn filter_index<'a>(index: &'a ScanIndex, filters: &FilterSet) -> Vec<&'a FileRecord> {
    index
        .files
        .iter()
        .filter(|record| matches_filters(record, filters))
        .collect()
}

fn matches_filters(record: &FileRecord, filters: &FilterSet) -> bool {
    let all_pass = filters
        .all
        .iter()
        .all(|filter| matches_filter(record, filter));
    let any_pass = filters.any.is_empty()
        || filters
            .any
            .iter()
            .any(|filter| matches_filter(record, filter));
    let custom_pass = filters
        .custom
        .iter()
        .all(|filter| matches_custom(record, filter));
    all_pass && any_pass && custom_pass
}

fn matches_filter(record: &FileRecord, filter: &FileFilter) -> bool {
    match filter {
        FileFilter::Extension(value) => record
            .extension
            .eq_ignore_ascii_case(value.trim_start_matches('.')),
        FileFilter::PathContains(value) => {
            record.path.to_lowercase().contains(&value.to_lowercase())
        }
        FileFilter::NameContains(value) => {
            record.name.to_lowercase().contains(&value.to_lowercase())
        }
        FileFilter::MinSize(value) => record.size >= *value,
        FileFilter::MaxSize(value) => record.size <= *value,
        FileFilter::ModifiedAfter(value) => record.modified_unix >= *value,
        FileFilter::ModifiedBefore(value) => record.modified_unix <= *value,
        FileFilter::Kind(value) => record.kind.eq_ignore_ascii_case(value),
    }
}

fn matches_custom(record: &FileRecord, filter: &CustomFilter) -> bool {
    let target = match filter.field.as_str() {
        "path" => record.path.clone(),
        "name" => record.name.clone(),
        "extension" => record.extension.clone(),
        "kind" => record.kind.clone(),
        "size" => record.size.to_string(),
        "modified" => record.modified_unix.to_string(),
        _ => return false,
    };

    match filter.operator {
        CustomOperator::Equals => target.eq_ignore_ascii_case(&filter.value),
        CustomOperator::Contains => target.to_lowercase().contains(&filter.value.to_lowercase()),
        CustomOperator::StartsWith => target
            .to_lowercase()
            .starts_with(&filter.value.to_lowercase()),
        CustomOperator::EndsWith => target
            .to_lowercase()
            .ends_with(&filter.value.to_lowercase()),
        CustomOperator::GreaterOrEqual => target
            .parse::<u64>()
            .ok()
            .zip(filter.value.parse::<u64>().ok())
            .map(|(left, right)| left >= right)
            .unwrap_or(false),
        CustomOperator::LessOrEqual => target
            .parse::<u64>()
            .ok()
            .zip(filter.value.parse::<u64>().ok())
            .map(|(left, right)| left <= right)
            .unwrap_or(false),
    }
}
