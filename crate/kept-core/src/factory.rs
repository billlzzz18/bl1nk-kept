use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::judgment::{AgentJudgment, JudgmentError};
use crate::operation::{ExpectedEffect, ObservedState, OperationContract, Policy};
use crate::scanner::{find_duplicates, scan_directory, DuplicateOptions, ScanOptions};

// NOTE-001: Error ประเภทต่าง ๆ ที่เกิดขึ้นจาก Factory Layer
#[derive(Debug, Error)]
pub enum FactoryError {
    #[error("I/O error during scan: {0}")]
    Io(#[from] std::io::Error),
    #[error("judgment error: {0}")]
    Judgment(#[from] JudgmentError),
    #[error("factory operation failed: {0}")]
    General(String),
}

/// User Intent สำหรับระบุความต้องการในการทำงาน
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserIntent {
    ScanDuplicate { root: PathBuf },
    AutoFix { root: PathBuf },
}

/// แปลง user intent → OperationContract
pub struct OperationFactory;

impl OperationFactory {
    /// สแกนและตรวจจับไฟล์ซ้ำจาก root path
    pub fn detect_duplicates(
        root: &Path,
        _policy: &Policy,
    ) -> Result<Vec<crate::scanner::DuplicateGroup>, FactoryError> {
        let scan_index = scan_directory(root, &ScanOptions::default())?;
        let duplicates = find_duplicates(&scan_index, &DuplicateOptions::default());
        Ok(duplicates)
    }

    /// สร้าง contract จาก user intent
    pub fn from_intent(
        intent: &UserIntent,
        current_state: &ObservedState,
        policy: &Policy,
    ) -> Result<OperationContract, FactoryError> {
        match intent {
            UserIntent::ScanDuplicate { root } => {
                // Deterministic: scan แล้วสร้าง plan
                let groups = Self::detect_duplicates(root, policy)?;
                Ok(OperationContract {
                    observed: current_state.clone(),
                    expected: ExpectedEffect::DuplicateRemoval { groups },
                    policy: policy.clone(),
                })
            }
            UserIntent::AutoFix { .. } => {
                // Agent judgment: ต้องตัดสินใจว่าแก้ไขอะไร
                let judgment = AgentJudgment::evaluate(current_state, policy)?;
                Ok(OperationContract {
                    observed: current_state.clone(),
                    expected: judgment.effect,
                    policy: policy.clone(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::ObservedState;
    use crate::scanner::ScanIndex;

    #[test]
    fn factory_creates_contract_from_autofix() {
        let scan_index = ScanIndex {
            root: ".".into(),
            scanned_at_unix: 0,
            total_size: 0,
            files: vec![],
            issues: vec![],
        };
        let current_state = ObservedState::new(scan_index);
        let policy = Policy::starter();

        let contract = OperationFactory::from_intent(
            &UserIntent::AutoFix { root: PathBuf::from(".") },
            &current_state,
            &policy,
        )
        .expect("contract creation should succeed");

        assert_eq!(
            contract.expected,
            ExpectedEffect::NoOp {
                reason: "No issues found".to_string()
            }
        );
    }
}
