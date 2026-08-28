//! Duplicate detection algorithms based on name similarity and content hashing.

use super::types::{FileRecord, ScanIndex};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateOptions {
    #[serde(rename = "nameThreshold")]
    pub name_threshold: f64,
    #[serde(rename = "includeExtension")]
    pub include_extension: bool,
    #[serde(rename = "sizeBucketBytes", default = "default_size_bucket_bytes")]
    pub size_bucket_bytes: u64,
    #[serde(rename = "nameNgramSize", default = "default_name_ngram_size")]
    pub name_ngram_size: usize,
    #[serde(rename = "maxNgramPostings", default = "default_max_ngram_postings")]
    pub max_ngram_postings: usize,
}

impl Default for DuplicateOptions {
    fn default() -> Self {
        Self {
            name_threshold: 0.90,
            include_extension: false,
            size_bucket_bytes: default_size_bucket_bytes(),
            name_ngram_size: default_name_ngram_size(),
            max_ngram_postings: default_max_ngram_postings(),
        }
    }
}

fn default_size_bucket_bytes() -> u64 {
    4 * 1024
}

fn default_name_ngram_size() -> usize {
    3
}

fn default_max_ngram_postings() -> usize {
    256
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateAllowRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_a: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_b: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl DuplicateAllowRule {
    pub fn matches_group(&self, group: &DuplicateGroup) -> bool {
        if let Some(rule_hash) = &self.sha256 {
            if let Some(evidence) = &group.evidence {
                if evidence.full_sha256.eq_ignore_ascii_case(rule_hash) {
                    return true;
                }
            }
        }
        if let (Some(a), Some(b)) = (&self.path_a, &self.path_b) {
            let has_a = group.items.iter().any(|item| item == a);
            let has_b = group.items.iter().any(|item| item == b);
            if has_a && has_b {
                return true;
            }
        }
        if let Some(pattern) = &self.glob_pattern {
            let matches_all = group.items.iter().all(|item| item.contains(pattern));
            if matches_all {
                return true;
            }
        }
        false
    }
}

pub fn filter_allowed_duplicates(
    groups: Vec<DuplicateGroup>,
    allow_list: &[DuplicateAllowRule],
) -> Vec<DuplicateGroup> {
    if allow_list.is_empty() {
        return groups;
    }
    groups
        .into_iter()
        .filter(|group| !allow_list.iter().any(|rule| rule.matches_group(group)))
        .collect()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateGroup {
    pub kind: String,
    pub similarity: f64,
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<DuplicateEvidence>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateEvidence {
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    #[serde(rename = "partialSha256")]
    pub partial_sha256: String,
    #[serde(rename = "fullSha256")]
    pub full_sha256: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContentDuplicateOptions {
    #[serde(rename = "partialHashBytes", default = "default_partial_hash_bytes")]
    pub partial_hash_bytes: usize,
}

impl Default for ContentDuplicateOptions {
    fn default() -> Self {
        Self {
            partial_hash_bytes: default_partial_hash_bytes(),
        }
    }
}

fn default_partial_hash_bytes() -> usize {
    64 * 1024
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ContentDuplicateStats {
    pub file_count: usize,
    pub size_candidate_files: usize,
    pub partial_hash_files: usize,
    pub full_hash_files: usize,
    pub unreadable_files: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DuplicateSearchStats {
    pub file_count: usize,
    pub total_possible_pairs: usize,
    pub candidate_pairs: usize,
    pub compared_pairs: usize,
    pub skipped_high_frequency_postings: usize,
}

pub fn find_duplicates(index: &ScanIndex, options: &DuplicateOptions) -> Vec<DuplicateGroup> {
    find_duplicates_with_stats(index, options).0
}

pub fn find_duplicates_with_stats(
    index: &ScanIndex,
    options: &DuplicateOptions,
) -> (Vec<DuplicateGroup>, DuplicateSearchStats) {
    let mut stats = DuplicateSearchStats {
        file_count: index.files.len(),
        total_possible_pairs: index
            .files
            .len()
            .saturating_mul(index.files.len().saturating_sub(1))
            / 2,
        ..DuplicateSearchStats::default()
    };
    let normalized_names: Vec<_> = index
        .files
        .iter()
        .map(|file| duplicate_name(file, options))
        .collect();

    let mut exact = HashMap::<String, Vec<String>>::new();
    for (file, name) in index.files.iter().zip(&normalized_names) {
        exact
            .entry(name.clone())
            .or_default()
            .push(file.path.clone());
    }
    let mut groups = exact
        .values_mut()
        .filter(|items| items.len() > 1)
        .map(|items| {
            items.sort();
            DuplicateGroup {
                kind: "exact_name".to_string(),
                similarity: 1.0,
                items: items.clone(),
                evidence: None,
            }
        })
        .collect::<Vec<_>>();

    let ngram_size = options.name_ngram_size.max(1);
    let bucket_size = options.size_bucket_bytes.max(1);
    let mut staged_postings = HashMap::<(u64, String), Vec<usize>>::new();
    for (file_index, file) in index.files.iter().enumerate() {
        let bucket = file.size / bucket_size;
        let ngrams: HashSet<_> = name_ngrams(&normalized_names[file_index], ngram_size)
            .into_iter()
            .collect();
        for ngram in ngrams {
            staged_postings
                .entry((bucket, ngram))
                .or_default()
                .push(file_index);
        }
    }

    let mut candidate_pairs = HashSet::<(usize, usize)>::new();
    for postings in staged_postings.values() {
        if postings.len() > options.max_ngram_postings.max(1) {
            stats.skipped_high_frequency_postings += 1;
            continue;
        }
        for (offset, left) in postings.iter().enumerate() {
            for right in &postings[offset + 1..] {
                candidate_pairs.insert((*left.min(right), *left.max(right)));
            }
        }
    }
    stats.candidate_pairs = candidate_pairs.len();

    let mut candidate_pairs: Vec<_> = candidate_pairs.into_iter().collect();
    candidate_pairs.sort_unstable();
    for (left, right) in candidate_pairs {
        let left_name = &normalized_names[left];
        let right_name = &normalized_names[right];
        if left_name == right_name {
            continue;
        }
        stats.compared_pairs += 1;
        let similarity = normalized_similarity(left_name, right_name);
        if similarity >= options.name_threshold {
            let mut items = vec![
                index.files[left].path.clone(),
                index.files[right].path.clone(),
            ];
            items.sort();
            groups.push(DuplicateGroup {
                kind: "near_name".to_string(),
                similarity,
                items,
                evidence: None,
            });
        }
    }

    groups.sort_by(|left, right| {
        right
            .similarity
            .total_cmp(&left.similarity)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.items.cmp(&right.items))
    });
    (groups, stats)
}

pub fn find_content_duplicates(
    index: &ScanIndex,
    options: &ContentDuplicateOptions,
) -> std::io::Result<(Vec<DuplicateGroup>, ContentDuplicateStats)> {
    let mut stats = ContentDuplicateStats {
        file_count: index.files.len(),
        ..ContentDuplicateStats::default()
    };
    let mut by_size = HashMap::<u64, Vec<usize>>::new();
    for (file_index, file) in index.files.iter().enumerate() {
        by_size.entry(file.size).or_default().push(file_index);
    }

    let mut by_partial = HashMap::<(u64, Vec<u8>), Vec<usize>>::new();
    for (size, candidates) in by_size {
        if candidates.len() < 2 {
            continue;
        }
        stats.size_candidate_files += candidates.len();
        for file_index in candidates {
            let path = resolve_record_path(index, &index.files[file_index]);
            match partial_hash(&path, size, options.partial_hash_bytes.max(1)) {
                Ok(hash) => {
                    stats.partial_hash_files += 1;
                    by_partial.entry((size, hash)).or_default().push(file_index);
                }
                Err(_) => stats.unreadable_files += 1,
            }
        }
    }

    let mut groups = Vec::new();
    for ((size, partial_hash), candidates) in by_partial {
        if candidates.len() < 2 {
            continue;
        }
        let mut by_full = HashMap::<Vec<u8>, Vec<usize>>::new();
        for file_index in candidates {
            let path = resolve_record_path(index, &index.files[file_index]);
            match full_hash(&path) {
                Ok(hash) => {
                    stats.full_hash_files += 1;
                    by_full.entry(hash).or_default().push(file_index);
                }
                Err(_) => stats.unreadable_files += 1,
            }
        }
        for (full_hash, file_indexes) in by_full {
            if file_indexes.len() < 2 {
                continue;
            }
            let mut items = file_indexes
                .iter()
                .map(|file_index| index.files[*file_index].path.clone())
                .collect::<Vec<_>>();
            items.sort();
            let evidence = DuplicateEvidence {
                size_bytes: size,
                partial_sha256: hash_hex(&partial_hash),
                full_sha256: hash_hex(&full_hash),
            };
            let hard_link_groups = hard_link_subgroups(index, &file_indexes);
            let all_items_are_one_hard_link_group =
                hard_link_groups.len() == 1 && hard_link_groups[0].len() == file_indexes.len();

            if all_items_are_one_hard_link_group {
                groups.push(DuplicateGroup {
                    kind: "hard_link".to_string(),
                    similarity: 1.0,
                    items,
                    evidence: Some(evidence),
                });
                continue;
            }

            groups.push(DuplicateGroup {
                kind: "same_content".to_string(),
                similarity: 1.0,
                items,
                evidence: Some(evidence.clone()),
            });
            for hard_link_indexes in hard_link_groups {
                let mut hard_link_items = hard_link_indexes
                    .iter()
                    .map(|file_index| index.files[*file_index].path.clone())
                    .collect::<Vec<_>>();
                hard_link_items.sort();
                groups.push(DuplicateGroup {
                    kind: "hard_link".to_string(),
                    similarity: 1.0,
                    items: hard_link_items,
                    evidence: Some(evidence.clone()),
                });
            }
        }
    }
    groups.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.items.cmp(&right.items))
    });
    Ok((groups, stats))
}

pub fn resolve_record_path(index: &ScanIndex, record: &FileRecord) -> PathBuf {
    let path = Path::new(&record.path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(&index.root).join(path)
    }
}

pub fn partial_hash(path: &Path, size: u64, bytes: usize) -> std::io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut first = vec![0_u8; bytes.min(size as usize)];
    file.read_exact(&mut first)?;
    hasher.update(&first);
    if size > bytes as u64 {
        let tail_start = size.saturating_sub(bytes as u64);
        file.seek(SeekFrom::Start(tail_start))?;
        let mut tail = vec![0_u8; bytes.min(size as usize)];
        file.read_exact(&mut tail)?;
        hasher.update(&tail);
    }
    Ok(hasher.finalize().to_vec())
}

pub fn full_hash(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_vec())
}

pub fn hash_hex(hash: &[u8]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(unix)]
fn hard_link_subgroups(index: &ScanIndex, indexes: &[usize]) -> Vec<Vec<usize>> {
    use std::os::unix::fs::MetadataExt;

    let mut by_inode = BTreeMap::<(u64, u64), Vec<usize>>::new();
    for file_index in indexes {
        let path = resolve_record_path(index, &index.files[*file_index]);
        let Ok(metadata) = fs::metadata(path) else {
            continue;
        };
        if metadata.nlink() > 1 {
            by_inode
                .entry((metadata.dev(), metadata.ino()))
                .or_default()
                .push(*file_index);
        }
    }

    by_inode
        .into_values()
        .filter(|subgroup| subgroup.len() > 1)
        .collect()
}

#[cfg(not(unix))]
fn hard_link_subgroups(_index: &ScanIndex, _indexes: &[usize]) -> Vec<Vec<usize>> {
    Vec::new()
}

fn duplicate_name(file: &FileRecord, options: &DuplicateOptions) -> String {
    let name = if options.include_extension {
        file.name.clone()
    } else {
        stem_name(&file.name)
    };
    name.to_lowercase()
}

fn name_ngrams(name: &str, ngram_size: usize) -> Vec<String> {
    let characters: Vec<char> = name.chars().collect();
    if characters.is_empty() {
        return Vec::new();
    }
    if characters.len() <= ngram_size {
        return vec![characters.into_iter().collect()];
    }
    characters
        .windows(ngram_size)
        .map(|window| window.iter().collect())
        .collect()
}

pub fn normalized_similarity(left: &str, right: &str) -> f64 {
    let left_len = left.chars().count();
    let right_len = right.chars().count();
    let max_len = left_len.max(right_len);
    if max_len == 0 {
        return 1.0;
    }
    let distance = strsim::levenshtein(left, right);
    1.0 - (distance as f64 / max_len as f64)
}

fn stem_name(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(name)
        .to_string()
}
