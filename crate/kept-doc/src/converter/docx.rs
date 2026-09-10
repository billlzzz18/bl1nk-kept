//! DOCX Document Adapter (Microsoft Word)
//!
//! Converts between DOCX binary format and Universal IR.

use crate::converter::{ConvertError, ConverterError, FromPlatform, Reader, ToPlatform, Writer};
use crate::ir::{inline::text, DocumentMetadata, Platform, UniversalBlock, UniversalDocument};
use docx_rs::*;

/// DOCX Document Adapter
pub struct DocxAdapter;

impl DocxAdapter {
    /// Read DOCX bytes and convert into UniversalDocument
    pub fn read_bytes(bytes: &[u8]) -> Result<UniversalDocument, ConvertError> {
        let docx = read_docx(bytes)
            .map_err(|e| ConvertError::InvalidData(format!("Failed to parse DOCX: {:?}", e)))?;

        let mut blocks = Vec::new();

        // Extract paragraphs and children from docx body
        for child in docx.document.children {
            match child {
                DocumentChild::Paragraph(p) => {
                    let mut text_content = String::new();
                    for p_child in p.children {
                        if let ParagraphChild::Run(run) = p_child {
                            for r_child in run.children {
                                if let RunChild::Text(t) = r_child {
                                    text_content.push_str(&t.text);
                                }
                            }
                        }
                    }

                    let trimmed = text_content.trim();
                    if !trimmed.is_empty() {
                        blocks.push(UniversalBlock::Paragraph {
                            content: vec![text(trimmed)],
                            style: None,
                        });
                    }
                }
                DocumentChild::Table(_t) => {
                    // Table support placeholder in docx AST
                }
                _ => {}
            }
        }

        let metadata = DocumentMetadata {
            source_platform: Some(Platform::Docx),
            ..Default::default()
        };

        Ok(UniversalDocument {
            metadata,
            blocks,
            styles: crate::ir::StyleSheet::default(),
        })
    }

    /// Write UniversalDocument into DOCX bytes
    pub fn write_bytes(doc: &UniversalDocument) -> Result<Vec<u8>, ConvertError> {
        let mut docx = Docx::new();

        for block in &doc.blocks {
            match block {
                UniversalBlock::Paragraph { content, .. } => {
                    let mut paragraph = Paragraph::new();
                    for inline in content {
                        if let crate::ir::InlineElement::TextRun { content, style } = inline {
                            let mut run = Run::new().add_text(content);
                            if style.as_ref().and_then(|s| s.bold).unwrap_or(false) {
                                run = run.bold();
                            }
                            if style.as_ref().and_then(|s| s.italic).unwrap_or(false) {
                                run = run.italic();
                            }
                            paragraph = paragraph.add_run(run);
                        }
                    }
                    docx = docx.add_paragraph(paragraph);
                }
                UniversalBlock::Heading { level: _level, content, .. } => {
                    let heading_text = content
                        .iter()
                        .map(|elem| match elem {
                            crate::ir::InlineElement::TextRun { content, .. } => content.as_str(),
                            _ => "",
                        })
                        .collect::<Vec<&str>>()
                        .join("");

                    let paragraph =
                        Paragraph::new().add_run(Run::new().add_text(heading_text).bold());

                    docx = docx.add_paragraph(paragraph);
                }
                _ => {}
            }
        }

        let mut cursor = std::io::Cursor::new(Vec::new());
        docx.build().pack(&mut cursor).map_err(|e| {
            ConvertError::ConversionFailed(format!("Failed to build DOCX: {:?}", e))
        })?;

        Ok(cursor.into_inner())
    }
}

impl Reader for DocxAdapter {
    fn read(&self, input: &[u8]) -> Result<UniversalDocument, ConvertError> {
        DocxAdapter::read_bytes(input)
    }
}

impl Writer for DocxAdapter {
    fn write(&self, doc: &UniversalDocument) -> Result<Vec<u8>, ConvertError> {
        DocxAdapter::write_bytes(doc)
    }
}

impl FromPlatform for DocxAdapter {
    const PLATFORM: Platform = Platform::Docx;
    type Input = Vec<u8>;

    fn from_platform(input: Self::Input) -> Result<UniversalDocument, ConverterError> {
        DocxAdapter::read_bytes(&input).map_err(|e| ConverterError::InvalidData(e.to_string()))
    }
}

impl ToPlatform for DocxAdapter {
    const PLATFORM: Platform = Platform::Docx;
    type Output = Vec<u8>;

    fn from_universal(doc: &UniversalDocument) -> Result<Self::Output, ConverterError> {
        DocxAdapter::write_bytes(doc).map_err(|e| ConverterError::ConversionFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docx_write_and_read() {
        let doc = UniversalDocument {
            metadata: DocumentMetadata::default(),
            blocks: vec![
                UniversalBlock::Heading {
                    level: 1,
                    content: vec![text("Test DOCX Heading")],
                    style: None,
                },
                UniversalBlock::Paragraph {
                    content: vec![text("Test DOCX paragraph body text.")],
                    style: None,
                },
            ],
            styles: crate::ir::StyleSheet::default(),
        };

        let bytes = DocxAdapter::write_bytes(&doc).expect("Failed to write docx bytes");
        assert!(!bytes.is_empty());

        let read_doc = DocxAdapter::read_bytes(&bytes).expect("Failed to read back docx");
        assert!(!read_doc.blocks.is_empty());
    }
}
