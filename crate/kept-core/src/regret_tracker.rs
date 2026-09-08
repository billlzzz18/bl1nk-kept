/// Compression Regret Tracker — ported from SQZ regret_tracker.rs
/// Source: D:\01work\Active\references\campbellr\sqz\sqz_engine\src\regret_tracker.rs
///
/// Learns from compression mistakes to improve future decisions per-file.
/// When content is re-read or information is lost, that's a "regret event."
/// The tracker records these and adjusts aggressiveness per content ID.
use std::collections::HashMap;

/// A single regret event.
#[derive(Debug, Clone)]
pub struct RegretEvent {
    pub content_id: String,
    pub kind: RegretKind,
    pub turn: u64,
}

/// Types of compression regret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegretKind {
    /// LLM re-read content served from dedup cache.
    DedupReRead,
    /// Verifier triggered safe-mode fallback (too aggressive).
    VerifierFallback,
    /// LLM asked about content that was compressed away.
    InformationLoss,
}

/// Per-content profile learned from regret events.
#[derive(Debug, Clone)]
pub struct ContentProfile {
    pub regret_count: u32,
    /// 0.0 = safe, 1.0 = aggressive
    pub aggressiveness: f64,
    pub last_access_turn: u64,
}

impl Default for ContentProfile {
    fn default() -> Self {
        Self {
            regret_count: 0,
            aggressiveness: 0.5,
            last_access_turn: 0,
        }
    }
}

/// Tracks regret events and learns per-content compression profiles.
pub struct RegretTracker {
    profiles: HashMap<String, ContentProfile>,
    events: Vec<RegretEvent>,
    decay_rate: f64,
    min_aggressiveness: f64,
}

impl Default for RegretTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RegretTracker {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            events: Vec::new(),
            decay_rate: 0.15,
            min_aggressiveness: 0.1,
        }
    }

    /// Record a regret event and reduce aggressiveness.
    pub fn record_regret(&mut self, event: RegretEvent) {
        let profile = self.profiles.entry(event.content_id.clone()).or_default();
        profile.regret_count += 1;
        profile.last_access_turn = event.turn;

        let penalty = match event.kind {
            RegretKind::DedupReRead => self.decay_rate * 0.5,
            RegretKind::VerifierFallback => self.decay_rate,
            RegretKind::InformationLoss => self.decay_rate * 2.0,
        };

        profile.aggressiveness = (profile.aggressiveness - penalty).max(self.min_aggressiveness);
        self.events.push(event);
    }

    /// Record a successful compression — slowly recover aggressiveness.
    pub fn record_success(&mut self, content_id: &str, turn: u64) {
        let profile = self.profiles.entry(content_id.to_string()).or_default();
        profile.last_access_turn = turn;
        profile.aggressiveness = (profile.aggressiveness + 0.02).min(1.0);
    }

    /// Recommended aggressiveness for a content ID (0.0-1.0).
    pub fn recommended_aggressiveness(&self, content_id: &str) -> f64 {
        self.profiles
            .get(content_id)
            .map(|p| p.aggressiveness)
            .unwrap_or(0.5)
    }

    pub fn get_profile(&self, content_id: &str) -> Option<&ContentProfile> {
        self.profiles.get(content_id)
    }

    pub fn total_regrets(&self) -> usize {
        self.events.len()
    }

    /// Top N most regretted content IDs.
    pub fn most_regretted(&self, top_n: usize) -> Vec<(&str, &ContentProfile)> {
        let mut sorted: Vec<_> = self.profiles.iter().map(|(k, v)| (k.as_str(), v)).collect();
        sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1.regret_count));
        sorted.truncate(top_n);
        sorted
    }

    /// Human-readable regret report.
    pub fn format_report(&self) -> String {
        let mut out = format!("kept regret tracker: {} events\n", self.events.len());
        let top = self.most_regretted(5);
        if top.is_empty() {
            out.push_str("  No regret events recorded.\n");
            return out;
        }
        out.push_str("  Most regretted:\n");
        for (path, profile) in &top {
            out.push_str(&format!(
                "  {} — {} regrets, aggressiveness: {:.2}\n",
                path, profile.regret_count, profile.aggressiveness
            ));
        }
        out
    }

    pub fn reset(&mut self) {
        self.profiles.clear();
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_empty() {
        let tracker = RegretTracker::new();
        assert_eq!(tracker.total_regrets(), 0);
        assert_eq!(tracker.recommended_aggressiveness("any.rs"), 0.5);
    }

    #[test]
    fn regret_reduces_aggressiveness() {
        let mut tracker = RegretTracker::new();
        let initial = tracker.recommended_aggressiveness("auth.rs");
        tracker.record_regret(RegretEvent {
            content_id: "auth.rs".into(),
            kind: RegretKind::VerifierFallback,
            turn: 1,
        });
        assert!(tracker.recommended_aggressiveness("auth.rs") < initial);
    }

    #[test]
    fn multiple_regrets_compound() {
        let mut tracker = RegretTracker::new();
        for i in 0..5 {
            tracker.record_regret(RegretEvent {
                content_id: "config.yaml".into(),
                kind: RegretKind::InformationLoss,
                turn: i,
            });
        }
        assert_eq!(tracker.recommended_aggressiveness("config.yaml"), 0.1);
    }

    #[test]
    fn success_recovers() {
        let mut tracker = RegretTracker::new();
        tracker.record_regret(RegretEvent {
            content_id: "lib.rs".into(),
            kind: RegretKind::DedupReRead,
            turn: 1,
        });
        let after_regret = tracker.recommended_aggressiveness("lib.rs");
        for i in 2..20 {
            tracker.record_success("lib.rs", i);
        }
        assert!(tracker.recommended_aggressiveness("lib.rs") > after_regret);
    }

    #[test]
    fn reset_clears_all() {
        let mut tracker = RegretTracker::new();
        tracker.record_regret(RegretEvent {
            content_id: "x.rs".into(),
            kind: RegretKind::DedupReRead,
            turn: 1,
        });
        tracker.reset();
        assert_eq!(tracker.total_regrets(), 0);
        assert_eq!(tracker.recommended_aggressiveness("x.rs"), 0.5);
    }
}
