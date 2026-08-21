use crate::keyboard::{likely_thai_wrong_layout, qwerty_to_thai};
use crate::scanner::normalized_similarity;
use crate::schema::{KeywordRegistry, SearchPolicy, SearchResult, SemanticMetadata};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// NOTE-001: ตัดคำโดยรองรับ Thai bigram เพื่อค้นหาข้อความไทยที่ไม่มีการเว้นวรรคได้
fn tokenize(text: &str) -> Vec<String> {
    let normalized = normalize_query(text);
    let mut tokens = Vec::new();
    for word in normalized.split_whitespace() {
        let chars: Vec<char> = word.chars().collect();
        let is_thai = chars
            .iter()
            .any(|&character| ('\u{0E00}'..='\u{0E7F}').contains(&character));

        if is_thai && chars.len() > 1 {
            for index in 0..chars.len() - 1 {
                tokens.push([chars[index], chars[index + 1]].iter().collect());
            }
        } else if !chars.is_empty() {
            tokens.push(word.to_string());
        }
    }
    tokens
}

/// NOTE-001: สร้าง character n-gram สำหรับลดพื้นที่ที่ต้องเรียก fuzzy matcher
fn character_ngrams(text: &str, gram_size: usize) -> Vec<String> {
    let characters: Vec<char> = normalize_query(text).chars().collect();
    if characters.is_empty() {
        return Vec::new();
    }
    if characters.len() <= gram_size {
        return vec![characters.into_iter().collect()];
    }
    characters
        .windows(gram_size)
        .map(|window| window.iter().collect())
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchIndexStats {
    pub document_count: usize,
    pub term_count: usize,
    pub posting_count: usize,
    pub fuzzy_ngram_count: usize,
}

#[derive(Default)]
struct Bm25Index {
    docs: Vec<Bm25Document>,
    /// NOTE-001: token เก็บหนึ่งครั้งใน dictionary แล้ว document/postings อ้างอิงด้วย u32 ID
    token_ids: HashMap<String, u32>,
    doc_freqs: Vec<usize>,
    /// NOTE-001: term ID -> document indexes ใช้ดึง candidate ก่อนคำนวณ BM25
    postings: Vec<Vec<usize>>,
    /// NOTE-001: n-gram จาก id/alias เท่านั้น ลด fuzzy fallback จาก O(N) เป็น candidate set จำกัด
    fuzzy_postings: HashMap<String, Vec<usize>>,
    avgdl: f64,
}

struct Bm25Document {
    group_index: usize,
    entry_index: usize,
    // NOTE-001: term frequency เป็น integer IDs ลดการเก็บ String/HashMap ซ้ำต่อ document
    token_frequencies: Vec<(u32, u32)>,
    document_length: usize,
}

impl Bm25Index {
    fn build(registry: &KeywordRegistry, policy: &SearchPolicy) -> Self {
        let mut index = Self::default();
        let mut total_length = 0usize;

        for (group_index, group) in registry.groups.iter().enumerate() {
            for (entry_index, entry) in group.entries.iter().enumerate() {
                let Some(id) = entry.get("id").and_then(Value::as_str) else {
                    continue;
                };

                let mut content = String::from(id);
                content.push(' ');
                let mut fuzzy_terms = vec![normalize_query(id)];
                if let Some(aliases) = entry.get("aliases").and_then(Value::as_array) {
                    for alias in aliases.iter().filter_map(Value::as_str) {
                        content.push_str(alias);
                        content.push(' ');
                        fuzzy_terms.push(normalize_query(alias));
                    }
                }
                if let Some(description) = entry.get("description").and_then(Value::as_str) {
                    content.push_str(description);
                    content.push(' ');
                }
                if let Some(semantic) = entry.get("semanticContext") {
                    if let Ok(context) =
                        serde_json::from_value::<SemanticMetadata>(semantic.clone())
                    {
                        for value in [context.root_word, context.definition_th]
                            .into_iter()
                            .flatten()
                        {
                            content.push_str(&value);
                            content.push(' ');
                        }
                    }
                }

                fuzzy_terms.retain(|term| !term.is_empty());
                fuzzy_terms.sort();
                fuzzy_terms.dedup();

                let tokens = tokenize(&content);
                let document_length = tokens.len();
                let mut token_frequencies = HashMap::<u32, u32>::new();
                for token in tokens {
                    let term_id = match index.token_ids.get(&token) {
                        Some(term_id) => *term_id,
                        None => {
                            let term_id = index.doc_freqs.len() as u32;
                            index.token_ids.insert(token, term_id);
                            index.doc_freqs.push(0);
                            index.postings.push(Vec::new());
                            term_id
                        }
                    };
                    *token_frequencies.entry(term_id).or_insert(0) += 1;
                }

                let document_index = index.docs.len();
                let token_frequencies = token_frequencies.into_iter().collect::<Vec<_>>();
                for (term_id, _) in &token_frequencies {
                    index.doc_freqs[*term_id as usize] += 1;
                    index.postings[*term_id as usize].push(document_index);
                }
                for term in &fuzzy_terms {
                    for ngram in character_ngrams(term, policy.fuzzy_ngram_size) {
                        index
                            .fuzzy_postings
                            .entry(ngram)
                            .or_default()
                            .push(document_index);
                    }
                }

                total_length += document_length;
                index.docs.push(Bm25Document {
                    group_index,
                    entry_index,
                    token_frequencies,
                    document_length,
                });
            }
        }

        for postings in index.fuzzy_postings.values_mut() {
            postings.sort_unstable();
            postings.dedup();
        }
        index.avgdl = if index.docs.is_empty() {
            0.0
        } else {
            total_length as f64 / index.docs.len() as f64
        };
        index
    }

    fn candidate_document_ids(&self, tokens: &[String]) -> Vec<usize> {
        let mut candidates = HashSet::new();
        for token in tokens {
            if let Some(term_id) = self.token_ids.get(token) {
                candidates.extend(self.postings[*term_id as usize].iter().copied());
            }
        }
        let mut candidates: Vec<_> = candidates.into_iter().collect();
        candidates.sort_unstable();
        candidates
    }

    fn fuzzy_candidate_document_ids(&self, query: &str, policy: &SearchPolicy) -> Vec<usize> {
        let mut overlap_counts = HashMap::<usize, usize>::new();
        for ngram in character_ngrams(query, policy.fuzzy_ngram_size) {
            if let Some(postings) = self.fuzzy_postings.get(&ngram) {
                // NOTE-001: n-gram ที่พบบ่อยเกินไปไม่ช่วยคัด candidate และทำให้กลับเป็น O(N)
                if postings.len() > policy.max_fuzzy_ngram_postings {
                    continue;
                }
                for document_index in postings {
                    *overlap_counts.entry(*document_index).or_insert(0) += 1;
                }
            }
        }

        let mut candidates: Vec<_> = overlap_counts.into_iter().collect();
        candidates.sort_unstable_by(|(left_document, left_hits), (right_document, right_hits)| {
            right_hits
                .cmp(left_hits)
                .then_with(|| left_document.cmp(right_document))
        });
        candidates
            .into_iter()
            .take(policy.fuzzy_candidate_limit)
            .map(|(document_index, _)| document_index)
            .collect()
    }

    fn score(&self, query_tokens: &[String], document_index: usize) -> f64 {
        if self.avgdl == 0.0 {
            return 0.0;
        }
        let document = &self.docs[document_index];
        let k1 = 1.2;
        let b = 0.75;
        let mut score = 0.0;

        for token in query_tokens {
            let Some(term_id) = self.token_ids.get(token) else {
                continue;
            };
            let document_frequency = self.doc_freqs[*term_id as usize];
            let term_frequency = document
                .token_frequencies
                .iter()
                .find(|(document_term_id, _)| document_term_id == term_id)
                .map(|(_, frequency)| *frequency as f64)
                .unwrap_or(0.0);
            if term_frequency == 0.0 {
                continue;
            }
            let inverse_document_frequency = ((self.docs.len() as f64 - document_frequency as f64
                + 0.5)
                / (document_frequency as f64 + 0.5)
                + 1.0)
                .ln();
            let length_normalizer =
                k1 * (1.0 - b + b * (document.document_length as f64 / self.avgdl));
            score += inverse_document_frequency * (term_frequency * (k1 + 1.0))
                / (term_frequency + length_normalizer);
        }
        score
    }

    fn stats(&self) -> SearchIndexStats {
        SearchIndexStats {
            document_count: self.docs.len(),
            term_count: self.postings.len(),
            posting_count: self.postings.iter().map(Vec::len).sum(),
            fuzzy_ngram_count: self.fuzzy_postings.len(),
        }
    }
}

pub struct KeywordSearch {
    registry: KeywordRegistry,
    bm25: Bm25Index,
    policy: SearchPolicy,
    matcher: SkimMatcherV2,
}

impl KeywordSearch {
    pub fn new(registry: KeywordRegistry) -> Self {
        let policy = registry.search_policy.clone().unwrap_or_default();
        let bm25 = Bm25Index::build(&registry, &policy);
        Self {
            registry,
            bm25,
            policy,
            matcher: SkimMatcherV2::default().smart_case(),
        }
    }

    pub fn index_stats(&self) -> SearchIndexStats {
        self.bm25.stats()
    }

    pub fn search(&self, query: &str, group_id: Option<&str>) -> Vec<SearchResult> {
        let final_query = if likely_thai_wrong_layout(query) {
            qwerty_to_thai(query)
        } else {
            query.to_string()
        };
        let normalized_query = normalize_query(&final_query);
        let query_tokens = tokenize(&final_query);
        let mut scores = HashMap::<usize, (f64, &'static str)>::new();

        self.score_bm25_candidates(&query_tokens, group_id, 1.0, "bm25", &mut scores);
        if let Some(index) = &self.registry.index {
            for (map, match_type) in [
                (&index.synonyms, "synonym"),
                (&index.homographs, "homograph"),
                (&index.homophones, "homophone"),
                (&index.semantic_synonyms, "semantic"),
            ] {
                if let Some(synonyms) = map.get(&normalized_query) {
                    for synonym in synonyms {
                        self.score_bm25_candidates(
                            &tokenize(synonym),
                            group_id,
                            0.8,
                            match_type,
                            &mut scores,
                        );
                    }
                }
            }
        }

        let best_ranked_score = scores
            .values()
            .map(|(score, _)| *score)
            .fold(0.0_f64, f64::max);
        if scores.is_empty() || best_ranked_score < 1.0 {
            for document_index in self
                .bm25
                .fuzzy_candidate_document_ids(&normalized_query, &self.policy)
            {
                let document = &self.bm25.docs[document_index];
                if !self.matches_group(document, group_id) {
                    continue;
                }
                let Some(entry) = self.entry_for_document(document) else {
                    continue;
                };
                let best_fuzzy_score = entry_fuzzy_score(&self.matcher, entry, &normalized_query);
                let fuzzy_similarity = entry_fuzzy_similarity(entry, &normalized_query);
                if best_fuzzy_score > 0 && fuzzy_similarity >= self.policy.fuzzy_min_similarity {
                    upsert_score(
                        &mut scores,
                        document_index,
                        best_fuzzy_score as f64 / 10.0,
                        "fuzzy",
                    );
                }
            }
        }

        let mut results: Vec<_> = scores
            .into_iter()
            .filter_map(|(document_index, (score, match_type))| {
                let document = self.bm25.docs.get(document_index)?;
                let entry = self.entry_for_document(document)?;
                let group_id = &self.registry.groups.get(document.group_index)?.group_id;
                Some(self.create_result(entry, group_id, score, match_type))
            })
            .collect();
        results.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
                .then_with(|| left.group_id.cmp(&right.group_id))
        });
        results.truncate(20);
        results
    }

    fn score_bm25_candidates(
        &self,
        query_tokens: &[String],
        group_id: Option<&str>,
        weight: f64,
        match_type: &'static str,
        scores: &mut HashMap<usize, (f64, &'static str)>,
    ) {
        for document_index in self.bm25.candidate_document_ids(query_tokens) {
            let document = &self.bm25.docs[document_index];
            if !self.matches_group(document, group_id) {
                continue;
            }
            let score = self.bm25.score(query_tokens, document_index) * weight;
            if score > 0.0 {
                upsert_score(scores, document_index, score, match_type);
            }
        }
    }

    fn matches_group(&self, document: &Bm25Document, group_id: Option<&str>) -> bool {
        group_id.is_none_or(|expected_group| {
            self.registry
                .groups
                .get(document.group_index)
                .is_some_and(|group| group.group_id == expected_group)
        })
    }

    fn entry_for_document(&self, document: &Bm25Document) -> Option<&Value> {
        self.registry
            .groups
            .get(document.group_index)?
            .entries
            .get(document.entry_index)
    }

    fn create_result(
        &self,
        entry: &Value,
        group_id: &str,
        score: f64,
        match_type: &str,
    ) -> SearchResult {
        SearchResult {
            id: entry
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            group_id: group_id.to_string(),
            description: entry
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            aliases: entry
                .get("aliases")
                .and_then(Value::as_array)
                .map(|aliases| {
                    aliases
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            match_type: match_type.to_string(),
            score,
            semantic_context: entry
                .get("semanticContext")
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            usage_stats: entry
                .get("usageStats")
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            language_score: None,
            confidence: Some(score.min(10.0) / 10.0),
        }
    }
}

fn entry_fuzzy_score(matcher: &SkimMatcherV2, entry: &Value, query: &str) -> i64 {
    entry_terms(entry)
        .filter_map(|term| matcher.fuzzy_match(&normalize_query(term), query))
        .max()
        .unwrap_or_default()
}

/// NOTE-001: policy ใช้ normalized similarity เพื่อให้ค่า 0.0–1.0 เปรียบเทียบได้ ต่างจาก raw score ของ matcher
fn entry_fuzzy_similarity(entry: &Value, query: &str) -> f64 {
    entry_terms(entry)
        .map(|term| normalized_similarity(&normalize_query(term), query))
        .fold(0.0, f64::max)
}

fn entry_terms(entry: &Value) -> impl Iterator<Item = &str> {
    std::iter::once(entry.get("id").and_then(Value::as_str))
        .chain(
            entry
                .get("aliases")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(Some),
        )
        .flatten()
}

fn upsert_score(
    scores: &mut HashMap<usize, (f64, &'static str)>,
    document_index: usize,
    score: f64,
    match_type: &'static str,
) {
    let entry = scores.entry(document_index).or_insert((score, match_type));
    if score > entry.0 {
        *entry = (score, match_type);
    }
}

pub fn normalize_query(query: &str) -> String {
    let mut value = query.trim().to_lowercase();
    if likely_thai_wrong_layout(&value) {
        value = qwerty_to_thai(&value);
    }
    value
        .chars()
        .filter(|character| {
            !matches!(
                *character,
                '\u{0E31}' | '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}'
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        CustomFieldConfig, KeywordGroup, Metadata, RegistryIndex, SearchPolicy, ValidationConfig,
    };
    use serde_json::json;
    use std::collections::HashMap;

    fn mock_registry() -> KeywordRegistry {
        KeywordRegistry {
            version: "1.1.0".to_string(),
            metadata: Metadata::default(),
            groups: vec![KeywordGroup {
                group_id: "test".to_string(),
                group_name: "Test".to_string(),
                description: "Test".to_string(),
                base_fields_schema: HashMap::new(),
                custom_field_allowed: CustomFieldConfig::default(),
                entries: vec![
                    json!({"id": "hello", "aliases": ["สวัสดี", "hi"], "description": "Greeting"}),
                    json!({"id": "report", "aliases": ["monthly report"], "description": "Business summary"}),
                ],
                group_stats: None,
            }],
            validation: ValidationConfig::default(),
            synonym_sets: Vec::new(),
            index: None,
            search_policy: None,
            foundation: None,
        }
    }

    #[test]
    fn thai_tokenization_creates_bigrams() {
        let tokens = tokenize("สวัสดี");
        assert!(tokens.contains(&"สว".to_string()));
    }

    #[test]
    fn search_returns_thai_bm25_match() {
        let search = KeywordSearch::new(mock_registry());
        let results = search.search("สวัสด", None);
        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("hello")
        );
        assert_eq!(
            results.first().map(|result| result.match_type.as_str()),
            Some("bm25")
        );
    }

    #[test]
    fn fuzzy_search_uses_ngram_candidates_without_full_document_scan() {
        let search = KeywordSearch::new(mock_registry());
        let candidates = search
            .bm25
            .fuzzy_candidate_document_ids("rport", &search.policy);
        assert_eq!(candidates, vec![1]);

        let results = search.search("rport", None);
        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("report")
        );
        assert_eq!(
            results.first().map(|result| result.match_type.as_str()),
            Some("fuzzy")
        );
    }

    #[test]
    fn user_search_policy_can_raise_minimum_fuzzy_similarity() {
        let mut registry = mock_registry();
        registry.search_policy = Some(SearchPolicy {
            fuzzy_min_similarity: 0.99,
            ..SearchPolicy::default()
        });

        let results = KeywordSearch::new(registry).search("rport", None);

        assert!(results.is_empty());
    }

    #[test]
    fn synonym_search_remains_available_through_candidate_retrieval() {
        let mut registry = mock_registry();
        registry.index = Some(RegistryIndex {
            last_indexed: "2026-08-16T00:00:00Z".to_string(),
            synonyms: HashMap::from([("greeting-lookup".to_string(), vec!["สวัสดี".to_string()])]),
            homographs: HashMap::new(),
            homophones: HashMap::new(),
            semantic_synonyms: HashMap::new(),
            fuzzy_map: HashMap::new(),
        });

        let results = KeywordSearch::new(registry).search("greeting-lookup", None);
        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("hello")
        );
        assert_eq!(
            results.first().map(|result| result.match_type.as_str()),
            Some("synonym")
        );
    }

    #[test]
    fn inverted_index_limits_rare_term_candidates() {
        let mut registry = mock_registry();
        let group = registry.groups.first_mut().expect("test group exists");
        for index in 0..1_000 {
            group.entries.push(json!({
                "id": format!("generic-{index}"),
                "aliases": [format!("common alias {index}")],
                "description": "Common content"
            }));
        }
        group.entries.push(json!({
            "id": "needle",
            "aliases": ["rare-indexed-token"],
            "description": "Target"
        }));

        let search = KeywordSearch::new(registry);
        let candidates = search
            .bm25
            .candidate_document_ids(&tokenize("rare-indexed-token"));
        assert_eq!(candidates.len(), 1);
        assert_eq!(search.search("rare-indexed-token", None)[0].id, "needle");
        assert_eq!(search.index_stats().document_count, 1_003);
    }
}
