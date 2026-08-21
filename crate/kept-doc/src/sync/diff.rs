use crate::ir::UniversalDocument;
use crate::sync::{ChangeSet, Op, Result};

pub struct Reconciler;

impl Reconciler {
    /// NOTE-001: สร้างแผนเปลี่ยนแปลงตามลำดับบล็อก; remote IDs จริงยังเป็นงานของ Notion safe sync
    pub fn diff(local: &UniversalDocument, remote: &UniversalDocument) -> Result<ChangeSet> {
        let mut changes = ChangeSet::new();
        let shared_length = local.blocks.len().min(remote.blocks.len());

        for index in 0..shared_length {
            if serde_json::to_value(&local.blocks[index])?
                != serde_json::to_value(&remote.blocks[index])?
            {
                changes.push(Op::Update {
                    block_id: format!("block_{index}"),
                    block: local.blocks[index].clone(),
                });
            }
        }

        for (index, block) in local.blocks.iter().enumerate().skip(shared_length) {
            changes.push(Op::Insert {
                index: Some(index),
                block: block.clone(),
            });
        }

        for index in (shared_length..remote.blocks.len()).rev() {
            changes.push(Op::Delete {
                block_id: format!("block_{index}"),
            });
        }

        Ok(changes)
    }
}
