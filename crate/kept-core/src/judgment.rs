use thiserror::Error;

use crate::operation::{ExpectedEffect, ObservedState, Policy};

// NOTE-001: Error สำหรับกระบวนการประเมินผลของ AgentJudgment
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JudgmentError {
    #[error("failed to evaluate judgment: {0}")]
    Evaluation(String),
}

/// ผลลัพธ์จากการตัดสินใจของ AI / Agent
#[derive(Debug, Clone, PartialEq)]
pub struct JudgmentResult {
    pub effect: ExpectedEffect,
    pub confidence: f64,
}

/// AI-based decisions สำหรับ non-deterministic cases
pub struct AgentJudgment;

impl AgentJudgment {
    /// ประเมินสถานะและตัดสินใจ
    pub fn evaluate(
        state: &ObservedState,
        _policy: &Policy,
    ) -> Result<JudgmentResult, JudgmentError> {
        let mut effects = Vec::new();

        // Rule 1: ถ้ามี duplicate ให้ลบ
        if !state.duplicates.is_empty() {
            effects.push(ExpectedEffect::DuplicateRemoval {
                groups: state.duplicates.clone(),
            });
        }

        // Rule 2: ถ้ามี naming issues ให้แก้
        if !state.naming_issues.is_empty() {
            effects.push(ExpectedEffect::Rename {
                changes: state
                    .naming_issues
                    .iter()
                    .map(|issue| issue.to_rename_change())
                    .collect(),
            });
        }

        // Rule 3: ถ้าไม่มีปัญหา ให้ NoOp
        if effects.is_empty() {
            return Ok(JudgmentResult {
                effect: ExpectedEffect::NoOp {
                    reason: "No issues found".to_string(),
                },
                confidence: 1.0,
            });
        }

        // Return first effect (or merged)
        if let Some(first_effect) = effects.into_iter().next() {
            Ok(JudgmentResult {
                effect: first_effect,
                confidence: 0.9,
            })
        } else {
            Ok(JudgmentResult {
                effect: ExpectedEffect::NoOp {
                    reason: "No issues found".to_string(),
                },
                confidence: 1.0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::NamingIssueItem;
    use crate::policy::NamingIssueKind;
    use crate::scanner::{DuplicateGroup, ScanIndex};
    use std::path::PathBuf;

    #[test]
    fn judgment_returns_noop_when_empty() {
        let scan_index = ScanIndex {
            root: ".".into(),
            scanned_at_unix: 0,
            total_size: 0,
            files: vec![],
            issues: vec![],
        };
        let state = ObservedState::new(scan_index);
        let policy = Policy::starter();

        let result = AgentJudgment::evaluate(&state, &policy).expect("judgment should succeed");
        assert_eq!(
            result.effect,
            ExpectedEffect::NoOp {
                reason: "No issues found".to_string()
            }
        );
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn judgment_detects_duplicates() {
        let scan_index = ScanIndex {
            root: ".".into(),
            scanned_at_unix: 0,
            total_size: 0,
            files: vec![],
            issues: vec![],
        };
        let group = DuplicateGroup {
            kind: "exact".into(),
            similarity: 1.0,
            items: vec!["file1.txt".into(), "file2.txt".into()],
            evidence: None,
        };
        let state = ObservedState::new(scan_index).with_duplicates(vec![group.clone()]);
        let policy = Policy::starter();

        let result = AgentJudgment::evaluate(&state, &policy).expect("judgment should succeed");
        assert_eq!(result.effect, ExpectedEffect::DuplicateRemoval { groups: vec![group] });
    }

    #[test]
    fn judgment_detects_naming_issues() {
        let scan_index = ScanIndex {
            root: ".".into(),
            scanned_at_unix: 0,
            total_size: 0,
            files: vec![],
            issues: vec![],
        };
        let issue = NamingIssueItem {
            path: PathBuf::from("BadName.txt"),
            proposed_path: Some(PathBuf::from("bad_name.txt")),
            issues: vec![crate::policy::NamingIssue {
                kind: NamingIssueKind::Case,
                message: "uppercase not allowed".into(),
            }],
        };
        let state = ObservedState::new(scan_index).with_naming_issues(vec![issue]);
        let policy = Policy::starter();

        let result = AgentJudgment::evaluate(&state, &policy).expect("judgment should succeed");
        assert!(matches!(result.effect, ExpectedEffect::Rename { .. }));
    }
}
