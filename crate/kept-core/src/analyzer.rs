use crate::schema::{GroupStats, KeywordRegistry, RegistryIndex};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

pub struct RegistryAnalyzer;

impl RegistryAnalyzer {
    /// NOTE-001: สร้าง index และสถิติจากข้อมูล registry ปัจจุบันโดยไม่แก้ไข entries ต้นทาง
    pub fn build_index_and_stats(registry: &mut KeywordRegistry) {
        let mut synonyms = HashMap::new();
        let mut homographs = HashMap::new();
        let mut homophones = HashMap::new();
        let mut semantic_synonyms = HashMap::new();
        for set in &registry.synonym_sets {
            let term = set.term.to_lowercase();
            if !set.synonyms.is_empty() {
                synonyms.insert(term.clone(), set.synonyms.clone());
            }
            if !set.homographs.is_empty() {
                homographs.insert(term.clone(), set.homographs.clone());
            }
            if !set.homophones.is_empty() {
                homophones.insert(term.clone(), set.homophones.clone());
            }
            if !set.semantic_synonyms.is_empty() {
                semantic_synonyms.insert(term, set.semantic_synonyms.clone());
            }
        }

        let mut fuzzy_map = HashMap::new();
        let mut entry_count = 0usize;
        for group in &mut registry.groups {
            let mut identifiers = HashSet::new();
            let mut duplicates = 0usize;
            for entry in &group.entries {
                entry_count += 1;
                if let Some(id) = entry.get("id").and_then(|value| value.as_str()) {
                    if !identifiers.insert(id.to_lowercase()) {
                        duplicates += 1;
                    }
                    if let Some(aliases) = entry.get("aliases").and_then(|value| value.as_array()) {
                        for alias in aliases.iter().filter_map(|value| value.as_str()) {
                            fuzzy_map.insert(alias.to_lowercase(), id.to_string());
                        }
                    }
                }
            }
            group.group_stats = Some(GroupStats {
                total_weight: group.entries.len() as f64,
                extension_counts: HashMap::new(),
                duplicate_count: duplicates,
            });
        }

        registry.metadata.entry_count = Some(entry_count);
        registry.metadata.last_updated = Utc::now().to_rfc3339();
        registry.index = Some(RegistryIndex {
            last_indexed: Utc::now().to_rfc3339(),
            synonyms,
            homographs,
            homophones,
            semantic_synonyms,
            fuzzy_map,
        });
    }
}
