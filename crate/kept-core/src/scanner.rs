// NOTE-001: scanner รองรับการสแกนไฟล์จริง เก็บดัชนีครั้งแรก และใช้ดัชนีเดิมซ้ำในการกรอง/วิเคราะห์
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    #[serde(rename = "modifiedUnix")]
    pub modified_unix: u64,
    pub kind: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanIndex {
    pub root: String,
    #[serde(rename = "scannedAtUnix")]
    pub scanned_at_unix: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub files: Vec<FileRecord>,
    #[serde(default)]
    pub issues: Vec<ScanIssue>,
}

/// NOTE-001: issue เป็นผลจากการสแกนที่บันทึกไว้เพื่อให้ review ตรวจจุดที่ข้ามได้โดยไม่ทำให้ทั้ง index ใช้ไม่ได้
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ScanIssue {
    pub path: String,
    pub operation: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RefreshPlan {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    pub unchanged: Vec<String>,
}

pub const CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION: &str = "1.2.0";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentScanSnapshot {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub root: String,
    #[serde(rename = "rootFingerprint")]
    pub root_fingerprint: String,
    #[serde(rename = "includeHidden")]
    pub include_hidden: bool,
    #[serde(rename = "maxDepth")]
    pub max_depth: Option<usize>,
    pub index: ScanIndex,
}

/// NOTE-001: snapshot เก็บ fingerprint ของ root/options เพื่อไม่ refresh ข้าม context โดยไม่ตั้งใจ
pub fn create_persistent_snapshot(
    index: ScanIndex,
    options: &ScanOptions,
) -> PersistentScanSnapshot {
    let root_fingerprint = scan_context_fingerprint(&index.root, options);
    PersistentScanSnapshot {
        schema_version: CURRENT_SCAN_SNAPSHOT_SCHEMA_VERSION.to_string(),
        root: index.root.clone(),
        root_fingerprint,
        include_hidden: options.include_hidden,
        max_depth: options.max_depth,
        index,
    }
}

fn scan_context_fingerprint(root: &str, options: &ScanOptions) -> String {
    let mut digest = Sha256::new();
    digest.update(root.as_bytes());
    digest.update([0]);
    digest.update([u8::from(options.include_hidden)]);
    digest.update([0]);
    digest.update(options.max_depth.unwrap_or(usize::MAX).to_le_bytes());
    format!("{:x}", digest.finalize())
}

/// NOTE-001: refresh plan เป็นผลต่างแบบ deterministic เพื่อให้ caller review ก่อนทำงานต่อ
pub fn plan_incremental_refresh(
    previous: &ScanIndex,
    current: &ScanIndex,
) -> Result<RefreshPlan, String> {
    if previous.root != current.root {
        return Err("Cannot refresh indexes with different roots".to_string());
    }
    let previous_by_path = previous
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let current_by_path = current
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut plan = RefreshPlan::default();

    for (path, file) in &current_by_path {
        match previous_by_path.get(path) {
            None => plan.added.push((*path).to_string()),
            Some(previous_file) if file_changed(previous_file, file) => {
                plan.modified.push((*path).to_string())
            }
            Some(_) => plan.unchanged.push((*path).to_string()),
        }
    }
    for path in previous_by_path.keys() {
        if !current_by_path.contains_key(path) {
            plan.removed.push((*path).to_string());
        }
    }
    Ok(plan)
}

fn file_changed(previous: &FileRecord, current: &FileRecord) -> bool {
    previous.size != current.size
        || previous.modified_unix != current.modified_unix
        || previous.kind != current.kind
        || previous.extension != current.extension
}

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
    // NOTE-001: all เงื่อนไขต้องผ่าน และ any ต้องผ่านอย่างน้อยหนึ่งข้อถ้ามีการกำหนด
    pub all: Vec<FileFilter>,
    pub any: Vec<FileFilter>,
    pub custom: Vec<CustomFilter>,
}

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

/// NOTE-001: เลือก n-gram 3 จาก experiment `foundation-th-cc0-r1`; ดู raw/summary ใน benchmarks/data/foundation_th_cc0_r1
fn default_name_ngram_size() -> usize {
    3
}

/// NOTE-001: cap 256 ผ่าน safety envelope ของ baseline พร้อมลด candidate workload ใน repeated sweep
fn default_max_ngram_postings() -> usize {
    256
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

pub fn scan_directory(root: impl AsRef<Path>, options: &ScanOptions) -> std::io::Result<ScanIndex> {
    let root = root.as_ref().canonicalize()?;
    let mut files = Vec::new();
    let mut issues = Vec::new();
    visit_directory(&root, &root, options, 0, &mut files, &mut issues);
    let total_size = files.iter().map(|file| file.size).sum();
    Ok(ScanIndex {
        root: root.display().to_string(),
        scanned_at_unix: unix_now(),
        total_size,
        files,
        issues,
    })
}

fn visit_directory(
    root: &Path,
    current: &Path,
    options: &ScanOptions,
    depth: usize,
    files: &mut Vec<FileRecord>,
    issues: &mut Vec<ScanIssue>,
) {
    if options.max_depth.is_some_and(|max| depth > max) {
        return;
    }

    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(error) => {
            record_scan_issue(issues, current, "read_dir", error);
            return;
        }
    };
    for item in entries {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                record_scan_issue(issues, current, "read_dir_entry", error);
                continue;
            }
        };
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();
        if !options.include_hidden && name.starts_with('.') {
            continue;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                record_scan_issue(issues, &path, "metadata", error);
                continue;
            }
        };
        if metadata.is_dir() {
            visit_directory(root, &path, options, depth + 1, files, issues);
            continue;
        }
        if !metadata.is_file() {
            continue;
        }

        let relative = path.strip_prefix(root).unwrap_or(&path);
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let modified_unix = match metadata.modified() {
            Ok(modified) => to_unix(modified),
            Err(error) => {
                record_scan_issue(issues, &path, "modified", error);
                0
            }
        };
        files.push(FileRecord {
            path: relative.display().to_string(),
            name,
            extension,
            size: metadata.len(),
            modified_unix,
            kind: "file".to_string(),
        });
    }
}

fn record_scan_issue(
    issues: &mut Vec<ScanIssue>,
    path: &Path,
    operation: &str,
    error: std::io::Error,
) {
    issues.push(ScanIssue {
        path: path.display().to_string(),
        operation: operation.to_string(),
        message: error.to_string(),
    });
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
    let field_value = match filter.field.to_lowercase().as_str() {
        "path" => record.path.clone(),
        "name" => record.name.clone(),
        "extension" => record.extension.clone(),
        "kind" => record.kind.clone(),
        "size" => record.size.to_string(),
        "modified" | "modifiedunix" => record.modified_unix.to_string(),
        _ => return false,
    };
    match filter.operator {
        CustomOperator::Equals => field_value.eq_ignore_ascii_case(&filter.value),
        CustomOperator::Contains => field_value
            .to_lowercase()
            .contains(&filter.value.to_lowercase()),
        CustomOperator::StartsWith => field_value
            .to_lowercase()
            .starts_with(&filter.value.to_lowercase()),
        CustomOperator::EndsWith => field_value
            .to_lowercase()
            .ends_with(&filter.value.to_lowercase()),
        CustomOperator::GreaterOrEqual => {
            field_value.parse::<u64>().unwrap_or(0)
                >= filter.value.parse::<u64>().unwrap_or(u64::MAX)
        }
        CustomOperator::LessOrEqual => {
            field_value.parse::<u64>().unwrap_or(u64::MAX)
                <= filter.value.parse::<u64>().unwrap_or(0)
        }
    }
}

pub fn build_treemap(index: &ScanIndex) -> TreemapNode {
    let mut root = TreemapNode {
        name: Path::new(&index.root)
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| index.root.clone()),
        path: index.root.clone(),
        size: index.total_size,
        kind: "directory".to_string(),
        children: Vec::new(),
    };

    let mut directories: HashMap<String, TreemapNode> = HashMap::new();
    for file in &index.files {
        let components: Vec<&str> = Path::new(&file.path)
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect();
        let mut parent = String::new();
        for component in components.iter().take(components.len().saturating_sub(1)) {
            if !parent.is_empty() {
                parent.push('/');
            }
            parent.push_str(component);
            directories
                .entry(parent.clone())
                .or_insert_with(|| TreemapNode {
                    name: component.to_string(),
                    path: parent.clone(),
                    size: 0,
                    kind: "directory".to_string(),
                    children: Vec::new(),
                });
        }
        let file_path = file.path.clone();
        let file_node = TreemapNode {
            name: file.name.clone(),
            path: file_path.clone(),
            size: file.size,
            kind: "file".to_string(),
            children: Vec::new(),
        };
        let parent_path = components
            .iter()
            .take(components.len().saturating_sub(1))
            .copied()
            .collect::<Vec<_>>()
            .join("/");
        if parent_path.is_empty() {
            root.children.push(file_node);
        } else {
            directories
                .entry(parent_path.clone())
                .or_insert_with(|| TreemapNode {
                    name: parent_path.clone(),
                    path: parent_path.clone(),
                    size: 0,
                    kind: "directory".to_string(),
                    children: Vec::new(),
                })
                .children
                .push(file_node);
        }
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

pub fn find_duplicates(index: &ScanIndex, options: &DuplicateOptions) -> Vec<DuplicateGroup> {
    find_duplicates_with_stats(index, options).0
}

/// NOTE-001: สร้าง candidate pairs จาก size bucket และ shared n-gram ก่อนคำนวณ Levenshtein
/// เพื่อให้การเทียบชื่อใกล้เคียงไม่ต้องวนทุกคู่ของไฟล์ทั้งหมด
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

/// NOTE-001: ยืนยันไฟล์ซ้ำจากเนื้อหาด้วย size gate → partial SHA-256 → full SHA-256
/// ไม่รวม mutation ใด ๆ; caller ตัดสินใจ action จาก groups ที่ได้ภายหลังเท่านั้น
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

fn resolve_record_path(index: &ScanIndex, record: &FileRecord) -> PathBuf {
    let path = Path::new(&record.path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(&index.root).join(path)
    }
}

fn partial_hash(path: &Path, size: u64, bytes: usize) -> std::io::Result<Vec<u8>> {
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

fn full_hash(path: &Path) -> std::io::Result<Vec<u8>> {
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

fn hash_hex(hash: &[u8]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// NOTE-001: แตก subgroup จาก inode เดียวกัน เพื่อให้ copy ที่เนื้อหาเท่ากันไม่กลบ hard-link relation
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
    let a = left.to_lowercase();
    let b = right.to_lowercase();
    if a == b {
        return 1.0;
    }
    let max_len = a.chars().count().max(b.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - levenshtein(&a, &b) as f64 / max_len as f64
}

fn levenshtein(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    for (i, left_char) in left.chars().enumerate() {
        let mut current = vec![i + 1; right_chars.len() + 1];
        for (j, right_char) in right_chars.iter().enumerate() {
            current[j + 1] = (current[j] + 1)
                .min(previous[j + 1] + 1)
                .min(previous[j] + usize::from(left_char != *right_char));
        }
        previous = current;
    }
    previous[right_chars.len()]
}

fn stem_name(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
fn to_unix(value: SystemTime) -> u64 {
    value
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_index_persists_structured_scan_issues() {
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: 0,
            files: Vec::new(),
            issues: vec![ScanIssue {
                path: "locked".into(),
                operation: "read_dir".into(),
                message: "permission denied".into(),
            }],
        };

        let value = serde_json::to_value(index).expect("index must serialize");
        assert_eq!(value["issues"][0]["path"], "locked");
        assert_eq!(value["issues"][0]["operation"], "read_dir");
    }

    #[test]
    fn similarity_is_adjustable_and_normalized() {
        assert_eq!(normalized_similarity("report.txt", "report.txt"), 1.0);
        assert!(normalized_similarity("report", "reports") > 0.8);
    }

    fn file(path: &str, name: &str, size: u64) -> FileRecord {
        FileRecord {
            path: path.to_string(),
            name: name.to_string(),
            extension: Path::new(name)
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_string(),
            size,
            modified_unix: 0,
            kind: "file".to_string(),
        }
    }

    #[test]
    fn staged_detection_preserves_exact_and_near_name_matches() {
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: 12_000,
            files: vec![
                file("one/report.md", "report.md", 1),
                file("two/report.txt", "report.txt", 10_000),
                file("three/report-2024.md", "report-2024.md", 512),
                file("four/report-2025.md", "report-2025.md", 513),
            ],
            issues: Vec::new(),
        };
        let options = DuplicateOptions {
            name_threshold: 0.90,
            ..DuplicateOptions::default()
        };

        let (groups, stats) = find_duplicates_with_stats(&index, &options);
        assert!(groups.iter().any(|group| {
            group.kind == "exact_name"
                && group.items == vec!["one/report.md".to_string(), "two/report.txt".to_string()]
        }));
        assert!(groups.iter().any(|group| {
            group.kind == "near_name"
                && group.items
                    == vec![
                        "four/report-2025.md".to_string(),
                        "three/report-2024.md".to_string(),
                    ]
        }));
        assert!(stats.compared_pairs < stats.total_possible_pairs);
    }

    #[test]
    fn staged_detection_avoids_quadratic_comparisons_on_large_input() {
        let mut files = Vec::new();
        for index in 0..1_000 {
            files.push(file(
                &format!("data/item-{index:04}.txt"),
                &format!("item-{index:04}.txt"),
                32_768,
            ));
        }
        files.push(file("data/report-2024.md", "report-2024.md", 32_768));
        files.push(file("data/report-2025.md", "report-2025.md", 32_768));
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: files.iter().map(|file| file.size).sum(),
            files,
            issues: Vec::new(),
        };
        let options = DuplicateOptions {
            name_ngram_size: 2,
            max_ngram_postings: 16,
            ..DuplicateOptions::default()
        };

        let (groups, stats) = find_duplicates_with_stats(&index, &options);
        assert!(groups.iter().any(|group| {
            group.kind == "near_name"
                && group.items
                    == vec![
                        "data/report-2024.md".to_string(),
                        "data/report-2025.md".to_string(),
                    ]
        }));
        assert!(stats.compared_pairs < stats.total_possible_pairs / 100);
        assert!(stats.skipped_high_frequency_postings > 0);
    }

    #[test]
    fn filters_are_composable() {
        let index = ScanIndex {
            root: "/tmp".into(),
            scanned_at_unix: 0,
            total_size: 10,
            files: vec![FileRecord {
                path: "docs/readme.md".into(),
                name: "readme.md".into(),
                extension: "md".into(),
                size: 10,
                modified_unix: 100,
                kind: "file".into(),
            }],
            issues: Vec::new(),
        };
        let filters = FilterSet {
            all: vec![FileFilter::Extension("md".into()), FileFilter::MinSize(10)],
            any: vec![],
            custom: vec![CustomFilter {
                field: "path".into(),
                operator: CustomOperator::Contains,
                value: "docs".into(),
            }],
        };
        assert_eq!(filter_index(&index, &filters).len(), 1);
    }
}

#[cfg(test)]
mod content_duplicate_tests {
    use super::*;

    fn temporary_directory(label: &str) -> std::path::PathBuf {
        let unique = format!(
            "bl1nk-kept-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        directory
    }

    #[test]
    fn content_scan_groups_equal_bytes_and_rejects_equal_size_different_bytes() {
        let directory = temporary_directory("content-scan");
        std::fs::write(
            directory.join("report-a.txt"),
            b"same payload for verified duplicate",
        )
        .expect("first fixture must be written");
        std::fs::write(
            directory.join("report-b.txt"),
            b"same payload for verified duplicate",
        )
        .expect("second fixture must be written");
        std::fs::write(
            directory.join("same-size-other.txt"),
            vec![b'x'; b"same payload for verified duplicate".len()],
        )
        .expect("different-content fixture must be written");

        let index = scan_directory(&directory, &ScanOptions::default())
            .expect("fixture directory must be indexed");
        let (groups, stats) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content scan must complete");

        assert!(groups.iter().any(|group| {
            group.kind == "same_content"
                && group.items == vec!["report-a.txt".to_string(), "report-b.txt".to_string()]
                && group.similarity == 1.0
        }));
        assert!(!groups.iter().any(|group| {
            group.kind == "same_content" && group.items.contains(&"same-size-other.txt".to_string())
        }));
        assert_eq!(stats.full_hash_files, 2);

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(all(test, unix))]
mod mixed_content_duplicate_tests {
    use super::*;

    #[test]
    fn content_scan_reports_hard_link_subgroup_when_copies_share_the_same_hash() {
        let directory = std::env::temp_dir().join(format!(
            "bl1nk-kept-mixed-content-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("temporary directory must be created");
        std::fs::write(directory.join("source.txt"), b"same verified payload")
            .expect("source fixture must be written");
        std::fs::hard_link(
            directory.join("source.txt"),
            directory.join("source-link.txt"),
        )
        .expect("hard link fixture must be created");
        std::fs::copy(directory.join("source.txt"), directory.join("copy.txt"))
            .expect("copy fixture must be created");

        let index = scan_directory(&directory, &ScanOptions::default())
            .expect("fixture directory must be indexed");
        let (groups, _) = find_content_duplicates(&index, &ContentDuplicateOptions::default())
            .expect("content scan must complete");

        assert!(groups.iter().any(|group| {
            group.kind == "same_content"
                && group.items
                    == vec![
                        "copy.txt".to_string(),
                        "source-link.txt".to_string(),
                        "source.txt".to_string(),
                    ]
        }));
        assert!(groups.iter().any(|group| {
            group.kind == "hard_link"
                && group.items == vec!["source-link.txt".to_string(), "source.txt".to_string()]
                && group
                    .evidence
                    .as_ref()
                    .is_some_and(|evidence| !evidence.full_sha256.is_empty())
        }));

        std::fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}

#[cfg(test)]
mod persistent_index_tests {
    use super::{plan_incremental_refresh, FileRecord, ScanIndex};

    fn record(path: &str, size: u64, modified_unix: u64) -> FileRecord {
        FileRecord {
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            extension: "txt".to_string(),
            size,
            modified_unix,
            kind: "file".to_string(),
        }
    }

    #[test]
    fn incremental_refresh_classifies_added_modified_removed_and_unchanged_files() {
        let previous = ScanIndex {
            root: "/fixture".to_string(),
            scanned_at_unix: 10,
            total_size: 3,
            files: vec![
                record("a.txt", 1, 1),
                record("gone.txt", 1, 1),
                record("same.txt", 1, 1),
            ],
            issues: Vec::new(),
        };
        let current = ScanIndex {
            root: "/fixture".to_string(),
            scanned_at_unix: 20,
            total_size: 4,
            files: vec![
                record("a.txt", 2, 2),
                record("new.txt", 1, 1),
                record("same.txt", 1, 1),
            ],
            issues: Vec::new(),
        };

        let plan = plan_incremental_refresh(&previous, &current)
            .expect("matching roots must produce a refresh plan");

        assert_eq!(plan.added, vec!["new.txt"]);
        assert_eq!(plan.modified, vec!["a.txt"]);
        assert_eq!(plan.removed, vec!["gone.txt"]);
        assert_eq!(plan.unchanged, vec!["same.txt"]);
    }
}

#[cfg(test)]
mod persistent_snapshot_tests {
    use super::{create_persistent_snapshot, FileRecord, ScanIndex, ScanOptions};

    #[test]
    fn persistent_snapshot_binds_index_to_root_and_scan_options() {
        let index = ScanIndex {
            root: "/fixture".to_string(),
            scanned_at_unix: 1,
            total_size: 1,
            files: vec![FileRecord {
                path: "a.txt".to_string(),
                name: "a.txt".to_string(),
                extension: "txt".to_string(),
                size: 1,
                modified_unix: 1,
                kind: "file".to_string(),
            }],
            issues: Vec::new(),
        };
        let snapshot = create_persistent_snapshot(
            index,
            &ScanOptions {
                include_hidden: true,
                max_depth: Some(3),
            },
        );

        assert_eq!(snapshot.schema_version, "1.2.0");
        assert_eq!(snapshot.root, "/fixture");
        assert!(snapshot.include_hidden);
        assert_eq!(snapshot.max_depth, Some(3));
        assert_eq!(snapshot.root_fingerprint.len(), 64);
    }
}

#[cfg(test)]
mod measured_duplicate_default_tests {
    use super::DuplicateOptions;

    #[test]
    fn duplicate_defaults_match_the_repeated_experiment_selection() {
        let defaults = DuplicateOptions::default();

        assert_eq!(defaults.name_threshold, 0.90);
        assert_eq!(defaults.name_ngram_size, 3);
        assert_eq!(defaults.max_ngram_postings, 256);
    }
}
