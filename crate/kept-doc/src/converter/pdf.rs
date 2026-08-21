//! PDF Inbound Document Adapter
//!
//! Extracts text and document structure from PDF files into Universal IR.

use crate::converter::{ConvertError, ConverterError, FromPlatform, Reader};
use crate::ir::{inline::text, DocumentMetadata, Platform, UniversalBlock, UniversalDocument};

/// PDF Document Adapter
pub struct PdfAdapter;

impl PdfAdapter {
    /// Read PDF bytes and convert to UniversalDocument
    pub fn read_bytes(bytes: &[u8]) -> Result<UniversalDocument, ConvertError> {
        let pdf_doc = lopdf::Document::load_mem(bytes)
            .map_err(|e| ConvertError::InvalidData(format!("Failed to parse PDF: {}", e)))?;

        let pages = pdf_doc.get_pages();
        let page_count = pages.len();
        let mut blocks = Vec::new();

        for (page_num, _) in pages {
            if let Ok(page_text) = pdf_doc.extract_text(&[page_num]) {
                for line in page_text.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Simple heuristic for headings vs paragraphs
                    if let Some(stripped) = trimmed.strip_prefix("# ") {
                        blocks.push(UniversalBlock::Heading {
                            level: 1,
                            content: vec![text(stripped)],
                            style: None,
                        });
                    } else if let Some(stripped) = trimmed.strip_prefix("## ") {
                        blocks.push(UniversalBlock::Heading {
                            level: 2,
                            content: vec![text(stripped)],
                            style: None,
                        });
                    } else {
                        blocks.push(UniversalBlock::Paragraph {
                            content: vec![text(trimmed)],
                            style: None,
                        });
                    }
                }
            }
        }

        let metadata = DocumentMetadata {
            title: Some(format!("Extracted PDF Document ({} pages)", page_count)),
            source_platform: Some(Platform::Pdf),
            ..Default::default()
        };

        Ok(UniversalDocument {
            metadata,
            blocks,
            styles: crate::ir::StyleSheet::default(),
        })
    }
}

impl Reader for PdfAdapter {
    fn read(&self, input: &[u8]) -> Result<UniversalDocument, ConvertError> {
        PdfAdapter::read_bytes(input)
    }
}

impl FromPlatform for PdfAdapter {
    const PLATFORM: Platform = Platform::Pdf;
    type Input = Vec<u8>;

    fn from_platform(input: Self::Input) -> Result<UniversalDocument, ConverterError> {
        PdfAdapter::read_bytes(&input).map_err(|e| ConverterError::InvalidData(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pdf_error_handling() {
        let bad_bytes = b"not a pdf";
        assert!(PdfAdapter::read_bytes(bad_bytes).is_err());
    }
}
