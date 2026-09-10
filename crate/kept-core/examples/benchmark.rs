use kept_core::schema::{
    CustomFieldConfig, FoundationProfile, KeywordGroup, KeywordRegistry, Metadata,
    NormalizationProfile, ValidationConfig,
};
use kept_core::{
    find_duplicates_with_stats, DuplicateOptions, FileRecord, KeywordSearch, ScanIndex, Validator,
};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::{Duration, Instant};

const DEFAULT_SCALES: &[usize] = &[10_000, 100_000];
const DEFAULT_QUERY_COUNT: usize = 100;

#[derive(Serialize)]
struct BenchmarkResult {
    objects: usize,
    query_count: usize,
    search_index_build_ms: f64,
    validation_ms: f64,
    exact_search_average_us: f64,
    full_text_search_average_us: f64,
    fuzzy_search_average_us: f64,
    index_document_count: usize,
    index_term_count: usize,
    index_posting_count: usize,
    fuzzy_ngram_count: usize,
    duplicate_detection_ms: f64,
    duplicate_candidate_pairs: usize,
    duplicate_compared_pairs: usize,
    duplicate_total_possible_pairs: usize,
    duplicate_high_frequency_postings_skipped: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let scales = parse_scales()?;
    println!("# bl1nk-kept Search/Duplicate Release Benchmark");
    println!(
        "mode=synthetic-deterministic; data is generated in memory and is not written to disk"
    );

    for scale in scales {
        let result = run_benchmark(scale, DEFAULT_QUERY_COUNT);
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}

fn parse_scales() -> Result<Vec<usize>, Box<dyn Error>> {
    let mut values = env::args().skip(1);
    let mut scales = Vec::new();
    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--objects" => {
                let value = values
                    .next()
                    .ok_or("--objects ต้องมีจำนวน object ที่เป็นจำนวนเต็มบวก")?;
                let count = value.parse::<usize>()?;
                if count == 0 {
                    return Err("--objects ต้องมากกว่า 0".into());
                }
                scales.push(count);
            }
            "--help" | "-h" => {
                println!("Usage: cargo run -p kept-core --example benchmark --release -- [--objects COUNT]...");
                println!(
                    "When no scale is supplied, the benchmark runs 10,000 and 100,000 objects."
                );
                std::process::exit(0);
            }
            _ => return Err(format!("ไม่รู้จัก option '{argument}'").into()),
        }
    }
    if scales.is_empty() {
        Ok(DEFAULT_SCALES.to_vec())
    } else {
        Ok(scales)
    }
}

fn run_benchmark(objects: usize, query_count: usize) -> BenchmarkResult {
    // NOTE-001: สร้าง workload แบบลำดับเวลาเพื่อไม่ให้ registry, search index และ scan index ซ้อนกันในหน่วยความจำ
    let started = Instant::now();
    let _validation = Validator::new(synthetic_registry(objects)).validate_registry();
    let validation = started.elapsed();

    let started = Instant::now();
    let search = KeywordSearch::new(synthetic_registry(objects));
    let search_index_build = started.elapsed();
    let index_stats = search.index_stats();

    let exact_search = average_latency(query_count, |index| {
        let query = format!("keyword-{index:06}");
        let _ = search.search(&query, None);
    });
    let full_text_search = average_latency(query_count, |index| {
        let query = format!("cluster-{}", index % 100);
        let _ = search.search(&query, None);
    });
    let fuzzy_search = average_latency(query_count, |index| {
        let query = format!("keywrd-{index:06}");
        let _ = search.search(&query, None);
    });
    drop(search);

    let scan_index = synthetic_scan_index(objects);
    let started = Instant::now();
    let (_groups, duplicate_stats) = find_duplicates_with_stats(
        &scan_index,
        &DuplicateOptions {
            max_ngram_postings: 128,
            ..DuplicateOptions::default()
        },
    );
    let duplicate_detection = started.elapsed();

    BenchmarkResult {
        objects,
        query_count,
        search_index_build_ms: milliseconds(search_index_build),
        validation_ms: milliseconds(validation),
        exact_search_average_us: microseconds(exact_search),
        full_text_search_average_us: microseconds(full_text_search),
        fuzzy_search_average_us: microseconds(fuzzy_search),
        index_document_count: index_stats.document_count,
        index_term_count: index_stats.term_count,
        index_posting_count: index_stats.posting_count,
        fuzzy_ngram_count: index_stats.fuzzy_ngram_count,
        duplicate_detection_ms: milliseconds(duplicate_detection),
        duplicate_candidate_pairs: duplicate_stats.candidate_pairs,
        duplicate_compared_pairs: duplicate_stats.compared_pairs,
        duplicate_total_possible_pairs: duplicate_stats.total_possible_pairs,
        duplicate_high_frequency_postings_skipped: duplicate_stats.skipped_high_frequency_postings,
    }
}

fn average_latency(iterations: usize, operation: impl Fn(usize)) -> Duration {
    let started = Instant::now();
    for index in 0..iterations {
        operation(index);
    }
    started.elapsed() / iterations as u32
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn microseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0
}

// NOTE-002: fixture deterministic เพื่อเปรียบเทียบ release benchmark คนละรอบได้โดยไม่เก็บไฟล์ benchmark ขนาดใหญ่
fn synthetic_registry(objects: usize) -> KeywordRegistry {
    let entries = (0..objects)
        .map(|index| {
            json!({
                "id": format!("keyword-{index:06}"),
                "aliases": [
                    format!("cluster-{}", index % 100),
                    format!("topic-{index:06}")
                ],
                "description": format!("Deterministic synthetic registry object {index}")
            })
        })
        .collect();
    KeywordRegistry {
        version: "1.2.0".to_string(),
        metadata: Metadata::default(),
        groups: vec![KeywordGroup {
            group_id: "benchmark".to_string(),
            group_name: "Benchmark".to_string(),
            description: "Synthetic deterministic benchmark corpus".to_string(),
            base_fields_schema: HashMap::new(),
            custom_field_allowed: CustomFieldConfig {
                enabled: true,
                ..CustomFieldConfig::default()
            },
            entries,
            group_stats: None,
        }],
        validation: ValidationConfig::default(),
        synonym_sets: Vec::new(),
        index: None,
        search_policy: None,
        foundation: Some(FoundationProfile {
            schema_version: "1.2.0".to_string(),
            normalization: NormalizationProfile::default(),
            glossary_terms: Vec::new(),
            provenance_records: Vec::new(),
            regex_rules: Vec::new(),
            classification_policy: None,
            corpus_manifest: None,
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }),
    }
}

fn synthetic_scan_index(objects: usize) -> ScanIndex {
    let files = (0..objects)
        .map(|index| FileRecord {
            path: format!("archive/batch-{}/object-{index:06}.txt", index / 1_000),
            name: format!("object-{index:06}.txt"),
            extension: "txt".to_string(),
            size: 32_768 + (index % 32) as u64,
            modified_unix: 0,
            kind: "file".to_string(),
            is_binary: None,
            git_status: None,
        })
        .chain([
            FileRecord {
                path: "archive/report-2024.md".to_string(),
                name: "report-2024.md".to_string(),
                extension: "md".to_string(),
                size: 32_768,
                modified_unix: 0,
                kind: "file".to_string(),
                is_binary: None,
                git_status: None,
            },
            FileRecord {
                path: "archive/report-2025.md".to_string(),
                name: "report-2025.md".to_string(),
                extension: "md".to_string(),
                size: 32_769,
                modified_unix: 0,
                kind: "file".to_string(),
                is_binary: None,
                git_status: None,
            },
        ])
        .collect::<Vec<_>>();
    ScanIndex {
        root: "synthetic".to_string(),
        scanned_at_unix: 0,
        total_size: files.iter().map(|file| file.size).sum(),
        files,
        issues: Vec::new(),
    }
}
