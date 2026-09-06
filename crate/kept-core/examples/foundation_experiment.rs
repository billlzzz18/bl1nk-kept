use kept_core::{
    find_duplicates_with_stats, select_default_candidate, summarize_measurements, DefaultCandidate,
    DuplicateOptions, FileRecord, RunMeasurement, ScanIndex, SelectionConstraints,
};
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const CORPUS_REVISION: &str = "foundation-th-cc0-r1";
const EXACT_STRATUM: &str = "thai_wordlist_exact_name";
const NEAR_STRATUM: &str = "thai_wordlist_near_name";
const REPETITIONS: usize = 7;
const WORD_LIMIT: usize = 10_000;
const NEAR_PAIR_INTERVAL: usize = 137;

#[derive(Debug, Clone)]
struct CandidateConfig {
    id: &'static str,
    name_threshold: f64,
    name_ngram_size: usize,
    max_ngram_postings: usize,
    baseline: bool,
}

#[derive(Serialize)]
struct ExperimentSummary<'a> {
    corpus_revision: &'a str,
    data_scope: &'a str,
    repetitions: usize,
    word_limit: usize,
    baseline_id: &'a str,
    selection_constraints: SelectionConstraints,
    selected: Option<DefaultCandidate>,
    candidates: Vec<DefaultCandidate>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = output_dir()?;
    fs::create_dir_all(&output_dir)?;
    let (scan_index, expected_exact_groups, expected_near_groups) = build_scan_index()?;
    let candidates = candidate_configs();
    let mut raw_measurements = Vec::new();

    for candidate in &candidates {
        for run in 0..REPETITIONS {
            let options = DuplicateOptions {
                name_threshold: candidate.name_threshold,
                include_extension: false,
                size_bucket_bytes: 4 * 1024,
                name_ngram_size: candidate.name_ngram_size,
                max_ngram_postings: candidate.max_ngram_postings,
            };
            let started = Instant::now();
            let (groups, stats) = find_duplicates_with_stats(&scan_index, &options);
            let latency_ms = started.elapsed().as_secs_f64() * 1_000.0;
            let exact_name_groups = count_exact_name_groups(&groups);
            raw_measurements.push((
                candidate.id.to_string(),
                measurement_for_group_kind(
                    candidate.id,
                    run,
                    EXACT_STRATUM,
                    latency_ms,
                    stats.candidate_pairs as u64,
                    exact_name_groups,
                    expected_exact_groups,
                ),
            ));
            let near_name_groups = count_groups_of_kind(&groups, "near_name");
            raw_measurements.push((
                candidate.id.to_string(),
                measurement_for_group_kind(
                    candidate.id,
                    run,
                    NEAR_STRATUM,
                    latency_ms,
                    stats.candidate_pairs as u64,
                    near_name_groups,
                    expected_near_groups,
                ),
            ));
        }
    }

    let raw_path = output_dir.join("foundation_th_cc0_r1_duplicate_raw.jsonl");
    let raw_content = raw_measurements
        .iter()
        .map(|(candidate_id, measurement)| {
            serde_json::json!({
                "candidateId": candidate_id,
                "corpusRevision": CORPUS_REVISION,
                "measurement": measurement,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&raw_path, format!("{raw_content}\n"))?;

    let mut default_candidates = Vec::new();
    let mut baseline_p95 = None;
    let mut baseline_max_false_positive = None;
    for candidate in &candidates {
        let measurements = raw_measurements
            .iter()
            .filter(|(candidate_id, _)| candidate_id == candidate.id)
            .map(|(_, measurement)| measurement.clone())
            .collect::<Vec<_>>();
        let summary = summarize_measurements(&measurements)?;
        if candidate.baseline {
            baseline_p95 = Some(summary.p95_latency_ms);
            baseline_max_false_positive = Some(summary.max_false_positive_count);
        }
        default_candidates.push(DefaultCandidate {
            id: candidate.id.to_string(),
            f1: summary.f1,
            false_positive_count: summary.max_false_positive_count,
            p95_latency_ms: summary.p95_latency_ms,
            mean_candidate_count: summary.mean_candidate_count,
            covered_strata: vec![EXACT_STRATUM.to_string(), NEAR_STRATUM.to_string()],
        });
    }

    let selection_constraints = SelectionConstraints {
        max_false_positive_count: baseline_max_false_positive
            .expect("baseline configuration must exist"),
        max_p95_latency_ms: baseline_p95.expect("baseline configuration must exist"),
        required_strata: vec![EXACT_STRATUM.to_string(), NEAR_STRATUM.to_string()],
    };
    let selected = select_default_candidate(&default_candidates, &selection_constraints).cloned();
    let baseline_id = candidates
        .iter()
        .find(|candidate| candidate.baseline)
        .expect("baseline configuration must exist")
        .id;
    let summary = ExperimentSummary {
        corpus_revision: CORPUS_REVISION,
        data_scope: "Public CC0 Thai word list; filenames and exact duplicate copies are deterministic derivatives for duplicate-engine regression. Results do not claim real-world file-distribution coverage.",
        repetitions: REPETITIONS,
        word_limit: WORD_LIMIT,
        baseline_id,
        selection_constraints,
        selected,
        candidates: default_candidates,
    };
    let summary_path = output_dir.join("foundation_th_cc0_r1_duplicate_summary.json");
    fs::write(&summary_path, serde_json::to_string_pretty(&summary)?)?;

    println!("raw={}", raw_path.display());
    println!("summary={}", summary_path.display());
    Ok(())
}

fn count_exact_name_groups(groups: &[kept_core::scanner::DuplicateGroup]) -> usize {
    count_groups_of_kind(groups, "exact_name")
}

fn count_groups_of_kind(groups: &[kept_core::scanner::DuplicateGroup], kind: &str) -> usize {
    groups.iter().filter(|group| group.kind == kind).count()
}

fn measurement_for_group_kind(
    candidate_id: &str,
    run: usize,
    stratum: &str,
    latency_ms: f64,
    candidate_count: u64,
    observed_groups: usize,
    expected_groups: usize,
) -> RunMeasurement {
    RunMeasurement {
        run_id: format!("{candidate_id}-{stratum}-run-{}", run + 1),
        stratum: stratum.to_string(),
        latency_ms,
        candidate_count,
        true_positive: observed_groups.min(expected_groups) as u64,
        false_positive: observed_groups.saturating_sub(expected_groups) as u64,
        false_negative: expected_groups.saturating_sub(observed_groups) as u64,
    }
}

fn default_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/data/foundation_th_cc0_r1")
}

fn output_dir() -> Result<PathBuf, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (None, None) => Ok(default_output_dir()),
        (Some("--output-dir"), Some(path)) => Ok(PathBuf::from(path)),
        _ => Err("Usage: cargo run -p kept-core --example foundation_experiment --release -- [--output-dir PATH]".into()),
    }
}

fn derived_source_name(word: &str, index: usize) -> String {
    format!("{word}_{index:05}")
}

fn derived_near_name(word: &str, index: usize) -> String {
    format!("{}x", derived_source_name(word, index))
}

fn build_scan_index() -> Result<(ScanIndex, usize, usize), Box<dyn Error>> {
    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/corpus/upstream/pythainlp_words_th_cc0.txt");
    let words = fs::read_to_string(corpus_path)?;
    let mut files = Vec::new();
    let mut expected_exact_groups = 0usize;
    let mut expected_near_groups = 0usize;
    for (index, word) in words
        .lines()
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .take(WORD_LIMIT)
        .enumerate()
    {
        let name = format!("{}.txt", derived_source_name(word, index));
        files.push(FileRecord {
            path: format!("corpus/source/{index:05}/{name}"),
            name: name.clone(),
            extension: "txt".to_string(),
            size: 1_024 + (index % 64) as u64,
            modified_unix: 0,
            kind: "file".to_string(),
            is_binary: None,
            git_status: None,
        });
        if index % 101 == 0 {
            files.push(FileRecord {
                path: format!("corpus/duplicate/{index:05}/{name}"),
                name: name.clone(),
                extension: "txt".to_string(),
                size: 1_024 + (index % 64) as u64,
                modified_unix: 0,
                kind: "file".to_string(),
                is_binary: None,
                git_status: None,
            });
            expected_exact_groups += 1;
        }
        if index % NEAR_PAIR_INTERVAL == 0 {
            let near_name = format!("{}.txt", derived_near_name(word, index));
            files.push(FileRecord {
                path: format!("corpus/near/{index:05}/{near_name}"),
                name: near_name,
                extension: "txt".to_string(),
                size: 1_024 + (index % 64) as u64,
                modified_unix: 0,
                kind: "file".to_string(),
                is_binary: None,
                git_status: None,
            });
            expected_near_groups += 1;
        }
    }
    let total_size = files.iter().map(|file| file.size).sum();
    Ok((
        ScanIndex {
            root: "public-cc0-thai-wordlist".to_string(),
            scanned_at_unix: 0,
            total_size,
            files,
            issues: Vec::new(),
        },
        expected_exact_groups,
        expected_near_groups,
    ))
}

fn candidate_configs() -> Vec<CandidateConfig> {
    vec![
        CandidateConfig {
            id: "baseline-t090-n2-p1024",
            name_threshold: 0.90,
            name_ngram_size: 2,
            max_ngram_postings: 1_024,
            baseline: true,
        },
        CandidateConfig {
            id: "t085-n2-p128",
            name_threshold: 0.85,
            name_ngram_size: 2,
            max_ngram_postings: 128,
            baseline: false,
        },
        CandidateConfig {
            id: "t085-n3-p128",
            name_threshold: 0.85,
            name_ngram_size: 3,
            max_ngram_postings: 128,
            baseline: false,
        },
        CandidateConfig {
            id: "t090-n3-p256",
            name_threshold: 0.90,
            name_ngram_size: 3,
            max_ngram_postings: 256,
            baseline: false,
        },
        CandidateConfig {
            id: "t095-n2-p256",
            name_threshold: 0.95,
            name_ngram_size: 2,
            max_ngram_postings: 256,
            baseline: false,
        },
        CandidateConfig {
            id: "t095-n3-p1024",
            name_threshold: 0.95,
            name_ngram_size: 3,
            max_ngram_postings: 1_024,
            baseline: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::count_exact_name_groups;
    use kept_core::scanner::DuplicateGroup;

    #[test]
    fn ground_truth_counter_counts_exact_name_groups_not_other_duplicate_kinds() {
        let groups = vec![
            DuplicateGroup {
                kind: "exact_name".to_string(),
                similarity: 1.0,
                items: vec!["a".to_string(), "b".to_string()],
                evidence: None,
            },
            DuplicateGroup {
                kind: "near_name".to_string(),
                similarity: 0.9,
                items: vec!["c".to_string(), "d".to_string()],
                evidence: None,
            },
        ];

        assert_eq!(count_exact_name_groups(&groups), 1);
    }
}

#[cfg(test)]
mod near_name_counter_tests {
    use super::count_groups_of_kind;
    use kept_core::scanner::DuplicateGroup;

    #[test]
    fn ground_truth_counter_keeps_exact_and_near_name_strata_separate() {
        let groups = vec![
            DuplicateGroup {
                kind: "exact_name".to_string(),
                similarity: 1.0,
                items: vec!["a".to_string(), "b".to_string()],
                evidence: None,
            },
            DuplicateGroup {
                kind: "near_name".to_string(),
                similarity: 0.9,
                items: vec!["c".to_string(), "d".to_string()],
                evidence: None,
            },
        ];

        assert_eq!(count_groups_of_kind(&groups, "exact_name"), 1);
        assert_eq!(count_groups_of_kind(&groups, "near_name"), 1);
    }
}

#[cfg(test)]
mod workload_shape_tests {
    use super::{derived_near_name, derived_source_name};

    #[test]
    fn near_name_fixture_uses_local_variation_without_a_shared_year_suffix() {
        let source = derived_source_name("ทดสอบ", 137);
        let near = derived_near_name("ทดสอบ", 137);

        assert_ne!(source, near);
        assert!(!source.contains("2024"));
        assert!(!near.contains("2025"));
        assert!(near.starts_with(&source));
    }
}

#[cfg(test)]
mod output_path_tests {
    use super::default_output_dir;
    use std::path::Path;

    #[test]
    fn default_output_directory_is_the_versioned_benchmark_data_path() {
        let output = default_output_dir();

        assert!(
            output.ends_with(Path::new("benchmarks/data/foundation_th_cc0_r1")),
            "unexpected benchmark output path: {}",
            output.display()
        );
    }
}
