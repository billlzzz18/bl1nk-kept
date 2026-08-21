//! Document Filter Pipeline (M3)
//!
//! Provides a way to transform Universal IR documents between Reader and Writer stages.
//! Used for platform-specific downgrading, cleaning, and metadata enrichment.

use crate::converter::ConvertError;
use crate::ir::UniversalDocument;

/// NOTE-001: M3 - Filter Trait
/// อินเทอร์เฟซสำหรับตัวกรองเอกสารที่ทำงานกับ Universal IR โดยตรง
pub trait Filter: Send + Sync {
    /// ชื่อของตัวกรอง (สำหรับการทำ Logging/Debugging)
    fn name(&self) -> &str;

    /// ทำการแปลงข้อมูลในเอกสาร
    fn apply(&self, doc: &mut UniversalDocument) -> Result<(), ConvertError>;
}

/// NOTE-001: M3 - FilterPipeline
/// ระบบรวบรวมและรันตัวกรองตามลำดับที่กำหนด (Middleware Pattern)
#[derive(Default)]
pub struct FilterPipeline {
    filters: Vec<Box<dyn Filter>>,
}

impl FilterPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// เพิ่มตัวกรองเข้าไปใน Pipeline
    pub fn add(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    /// รันตัวกรองทั้งหมดกับเอกสาร
    pub fn run(&self, doc: &mut UniversalDocument) -> Result<(), ConvertError> {
        for filter in &self.filters {
            filter.apply(doc)?;
        }
        Ok(())
    }
}

/// NOTE-001: M3 - ThaiSanitizationFilter
/// ตัวกรองสำหรับทำความสะอาดข้อความภาษาไทย (เช่น ลบช่องว่างส่วนเกิน, ปรับมาตรฐานวรรณยุกต์)
pub struct ThaiSanitizationFilter;

impl Filter for ThaiSanitizationFilter {
    fn name(&self) -> &str {
        "ThaiSanitizationFilter"
    }

    fn apply(&self, doc: &mut UniversalDocument) -> Result<(), ConvertError> {
        for block in &mut doc.blocks {
            self.process_block(block);
        }
        Ok(())
    }
}

impl ThaiSanitizationFilter {
    fn process_block(&self, block: &mut crate::ir::UniversalBlock) {
        use crate::ir::UniversalBlock::*;

        match block {
            Paragraph { content, .. } | Heading { content, .. } => {
                self.process_inline(content);
            }
            BulletList { items, .. } | OrderedList { items, .. } => {
                for item in items {
                    for b in &mut item.content {
                        self.process_block(b);
                    }
                }
            }
            Quote { content, .. } | Callout { content, .. } | Toggle { content, .. } => {
                for b in content {
                    self.process_block(b);
                }
            }
            _ => {}
        }
    }

    fn process_inline(&self, content: &mut Vec<crate::ir::inline::InlineElement>) {
        use crate::ir::inline::InlineElement::*;
        for element in content {
            if let TextRun { content, .. } = element {
                // NOTE-001: ลดช่องว่างซ้ำก่อนส่ง Universal IR ไปยัง converter ปลายทาง
                *content = content.replace("  ", " ");
            }
        }
    }
}

/// NOTE-001: M3 - MarkdownAlertFilter
/// แปลง GitHub Alerts (> [!NOTE]) ให้เป็น IR Callout
pub struct MarkdownAlertFilter;

impl Filter for MarkdownAlertFilter {
    fn name(&self) -> &str {
        "MarkdownAlertFilter"
    }

    fn apply(&self, _doc: &mut UniversalDocument) -> Result<(), ConvertError> {
        // NOTE-001: ยังสงวนตำแหน่งใน pipeline ไว้; alert-to-callout จะทำใน Sprint ของ document conversion
        Ok(())
    }
}
