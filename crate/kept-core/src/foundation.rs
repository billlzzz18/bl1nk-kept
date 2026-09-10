use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FoundationError {
    #[error("Input bytes are not valid UTF-8 at offset {offset}")]
    InvalidEncoding { offset: usize },
    #[error("Regex rule '{rule_id}' is invalid: {message}")]
    InvalidRegexRule { rule_id: String, message: String },
    #[error("Regex rule '{rule_id}' must contain accepted and rejected test vectors")]
    MissingRegexVectors { rule_id: String },
    #[error("Regex rule '{rule_id}' did not match vector '{value}' as expected={accepted}")]
    RegexVectorMismatch {
        rule_id: String,
        value: String,
        accepted: bool,
    },
    #[error("Evidence '{evidence_id}' must be verified before classification")]
    UnverifiedEvidence { evidence_id: String },
    #[error("Evidence '{evidence_id}' has an invalid weight {weight}")]
    InvalidEvidenceWeight { evidence_id: String, weight: String },
    #[error("Classification policy thresholds are invalid")]
    InvalidClassificationPolicy,
    #[error("Glossary term '{term_id}' does not match the active normalization policy")]
    GlossaryNormalizationMismatch { term_id: String },
    #[error("Glossary term '{term_id}' references unknown provenance '{provenance_id}'")]
    MissingProvenance {
        term_id: String,
        provenance_id: String,
    },
    #[error("Provenance '{provenance_id}' is incomplete or has an invalid SHA-256 checksum")]
    InvalidProvenance { provenance_id: String },
    #[error("Glossary term '{term_id}' has invalid confidence {confidence}")]
    InvalidGlossaryConfidence { term_id: String, confidence: String },
    #[error("Corpus manifest entry '{entry_id}' is incomplete, duplicated, or has invalid checksum/split")]
    InvalidCorpusManifestEntry { entry_id: String },
    #[error("Experiment summary requires at least one measurement")]
    EmptyExperimentMeasurements,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegexScope {
    Id,
    Alias,
    Description,
    Path,
    Filename,
    Content,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RegexTestVector {
    pub value: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RegexRule {
    pub id: String,
    pub scope: RegexScope,
    pub pattern: String,
    pub severity: RuleSeverity,
    pub description: String,
    #[serde(rename = "testVectors")]
    pub test_vectors: Vec<RegexTestVector>,
}

// NOTE-001: regex ทุก rule ต้องพิสูจน์ด้วย vector สองทิศทางก่อน validator เรียกใช้
pub fn validate_regex_rule(rule: &RegexRule) -> Result<(), FoundationError> {
    let regex = Regex::new(&rule.pattern).map_err(|error| FoundationError::InvalidRegexRule {
        rule_id: rule.id.clone(),
        message: error.to_string(),
    })?;
    let has_accepted = rule.test_vectors.iter().any(|vector| vector.accepted);
    let has_rejected = rule.test_vectors.iter().any(|vector| !vector.accepted);
    if !has_accepted || !has_rejected {
        return Err(FoundationError::MissingRegexVectors {
            rule_id: rule.id.clone(),
        });
    }
    for vector in &rule.test_vectors {
        if regex.is_match(&vector.value) != vector.accepted {
            return Err(FoundationError::RegexVectorMismatch {
                rule_id: rule.id.clone(),
                value: vector.value.clone(),
                accepted: vector.accepted,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    EncodingValid,
    LexicalValid,
    GlossaryExact,
    ProvenanceComplete,
    PatternRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRecord {
    pub id: String,
    pub kind: EvidenceKind,
    pub weight: f64,
    pub verified: bool,
}

impl EvidenceRecord {
    pub fn verified(id: impl Into<String>, kind: EvidenceKind, weight: f64) -> Self {
        Self {
            id: id.into(),
            kind,
            weight,
            verified: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ClassificationPolicy {
    #[serde(rename = "eligibleThreshold")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub eligible_threshold: f64,
    #[serde(rename = "reviewThreshold")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub review_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DimensionScores {
    #[serde(rename = "encodingQuality")]
    pub encoding_quality: f64,
    #[serde(rename = "lexicalQuality")]
    pub lexical_quality: f64,
    #[serde(rename = "glossarySupport")]
    pub glossary_support: f64,
    #[serde(rename = "provenanceQuality")]
    pub provenance_quality: f64,
    #[serde(rename = "patternRisk")]
    pub pattern_risk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationDecision {
    Reject,
    Review,
    Eligible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassificationResult {
    #[serde(rename = "subjectId")]
    pub subject_id: String,
    #[serde(rename = "dimensionScores")]
    pub dimension_scores: DimensionScores,
    #[serde(rename = "aggregateScore")]
    pub aggregate_score: f64,
    pub decision: ClassificationDecision,
    #[serde(rename = "reasonIds")]
    pub reason_ids: Vec<String>,
    #[serde(rename = "autoAdd")]
    pub auto_add: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ProvenanceRecord {
    pub id: String,
    #[serde(rename = "sourceUri")]
    pub source_uri: String,
    pub license: String,
    #[serde(rename = "contentSha256")]
    #[schemars(regex(pattern = "^[0-9A-Fa-f]{64}$"))]
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GlossaryTerm {
    pub id: String,
    pub term: String,
    pub normalized: String,
    pub language: String,
    pub script: String,
    pub aliases: Vec<String>,
    #[serde(rename = "provenanceId")]
    pub provenance_id: String,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence: f64,
}

// NOTE-002: glossary ต้องมี raw term, normalized term และ source ที่ตรวจ checksum ได้ก่อนเป็น evidence
pub fn validate_glossary_term(
    term: &GlossaryTerm,
    provenance: &[ProvenanceRecord],
) -> Result<(), FoundationError> {
    if !(0.0..=1.0).contains(&term.confidence) {
        return Err(FoundationError::InvalidGlossaryConfidence {
            term_id: term.id.clone(),
            confidence: term.confidence.to_string(),
        });
    }
    if normalize_text(&term.term)? != term.normalized {
        return Err(FoundationError::GlossaryNormalizationMismatch {
            term_id: term.id.clone(),
        });
    }
    let Some(source) = provenance
        .iter()
        .find(|source| source.id == term.provenance_id)
    else {
        return Err(FoundationError::MissingProvenance {
            term_id: term.id.clone(),
            provenance_id: term.provenance_id.clone(),
        });
    };
    if source.source_uri.trim().is_empty()
        || source.license.trim().is_empty()
        || !valid_sha256(&source.content_sha256)
    {
        return Err(FoundationError::InvalidProvenance {
            provenance_id: source.id.clone(),
        });
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Deserialize)]
struct RawImportRow {
    raw_text: Option<String>,
    source_ref: Option<String>,
    observed_at: Option<String>,
    language: Option<String>,
    source_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AnonymizedEvidence {
    #[serde(rename = "rawText")]
    pub raw_text: String,
    #[serde(rename = "normalizedText")]
    pub normalized_text: String,
    #[serde(rename = "sourceRef")]
    pub source_ref: String,
    #[serde(rename = "observedAt")]
    pub observed_at: String,
    pub language: String,
    #[serde(rename = "sourceKind")]
    pub source_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ImportRejection {
    pub line: usize,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub accepted: Vec<AnonymizedEvidence>,
    pub rejected: Vec<ImportRejection>,
}

// NOTE-003: importer เก็บเฉพาะข้อมูลที่ถูก anonymize และคืน rejection รายบรรทัดเพื่อ audit ได้
pub fn import_anonymized_jsonl(input: &str) -> ImportReport {
    let mut report = ImportReport::default();
    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let row: RawImportRow = match serde_json::from_str(line) {
            Ok(row) => row,
            Err(error) => {
                report.rejected.push(ImportRejection {
                    line: line_number,
                    code: "INVALID_JSONL".to_string(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        let (
            Some(raw_text),
            Some(source_ref),
            Some(observed_at),
            Some(language),
            Some(source_kind),
        ) = (
            row.raw_text,
            row.source_ref,
            row.observed_at,
            row.language,
            row.source_kind,
        )
        else {
            report.rejected.push(ImportRejection {
                line: line_number,
                code: "MISSING_REQUIRED_FIELD".to_string(),
                message:
                    "raw_text, source_ref, observed_at, language, and source_kind are required"
                        .to_string(),
            });
            continue;
        };
        if raw_text.trim().is_empty()
            || source_ref.trim().is_empty()
            || observed_at.trim().is_empty()
            || language.trim().is_empty()
            || source_kind.trim().is_empty()
        {
            report.rejected.push(ImportRejection {
                line: line_number,
                code: "EMPTY_REQUIRED_FIELD".to_string(),
                message: "required fields must not be blank".to_string(),
            });
            continue;
        }
        let raw_text = anonymize_text(&raw_text);
        let source_ref = anonymize_text(&source_ref);
        let normalized_text = match normalize_text(&raw_text) {
            Ok(value) => value,
            Err(error) => {
                report.rejected.push(ImportRejection {
                    line: line_number,
                    code: "NORMALIZATION_FAILED".to_string(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        report.accepted.push(AnonymizedEvidence {
            raw_text,
            normalized_text,
            source_ref,
            observed_at,
            language,
            source_kind,
        });
    }
    report
}

// NOTE-008: static regex patterns ที่ compile ครั้งเดียว ใช้ OnceLock แทน expect() เพื่อหลีก clippy lint
// ponytail: use `std::sync::LazyLock` (Rust 1.80+) for one-time compilation per pattern
fn compile_regex(pattern: &str) -> Regex {
    // NOTE-004: regex patterns ถูกต้องแล้ว ใช้ unwrap_or_else เพื่อให้ clippy ผ่าน
    Regex::new(pattern).unwrap_or_else(|_| unreachable!("static regex pattern must be valid"))
}

fn anonymize_text(value: &str) -> String {
    let email = compile_regex(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b");
    let ipv4 = compile_regex(r"\b(?:\d{1,3}\.){3}\d{1,3}\b");
    let phone = compile_regex(r"\b(?:\+66|0)\d{8,9}\b");
    let uuid =
        compile_regex(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b");
    let home_path = compile_regex(r"/home/[^/\s]+");
    let value = email.replace_all(value, "[EMAIL]");
    let value = ipv4.replace_all(&value, "[IP_ADDRESS]");
    let value = phone.replace_all(&value, "[PHONE]");
    let value = uuid.replace_all(&value, "[UUID]");
    home_path.replace_all(&value, "[HOME]").into_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CorpusManifestEntry {
    pub id: String,
    #[serde(rename = "sourceUri")]
    pub source_uri: String,
    pub license: String,
    #[serde(rename = "contentSha256")]
    #[schemars(regex(pattern = "^[0-9A-Fa-f]{64}$"))]
    pub content_sha256: String,
    #[schemars(regex(pattern = "^(build|validation|holdout)$"))]
    pub split: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CorpusManifest {
    pub revision: String,
    pub entries: Vec<CorpusManifestEntry>,
}

// NOTE-005: manifest เป็น gate ก่อนอ่าน corpus เพื่อป้องกันข้อมูลที่ไม่มี provenance เข้า benchmark
pub fn validate_corpus_manifest(manifest: &CorpusManifest) -> Result<(), FoundationError> {
    if manifest.revision.trim().is_empty() {
        return Err(FoundationError::InvalidCorpusManifestEntry {
            entry_id: "<manifest>".to_string(),
        });
    }
    let mut identifiers = std::collections::HashSet::new();
    for entry in &manifest.entries {
        let valid_split = matches!(entry.split.as_str(), "train" | "validation" | "test");
        if entry.id.trim().is_empty()
            || entry.source_uri.trim().is_empty()
            || entry.license.trim().is_empty()
            || !valid_sha256(&entry.content_sha256)
            || !valid_split
            || !identifiers.insert(entry.id.as_str())
        {
            return Err(FoundationError::InvalidCorpusManifestEntry {
                entry_id: entry.id.clone(),
            });
        }
    }
    Ok(())
}

// NOTE-006: corpus จะผ่าน experiment ได้ก็ต่อเมื่อ bytes ตรงกับ checksum ที่ manifest ระบุ
pub fn verify_corpus_bytes(entry: &CorpusManifestEntry, bytes: &[u8]) -> bool {
    let actual = hex::encode(Sha256::digest(bytes));
    actual.eq_ignore_ascii_case(&entry.content_sha256)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RunMeasurement {
    #[serde(rename = "runId")]
    pub run_id: String,
    pub stratum: String,
    #[serde(rename = "latencyMs")]
    pub latency_ms: f64,
    #[serde(rename = "candidateCount")]
    pub candidate_count: u64,
    #[serde(rename = "truePositive")]
    pub true_positive: u64,
    #[serde(rename = "falsePositive")]
    pub false_positive: u64,
    #[serde(rename = "falseNegative")]
    pub false_negative: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DistributionSummary {
    pub count: usize,
    #[serde(rename = "minLatencyMs")]
    pub min_latency_ms: f64,
    #[serde(rename = "maxLatencyMs")]
    pub max_latency_ms: f64,
    #[serde(rename = "meanLatencyMs")]
    pub mean_latency_ms: f64,
    #[serde(rename = "medianLatencyMs")]
    pub median_latency_ms: f64,
    #[serde(rename = "p95LatencyMs")]
    pub p95_latency_ms: f64,
    #[serde(rename = "p99LatencyMs")]
    pub p99_latency_ms: f64,
    #[serde(rename = "stddevLatencyMs")]
    pub stddev_latency_ms: f64,
    #[serde(rename = "meanCandidateCount")]
    pub mean_candidate_count: f64,
    #[serde(rename = "maxFalsePositiveCount")]
    pub max_false_positive_count: u64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

// NOTE-007: summary คำนวณจาก raw runs ทั้งหมดเพื่อแสดง distribution และ metric ที่ตรวจย้อนกลับได้
pub fn summarize_measurements(
    measurements: &[RunMeasurement],
) -> Result<DistributionSummary, FoundationError> {
    if measurements.is_empty() {
        return Err(FoundationError::EmptyExperimentMeasurements);
    }
    let mut latencies = measurements
        .iter()
        .map(|measurement| measurement.latency_ms)
        .collect::<Vec<_>>();
    latencies.sort_by(f64::total_cmp);
    let count = latencies.len();
    let mean_latency_ms = latencies.iter().sum::<f64>() / count as f64;
    let variance = latencies
        .iter()
        .map(|latency| (latency - mean_latency_ms).powi(2))
        .sum::<f64>()
        / count as f64;
    let median_latency_ms = percentile(&latencies, 0.5);
    let p95_latency_ms = percentile(&latencies, 0.95);
    let p99_latency_ms = percentile(&latencies, 0.99);
    let true_positive = measurements
        .iter()
        .map(|value| value.true_positive)
        .sum::<u64>();
    let false_positive = measurements
        .iter()
        .map(|value| value.false_positive)
        .sum::<u64>();
    let false_negative = measurements
        .iter()
        .map(|value| value.false_negative)
        .sum::<u64>();
    let precision = ratio(true_positive, true_positive + false_positive);
    let recall = ratio(true_positive, true_positive + false_negative);
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };

    Ok(DistributionSummary {
        count,
        min_latency_ms: latencies[0],
        max_latency_ms: latencies[count - 1],
        mean_latency_ms,
        median_latency_ms,
        p95_latency_ms,
        p99_latency_ms,
        stddev_latency_ms: variance.sqrt(),
        mean_candidate_count: measurements
            .iter()
            .map(|value| value.candidate_count as f64)
            .sum::<f64>()
            / count as f64,
        max_false_positive_count: measurements
            .iter()
            .map(|value| value.false_positive)
            .max()
            .unwrap_or(0),
        precision,
        recall,
        f1,
    })
}

fn percentile(sorted: &[f64], quantile: f64) -> f64 {
    let rank = (quantile * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1)]
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DefaultCandidate {
    pub id: String,
    pub f1: f64,
    #[serde(rename = "falsePositiveCount")]
    pub false_positive_count: u64,
    #[serde(rename = "p95LatencyMs")]
    pub p95_latency_ms: f64,
    #[serde(rename = "meanCandidateCount")]
    pub mean_candidate_count: f64,
    #[serde(rename = "coveredStrata")]
    pub covered_strata: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SelectionConstraints {
    #[serde(rename = "maxFalsePositiveCount")]
    pub max_false_positive_count: u64,
    #[serde(rename = "maxP95LatencyMs")]
    pub max_p95_latency_ms: f64,
    #[serde(rename = "requiredStrata")]
    pub required_strata: Vec<String>,
}

// NOTE-008: เลือกค่า default จากผลวัดที่ปลอดภัยก่อน แล้วใช้ F1/candidate cost เป็น tie-break ที่ระบุชัด
pub fn select_default_candidate<'a>(
    candidates: &'a [DefaultCandidate],
    constraints: &SelectionConstraints,
) -> Option<&'a DefaultCandidate> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate.false_positive_count <= constraints.max_false_positive_count
                && candidate.p95_latency_ms <= constraints.max_p95_latency_ms
                && constraints.required_strata.iter().all(|stratum| {
                    candidate
                        .covered_strata
                        .iter()
                        .any(|covered| covered == stratum)
                })
        })
        .max_by(|left, right| {
            left.f1
                .total_cmp(&right.f1)
                .then_with(|| {
                    right
                        .mean_candidate_count
                        .total_cmp(&left.mean_candidate_count)
                })
                .then_with(|| right.p95_latency_ms.total_cmp(&left.p95_latency_ms))
                .then_with(|| right.id.cmp(&left.id))
        })
}

// NOTE-009: คะแนนเกิดหลัง evidence ผ่านการตรวจเท่านั้น และ decision ไม่สั่งเพิ่ม keyword อัตโนมัติ
pub fn classify_evidence(
    subject_id: &str,
    evidence: &[EvidenceRecord],
    policy: &ClassificationPolicy,
) -> Result<ClassificationResult, FoundationError> {
    if !(0.0..=1.0).contains(&policy.review_threshold)
        || !(0.0..=1.0).contains(&policy.eligible_threshold)
        || policy.review_threshold > policy.eligible_threshold
    {
        return Err(FoundationError::InvalidClassificationPolicy);
    }

    let mut scores = DimensionScores {
        encoding_quality: 0.0,
        lexical_quality: 0.0,
        glossary_support: 0.0,
        provenance_quality: 0.0,
        pattern_risk: 0.0,
    };
    let mut reason_ids = Vec::with_capacity(evidence.len());
    for record in evidence {
        if !record.verified {
            return Err(FoundationError::UnverifiedEvidence {
                evidence_id: record.id.clone(),
            });
        }
        if !(0.0..=1.0).contains(&record.weight) {
            return Err(FoundationError::InvalidEvidenceWeight {
                evidence_id: record.id.clone(),
                weight: record.weight.to_string(),
            });
        }
        match record.kind {
            EvidenceKind::EncodingValid => {
                scores.encoding_quality = scores.encoding_quality.max(record.weight)
            }
            EvidenceKind::LexicalValid => {
                scores.lexical_quality = scores.lexical_quality.max(record.weight)
            }
            EvidenceKind::GlossaryExact => {
                scores.glossary_support = scores.glossary_support.max(record.weight)
            }
            EvidenceKind::ProvenanceComplete => {
                scores.provenance_quality = scores.provenance_quality.max(record.weight)
            }
            EvidenceKind::PatternRisk => {
                scores.pattern_risk = scores.pattern_risk.max(record.weight)
            }
        }
        reason_ids.push(record.id.clone());
    }
    reason_ids.sort();
    reason_ids.dedup();

    let quality = (scores.encoding_quality
        + scores.lexical_quality
        + scores.glossary_support
        + scores.provenance_quality)
        / 4.0;
    let aggregate_score = quality * (1.0 - scores.pattern_risk);
    let decision = if aggregate_score >= policy.eligible_threshold {
        ClassificationDecision::Eligible
    } else if aggregate_score >= policy.review_threshold {
        ClassificationDecision::Review
    } else {
        ClassificationDecision::Reject
    };

    Ok(ClassificationResult {
        subject_id: subject_id.to_string(),
        dimension_scores: scores,
        aggregate_score,
        decision,
        reason_ids,
        auto_add: false,
    })
}

// NOTE-010: ปฏิเสธ bytes ที่ decode ไม่ได้ก่อนแปลงข้อความเป็น evidence หรือ keyword
pub fn decode_utf8(input: &[u8]) -> Result<&str, FoundationError> {
    std::str::from_utf8(input).map_err(|error| FoundationError::InvalidEncoding {
        offset: error.valid_up_to(),
    })
}

// NOTE-011: normalization ต้อง deterministic เพื่อให้ index, validator และ experiment ใช้ข้อความฐานเดียวกัน
pub fn normalize_text(value: &str) -> Result<String, FoundationError> {
    let nfc: String = value.nfc().collect();
    let collapsed = nfc.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(collapsed.chars().map(lowercase_latin).collect::<String>())
}

// NOTE-012: ลดตัวพิมพ์เฉพาะอักษร Latin เพื่อไม่เปลี่ยน script อื่นโดยไม่ตั้งใจ
fn lowercase_latin(character: char) -> String {
    if is_latin(character) {
        character.to_lowercase().collect()
    } else {
        character.to_string()
    }
}

fn is_latin(character: char) -> bool {
    character.is_ascii_alphabetic()
        || matches!(
            character,
            '\u{00C0}'..='\u{024F}' | '\u{1E00}'..='\u{1EFF}' | '\u{2C60}'..='\u{2C7F}'
        )
}

#[cfg(test)]
mod tests {
    use super::{decode_utf8, FoundationError};

    #[test]
    fn decode_utf8_rejects_invalid_bytes_with_the_failing_offset() {
        assert_eq!(decode_utf8(b"valid").expect("ASCII must decode"), "valid");
        assert_eq!(
            decode_utf8(&[b'a', 0xFF]).expect_err("invalid UTF-8 must be rejected"),
            FoundationError::InvalidEncoding { offset: 1 }
        );
    }
}

#[cfg(test)]
mod regex_rule_tests {
    use super::{validate_regex_rule, RegexRule, RegexScope, RegexTestVector, RuleSeverity};

    #[test]
    fn regex_rule_requires_passing_accept_and_reject_vectors() {
        let valid_rule = RegexRule {
            id: "alias-latin".to_string(),
            scope: RegexScope::Alias,
            pattern: "^[a-z]+$".to_string(),
            severity: RuleSeverity::Error,
            description: "lower-case Latin alias".to_string(),
            test_vectors: vec![
                RegexTestVector {
                    value: "alpha".to_string(),
                    accepted: true,
                },
                RegexTestVector {
                    value: "Alpha".to_string(),
                    accepted: false,
                },
            ],
        };

        assert!(validate_regex_rule(&valid_rule).is_ok());

        let mut invalid_rule = valid_rule;
        invalid_rule.pattern = "(".to_string();
        assert!(validate_regex_rule(&invalid_rule).is_err());
    }
}

#[cfg(test)]
mod classification_tests {
    use super::{
        classify_evidence, ClassificationDecision, ClassificationPolicy, EvidenceKind,
        EvidenceRecord,
    };

    #[test]
    fn classifier_returns_dimension_scores_and_reason_chain_from_verified_evidence() {
        let result = classify_evidence(
            "keyword:รายงาน",
            &[
                EvidenceRecord::verified("encoding-1", EvidenceKind::EncodingValid, 1.0),
                EvidenceRecord::verified("lexical-1", EvidenceKind::LexicalValid, 0.9),
                EvidenceRecord::verified("glossary-1", EvidenceKind::GlossaryExact, 1.0),
                EvidenceRecord::verified("provenance-1", EvidenceKind::ProvenanceComplete, 1.0),
            ],
            &ClassificationPolicy {
                eligible_threshold: 0.90,
                review_threshold: 0.50,
            },
        )
        .expect("verified evidence must classify");

        assert_eq!(result.decision, ClassificationDecision::Eligible);
        assert!(result.aggregate_score > 0.9);
        assert_eq!(
            result.reason_ids,
            vec!["encoding-1", "glossary-1", "lexical-1", "provenance-1"]
        );
        assert!(!result.auto_add);
    }
}

#[cfg(test)]
mod glossary_tests {
    use super::{validate_glossary_term, GlossaryTerm, ProvenanceRecord};

    #[test]
    fn glossary_term_requires_normalized_text_and_known_provenance() {
        let provenance = ProvenanceRecord {
            id: "source-pythainlp-words".to_string(),
            source_uri: "https://example.invalid/source".to_string(),
            license: "Apache-2.0".to_string(),
            content_sha256: "a".repeat(64),
        };
        let term = GlossaryTerm {
            id: "term-report".to_string(),
            term: "  REPORT\tไทย  ".to_string(),
            normalized: "report ไทย".to_string(),
            language: "th-en".to_string(),
            script: "mixed".to_string(),
            aliases: vec!["monthly report".to_string()],
            provenance_id: provenance.id.clone(),
            confidence: 0.95,
        };

        assert!(validate_glossary_term(&term, &[provenance]).is_ok());
    }
}

#[cfg(test)]
mod anonymized_import_tests {
    use super::import_anonymized_jsonl;

    #[test]
    fn importer_masks_sensitive_values_and_reports_rejected_rows() {
        let report = import_anonymized_jsonl(
            concat!(
                "{\"raw_text\":\"ติดต่อ alice@example.com ที่ 127.0.0.1\",\"source_ref\":\"/home/alice/private.txt\",\"observed_at\":\"2026-08-19T00:00:00Z\",\"language\":\"th\",\"source_kind\":\"fixture\"}\n",
                "{\"raw_text\":\"missing required fields\"}\n"
            ),
        );

        assert_eq!(report.accepted.len(), 1);
        assert_eq!(report.rejected.len(), 1);
        assert!(report.accepted[0].raw_text.contains("[EMAIL]"));
        assert!(report.accepted[0].raw_text.contains("[IP_ADDRESS]"));
        assert!(report.accepted[0].source_ref.contains("[HOME]"));
        assert!(!report.accepted[0].raw_text.contains("alice@example.com"));
    }
}

#[cfg(test)]
mod corpus_manifest_tests {
    use super::{validate_corpus_manifest, CorpusManifest, CorpusManifestEntry};

    #[test]
    fn corpus_manifest_requires_complete_unique_and_checksum_verified_entries() {
        let manifest = CorpusManifest {
            revision: "foundation-th-cc0-r1".to_string(),
            entries: vec![CorpusManifestEntry {
                id: "pythainlp-words-th".to_string(),
                source_uri: "https://github.com/PyThaiNLP/pythainlp".to_string(),
                license: "CC0-1.0".to_string(),
                content_sha256: "a".repeat(64),
                split: "validation".to_string(),
            }],
        };

        assert!(validate_corpus_manifest(&manifest).is_ok());
    }
}

#[cfg(test)]
mod corpus_file_integrity_tests {
    use super::{verify_corpus_bytes, CorpusManifest};
    use std::path::PathBuf;

    #[test]
    fn committed_public_corpus_files_match_their_manifest_checksums() {
        let manifest: CorpusManifest = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/corpus/manifest.json"
        )))
        .expect("committed corpus manifest must deserialize");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/corpus/upstream");
        let files = [
            ("pythainlp-words-th", "pythainlp_words_th_cc0.txt"),
            ("pythainlp-stopwords-th", "pythainlp_stopwords_th_cc0.txt"),
        ];

        for (entry_id, file_name) in files {
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.id == entry_id)
                .expect("manifest entry must exist");
            let bytes = std::fs::read(root.join(file_name)).expect("corpus file must exist");
            assert!(verify_corpus_bytes(entry, &bytes));
        }
    }
}

#[cfg(test)]
mod experiment_distribution_tests {
    use super::{summarize_measurements, RunMeasurement};

    #[test]
    fn experiment_summary_keeps_distribution_statistics_for_repeated_runs() {
        let measurements = [1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .enumerate()
            .map(|(run, latency_ms)| RunMeasurement {
                run_id: format!("run-{run}"),
                stratum: "thai_no_space".to_string(),
                latency_ms,
                candidate_count: (run + 1) as u64,
                true_positive: 1,
                false_positive: 0,
                false_negative: 0,
            })
            .collect::<Vec<_>>();

        let summary = summarize_measurements(&measurements)
            .expect("non-empty repeated measurements must summarize");

        assert_eq!(summary.count, 5);
        assert_eq!(summary.min_latency_ms, 1.0);
        assert_eq!(summary.max_latency_ms, 5.0);
        assert_eq!(summary.mean_latency_ms, 3.0);
        assert_eq!(summary.median_latency_ms, 3.0);
        assert_eq!(summary.p95_latency_ms, 5.0);
        assert_eq!(summary.precision, 1.0);
        assert_eq!(summary.recall, 1.0);
    }
}

#[cfg(test)]
mod default_selection_tests {
    use super::{select_default_candidate, DefaultCandidate, SelectionConstraints};

    #[test]
    fn selection_rule_rejects_unsafe_candidate_and_prefers_lower_candidate_cost_on_f1_tie() {
        let constraints = SelectionConstraints {
            max_false_positive_count: 0,
            max_p95_latency_ms: 10.0,
            required_strata: vec!["thai_no_space".to_string(), "latin".to_string()],
        };
        let candidates = vec![
            DefaultCandidate {
                id: "unsafe".to_string(),
                f1: 0.99,
                false_positive_count: 1,
                p95_latency_ms: 1.0,
                mean_candidate_count: 1.0,
                covered_strata: constraints.required_strata.clone(),
            },
            DefaultCandidate {
                id: "expensive".to_string(),
                f1: 0.95,
                false_positive_count: 0,
                p95_latency_ms: 2.0,
                mean_candidate_count: 20.0,
                covered_strata: constraints.required_strata.clone(),
            },
            DefaultCandidate {
                id: r#"selected"#.to_string(),
                f1: 0.95,
                false_positive_count: 0,
                p95_latency_ms: 2.0,
                mean_candidate_count: 5.0,
                covered_strata: constraints.required_strata.clone(),
            },
        ];

        let selected = select_default_candidate(&candidates, &constraints)
            .expect("at least one safe candidate must be selected");

        assert_eq!(selected.id, "selected");
    }
}

#[cfg(test)]
mod experiment_false_positive_distribution_tests {
    use super::{summarize_measurements, RunMeasurement};

    #[test]
    fn summary_reports_per_run_false_positive_extrema_not_a_cross_run_total() {
        let measurements = [0_u64, 2, 1]
            .into_iter()
            .enumerate()
            .map(|(run, false_positive)| RunMeasurement {
                run_id: format!("run-{run}"),
                stratum: "near_name".to_string(),
                latency_ms: 1.0,
                candidate_count: 1,
                true_positive: 1,
                false_positive,
                false_negative: 0,
            })
            .collect::<Vec<_>>();

        let summary = summarize_measurements(&measurements).expect("measurements must summarize");

        assert_eq!(summary.max_false_positive_count, 2);
    }
}
