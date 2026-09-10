//! Schema-driven synchronization utilities for Notion.

pub mod builder;
pub mod diff;
pub mod error;
pub mod id_mapper;
pub mod json_schema;
pub mod schema;

pub use diff::Reconciler;

pub use builder::PropertyBuilder;
pub use error::{Result, SyncError};
pub use id_mapper::IdMapper;
pub use json_schema::JsonSchema;
pub use schema::{DatabaseSchema, PropertySchema};

use crate::ir::UniversalBlock;
use serde::{Deserialize, Serialize};

// NOTE-001: M5 - ChangeSet (Diff/Apply model)
/// ตัวแทนของรายการความเปลี่ยนแปลงที่ต้องนำไปใช้กับแพลตฟอร์มปลายทาง
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSet {
    /// รายการคำสั่งเปลี่ยนแปลงที่เรียงลำดับแล้ว
    pub ops: Vec<Op>,
    /// รหัสอ้างอิงสำหรับการทำ Idempotency
    pub idempotency_key: Option<String>,
}

// NOTE-002: ประเภทของคำสั่งเปลี่ยนแปลง (Atomic Operations)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Op {
    /// เพิ่มบล็อกใหม่
    Insert {
        /// ตำแหน่งที่ต้องการเพิ่ม (ถ้าไม่มีคือต่อท้าย)
        index: Option<usize>,
        /// ข้อมูลบล็อก
        block: UniversalBlock,
    },
    /// แก้ไขบล็อกเดิม
    Update {
        /// รหัสประจำบล็อกของแพลตฟอร์มปลายทาง
        block_id: String,
        /// ข้อมูลบล็อกใหม่
        block: UniversalBlock,
    },
    /// ย้ายบล็อก
    Move {
        /// รหัสประจำบล็อก
        block_id: String,
        /// ตำแหน่งใหม่
        new_index: usize,
    },
    /// ลบบล็อก
    Delete {
        /// รหัสประจำบล็อก
        block_id: String,
    },
}

impl Default for ChangeSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            idempotency_key: None,
        }
    }

    pub fn push(&mut self, op: Op) {
        self.ops.push(op);
    }
}

impl ChangeSet {
    /// สร้าง ChangeSet ใหม่พร้อม Idempotency Key (Stripe-style)
    pub fn with_idempotency(key: impl Into<String>) -> Self {
        Self {
            ops: Vec::new(),
            idempotency_key: Some(key.into()),
        }
    }

    /// ตรวจสอบว่ามีรายการความเปลี่ยนแปลงหรือไม่
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// สรุปจำนวนการเปลี่ยนแปลงแต่ละประเภท
    pub fn summary(&self) -> String {
        let mut inserts = 0;
        let mut updates = 0;
        let mut deletes = 0;
        let mut moves = 0;

        for op in &self.ops {
            match op {
                Op::Insert { .. } => inserts += 1,
                Op::Update { .. } => updates += 1,
                Op::Delete { .. } => deletes += 1,
                Op::Move { .. } => moves += 1,
            }
        }

        format!(
            "Sync Plan: {} inserts, {} updates, {} deletes, {} moves",
            inserts, updates, deletes, moves
        )
    }
}
