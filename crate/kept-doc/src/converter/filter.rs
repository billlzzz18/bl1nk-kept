//! Document Filter Pipeline (M3)
//!
//! Provides a way to transform Universal IR documents between Reader and Writer stages.
//! Used for platform-specific downgrading, cleaning, and metadata enrichment.

use crate::converter::ConvertError;
use crate::ir::UniversalDocument;

// NOTE-001: M3 - Filter Trait
/// อินเทอร์เฟซสำหรับตัวกรองเอกสารที่ทำงานกับ Universal IR โดยตรง
pub trait Filter: Send + Sync {
    /// ชื่อของตัวกรอง (สำหรับการทำ Logging/Debugging)
    fn name(&self) -> &str;

    /// ทำการแปลงข้อมูลในเอกสาร
    fn apply(&self, doc: &mut UniversalDocument) -> Result<(), ConvertError>;
}

// NOTE-002: M3 - FilterPipeline
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

// NOTE-003: M3 - ThaiSanitizationFilter
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
                // NOTE-004: ลดช่องว่างซ้ำก่อนส่ง Universal IR ไปยัง converter ปลายทาง
                *content = content.replace("  ", " ");
            }
        }
    }
}

// NOTE-005: M3 - MarkdownAlertFilter
/// แปลง GitHub Alerts (> [!NOTE], > [!TIP], > [!IMPORTANT], > [!WARNING], > [!CAUTION]) ให้เป็น IR Callout
pub struct MarkdownAlertFilter;

impl Filter for MarkdownAlertFilter {
    fn name(&self) -> &str {
        "MarkdownAlertFilter"
    }

    fn apply(&self, doc: &mut UniversalDocument) -> Result<(), ConvertError> {
        let mut new_blocks = Vec::with_capacity(doc.blocks.len());

        for block in doc.blocks.drain(..) {
            if let crate::ir::UniversalBlock::Quote { mut content, style } = block {
                if let Some((icon, color, cleaned_first_block)) = Self::detect_alert(&content) {
                    if let Some(first) = content.first_mut() {
                        *first = cleaned_first_block;
                    }
                    new_blocks.push(crate::ir::UniversalBlock::Callout {
                        icon: Some(icon.to_string()),
                        color: Some(color.to_string()),
                        content,
                        style,
                    });
                    continue;
                }
                new_blocks.push(crate::ir::UniversalBlock::Quote { content, style });
            } else {
                new_blocks.push(block);
            }
        }

        doc.blocks = new_blocks;
        Ok(())
    }
}

impl MarkdownAlertFilter {
    fn detect_alert(
        content: &[crate::ir::UniversalBlock],
    ) -> Option<(&'static str, &'static str, crate::ir::UniversalBlock)> {
        if let Some(crate::ir::UniversalBlock::Paragraph {
            content: inlines,
            style,
        }) = content.first()
        {
            if let Some(crate::ir::inline::InlineElement::TextRun {
                content: text_str,
                style: text_style,
            }) = inlines.first()
            {
                let trimmed = text_str.trim_start();
                let alerts = [
                    ("[!NOTE]", "💡", "blue"),
                    ("[!TIP]", "🎯", "green"),
                    ("[!IMPORTANT]", "📌", "purple"),
                    ("[!WARNING]", "⚠️", "yellow"),
                    ("[!CAUTION]", "🛑", "red"),
                ];

                for (tag, icon, color) in alerts {
                    if let Some(stripped) = trimmed.strip_prefix(tag) {
                        let remainder = stripped.trim_start();
                        let mut new_inlines = inlines.clone();
                        if remainder.is_empty() && new_inlines.len() > 1 {
                            new_inlines.remove(0);
                        } else {
                            new_inlines[0] = crate::ir::inline::InlineElement::TextRun {
                                content: remainder.to_string(),
                                style: text_style.clone(),
                            };
                        }

                        return Some((
                            icon,
                            color,
                            crate::ir::UniversalBlock::Paragraph {
                                content: new_inlines,
                                style: style.clone(),
                            },
                        ));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{inline::text, UniversalBlock, UniversalDocument};

    #[test]
    fn test_markdown_alert_filter_converts_note_to_callout() {
        let quote = UniversalBlock::Quote {
            content: vec![UniversalBlock::Paragraph {
                content: vec![text("[!NOTE] This is an important note.")],
                style: None,
            }],
            style: None,
        };

        let mut doc = UniversalDocument {
            metadata: Default::default(),
            blocks: vec![quote],
            styles: Default::default(),
        };

        let filter = MarkdownAlertFilter;
        filter.apply(&mut doc).unwrap();

        assert_eq!(doc.blocks.len(), 1);
        if let UniversalBlock::Callout {
            icon,
            color,
            content,
            ..
        } = &doc.blocks[0]
        {
            assert_eq!(icon.as_deref(), Some("💡"));
            assert_eq!(color.as_deref(), Some("blue"));
            assert_eq!(content.len(), 1);
        } else {
            panic!("Expected Callout block, got {:?}", doc.blocks[0]);
        }
    }
}
