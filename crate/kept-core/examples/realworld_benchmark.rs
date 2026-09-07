//! Real-world Benchmark Harness: kept (FFF-backed) vs Ripgrep (rg)
//!
//! Evaluates:
//! 1. Grep latency and match accuracy across real-world workloads on read-only directories
//! 2. Token consumption and Admission Decisions across simulated multi-turn AI context retrieval

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use kept_core::context::{AdmissionDecision, Judge};
use kept_core::scanner::{FffAcquisitionMode, FffScanner};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub target_directory: String,
    pub timestamp_utc: String,
    pub system_metadata: SystemMetadata,
    pub grep_comparisons: Vec<GrepComparisonResult>,
    pub admission_token_evaluations: Vec<AdmissionEvaluationResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetadata {
    pub os: String,
    pub rg_version: String,
    pub target_file_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrepComparisonResult {
    pub query: String,
    pub query_type: String,
    pub rg_matches: usize,
    pub rg_latency_ms: f64,
    pub kept_matches: usize,
    pub kept_latency_ms: f64,
    pub speed_ratio_kept_vs_rg: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdmissionEvaluationResult {
    pub scenario: String,
    pub turn: usize,
    pub target_uri: String,
    pub raw_token_estimate: usize,
    pub emitted_token_estimate: usize,
    pub decision: String,
    pub token_savings_percent: f64,
}

fn estimate_tokens(text: &str) -> usize {
    let len = text.len();
    if len == 0 {
        0
    } else {
        len.div_ceil(4)
    }
}

fn run_rg_search(dir: &Path, pattern: &str) -> (usize, f64) {
    let start = Instant::now();
    let output = Command::new("rg")
        .arg("--no-heading")
        .arg("--line-number")
        .arg("--color=never")
        .arg(pattern)
        .arg(dir)
        .output();

    let duration = start.elapsed().as_secs_f64() * 1000.0;

    let match_count = match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().count()
        }
        Err(_) => 0,
    };

    (match_count, duration)
}

fn run_kept_fff_search(scanner: &mut FffScanner, pattern: &str) -> (usize, f64) {
    let start = Instant::now();
    let mut matches = 0;

    if let Ok((records, _)) = scanner.scan_inventory() {
        for record in records {
            if record.is_binary.unwrap_or(false) {
                continue;
            }
            if let Ok(observation) = scanner.acquire(&record.path, FffAcquisitionMode::View) {
                if let Some(content) = observation.content {
                    for line in content.lines() {
                        if line.contains(pattern) {
                            matches += 1;
                        }
                    }
                }
            }
        }
    }

    let duration = start.elapsed().as_secs_f64() * 1000.0;
    (matches, duration)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let target_str = if args.len() > 1 {
        args[1].clone()
    } else {
        "D:\\01work\\Active\\workspace\\bl1nk-kept".to_string()
    };

    let target_dir = PathBuf::from(&target_str);
    if !target_dir.exists() {
        eprintln!(
            "Error: Target directory does not exist: {}",
            target_dir.display()
        );
        std::process::exit(1);
    }

    println!("============================================================");
    println!("  bl1nk-kept: Real-world Acquisition & Grep Benchmark");
    println!("  Target Directory (Read-Only): {}", target_dir.display());
    println!("============================================================\n");

    let rg_version_output = Command::new("rg").arg("--version").output();
    let rg_version = match rg_version_output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or("unknown")
            .to_string(),
        Err(_) => "rg not found".to_string(),
    };

    let mut scanner = match FffScanner::new(&target_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Failed to initialize FffScanner on {}: {}",
                target_dir.display(),
                e
            );
            std::process::exit(1);
        }
    };

    let (inventory, _) = scanner.scan_inventory().expect("Scan inventory failed");
    let file_count = inventory.len();
    println!("Target indexed files (via FFF): {}", file_count);
    println!("Ripgrep binary: {}\n", rg_version);

    let queries = vec![
        ("Observation", "common identifier"),
        ("fn handle_", "common function prefix"),
        ("struct ResourceState", "type declaration"),
        ("BL1NK_UNIQUE_NONEXISTENT_NEEDLE_XYZ", "miss query"),
    ];

    let mut grep_results = Vec::new();

    println!("--- 1. Grep Latency & Accuracy Benchmark ---");
    println!(
        "{:<28} | {:<12} | {:<12} | {:<12} | {:<12}",
        "Pattern", "rg (ms)", "rg matches", "kept (ms)", "kept matches"
    );
    println!("{}", "-".repeat(84));

    for (query, q_type) in queries {
        let (rg_matches, rg_lat) = run_rg_search(&target_dir, query);
        let (kept_matches, kept_lat) = run_kept_fff_search(&mut scanner, query);

        let ratio = if rg_lat > 0.0 { kept_lat / rg_lat } else { 1.0 };

        println!(
            "{:<28} | {:<12.2} | {:<12} | {:<12.2} | {:<12}",
            query, rg_lat, rg_matches, kept_lat, kept_matches
        );

        grep_results.push(GrepComparisonResult {
            query: query.to_string(),
            query_type: q_type.to_string(),
            rg_matches,
            rg_latency_ms: rg_lat,
            kept_matches,
            kept_latency_ms: kept_lat,
            speed_ratio_kept_vs_rg: ratio,
        });
    }

    println!("\n--- 2. Context Admission & Token Savings Benchmark (Judge Engine) ---");
    println!("Simulating 4 sequential turns of an AI agent requesting workspace documents...");

    let judge = Judge::new();
    let sample_files = vec!["SPEC.md", "plan.md", "README.md"];
    let mut admission_results = Vec::new();

    for rel_path in &sample_files {
        let full_path = target_dir.join(rel_path);
        if !full_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&full_path).unwrap_or_default();
        let raw_tokens = estimate_tokens(&content);

        println!("\nTarget: {} (Raw Tokens: ~{})", rel_path, raw_tokens);
        println!(
            "{:<8} | {:<12} | {:<14} | {:<14} | {:<12}",
            "Turn", "Decision", "Raw Tokens", "Emitted Tokens", "Savings %"
        );
        println!("{}", "-".repeat(70));

        for turn in 1..=4 {
            let obs = scanner
                .acquire(rel_path, FffAcquisitionMode::View)
                .expect("acquire failed");

            let target_uri = obs.source.target.to_string();
            let decision = judge.evaluate(obs);

            let (decision_str, emitted_tokens) = match &decision {
                AdmissionDecision::Pass(boxed) => {
                    let c = boxed.content.as_deref().unwrap_or_default();
                    ("PASS", estimate_tokens(c))
                }
                AdmissionDecision::Reference { token_cost, .. } => ("REFERENCE", *token_cost),
                AdmissionDecision::Delta { token_cost, .. } => ("DELTA", *token_cost),
                AdmissionDecision::Warn { .. } => ("WARN (LOOP)", 20),
                AdmissionDecision::Block { .. } => ("BLOCK", 0),
                _ => ("OTHER", raw_tokens),
            };

            let savings = if raw_tokens > 0 {
                ((raw_tokens as f64 - emitted_tokens as f64) / raw_tokens as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "{:<8} | {:<12} | {:<14} | {:<14} | {:<12.1}%",
                format!("Turn {}", turn),
                decision_str,
                raw_tokens,
                emitted_tokens,
                savings
            );

            admission_results.push(AdmissionEvaluationResult {
                scenario: format!("Multi-turn repeat access on {}", rel_path),
                turn,
                target_uri,
                raw_token_estimate: raw_tokens,
                emitted_token_estimate: emitted_tokens,
                decision: decision_str.to_string(),
                token_savings_percent: savings,
            });
        }
    }

    let now_str = chrono::Utc::now().to_rfc3339();
    let report = BenchmarkReport {
        target_directory: target_dir.to_string_lossy().to_string(),
        timestamp_utc: now_str.clone(),
        system_metadata: SystemMetadata {
            os: std::env::consts::OS.to_string(),
            rg_version,
            target_file_count: file_count,
        },
        grep_comparisons: grep_results,
        admission_token_evaluations: admission_results,
    };

    let out_dir = PathBuf::from("benchmarks/data");
    let _ = fs::create_dir_all(&out_dir);
    let out_file = out_dir.join("realworld_rg_and_judge_benchmark.json");

    let json_text = serde_json::to_string_pretty(&report).expect("Serialize report failed");
    fs::write(&out_file, json_text).expect("Write benchmark report failed");

    println!("\n============================================================");
    println!(
        "Benchmark Complete! Report written to: {}",
        out_file.display()
    );
    println!("============================================================");
}
