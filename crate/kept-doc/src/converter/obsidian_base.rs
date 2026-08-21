//! Obsidian Base and Markdown Table Adapter
//!
//! Maps tabular and database-like data between Universal IR and Obsidian Base / Markdown Tables.

use crate::converter::{ConverterError, FromPlatform, ToPlatform};
use crate::ir::{
    inline::text,
    table::{CellAlignment, TableCell, TableRow, TableRowType},
    Platform, UniversalBlock, UniversalDocument,
};

/// Obsidian Base / Markdown Table Adapter
pub struct ObsidianBaseAdapter;

impl ObsidianBaseAdapter {
    /// Parse GFM / Obsidian markdown table into UniversalDocument Table block
    pub fn parse_markdown_table(input: &str) -> Result<UniversalDocument, ConverterError> {
        let lines: Vec<&str> = input
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && l.starts_with('|') && l.ends_with('|'))
            .collect();

        if lines.len() < 2 {
            return Err(ConverterError::InvalidData(
                "A markdown table requires at least a header row and a delimiter row".to_string(),
            ));
        }

        // Parse Header
        let header_cells: Vec<&str> = lines[0]
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();

        // Parse Delimiter / Alignments
        let delimiter_cells: Vec<&str> = lines[1]
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();

        let alignments: Vec<Option<CellAlignment>> = delimiter_cells
            .iter()
            .map(|d| {
                let left = d.starts_with(':');
                let right = d.ends_with(':');
                match (left, right) {
                    (true, true) => Some(CellAlignment::Center),
                    (true, false) => Some(CellAlignment::Left),
                    (false, true) => Some(CellAlignment::Right),
                    (false, false) => None,
                }
            })
            .collect();

        let mut rows = Vec::new();

        // Build Header TableRow
        let header_row = TableRow {
            row_type: TableRowType::Header,
            style: None,
            cells: header_cells
                .iter()
                .enumerate()
                .map(|(i, &h)| TableCell {
                    content: vec![text(h)],
                    colspan: None,
                    rowspan: None,
                    style: None,
                    align: alignments.get(i).cloned().flatten(),
                })
                .collect(),
        };
        rows.push(header_row);

        // Build Body Rows (starting from line 2)
        for line in &lines[2..] {
            let cells: Vec<&str> = line
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim())
                .collect();

            let body_row = TableRow {
                row_type: TableRowType::Body,
                style: None,
                cells: cells
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| TableCell {
                        content: vec![text(c)],
                        colspan: None,
                        rowspan: None,
                        style: None,
                        align: alignments.get(i).cloned().flatten(),
                    })
                    .collect(),
            };
            rows.push(body_row);
        }

        let table_block = UniversalBlock::Table {
            rows,
            header: None,
            style: None,
        };

        Ok(UniversalDocument {
            metadata: crate::ir::DocumentMetadata::default(),
            blocks: vec![table_block],
            styles: crate::ir::StyleSheet::default(),
        })
    }

    /// Render UniversalDocument Table block into GFM / Obsidian Markdown Table
    pub fn render_markdown_table(doc: &UniversalDocument) -> Result<String, ConverterError> {
        let mut output = String::new();

        for block in &doc.blocks {
            if let UniversalBlock::Table { rows, .. } = block {
                if rows.is_empty() {
                    continue;
                }

                // Extract all rows and cells text
                let mut table_data: Vec<Vec<String>> = Vec::new();
                let mut alignments: Vec<Option<CellAlignment>> = Vec::new();

                for row in rows {
                    let mut row_cells = Vec::new();
                    for (i, cell) in row.cells.iter().enumerate() {
                        let text_val = cell
                            .content
                            .iter()
                            .map(|elem| match elem {
                                crate::ir::InlineElement::TextRun { content, .. } => {
                                    content.as_str()
                                }
                                _ => "",
                            })
                            .collect::<Vec<&str>>()
                            .join("");

                        if alignments.len() <= i {
                            alignments.push(cell.align.clone());
                        }
                        row_cells.push(text_val);
                    }
                    table_data.push(row_cells);
                }

                if table_data.is_empty() {
                    continue;
                }

                let col_count = table_data[0].len();
                let mut col_widths = vec![3; col_count];

                for row in &table_data {
                    for (i, cell) in row.iter().enumerate() {
                        if i < col_widths.len() {
                            col_widths[i] = col_widths[i].max(cell.len());
                        }
                    }
                }

                // Header Row
                output.push('|');
                for (i, header) in table_data[0].iter().enumerate() {
                    let width = col_widths.get(i).copied().unwrap_or(header.len());
                    output.push_str(&format!(" {:<width$} |", header, width = width));
                }
                output.push('\n');

                // Delimiter Row
                output.push('|');
                for (i, width) in col_widths.iter().enumerate() {
                    let align = alignments.get(i).cloned().flatten();
                    let d = match align {
                        Some(CellAlignment::Left) => format!(
                            ":{:-<width$}",
                            "",
                            width = (*width).saturating_sub(1).max(2)
                        ),
                        Some(CellAlignment::Center) => format!(
                            ":{:-<width$}:",
                            "",
                            width = (*width).saturating_sub(2).max(1)
                        ),
                        Some(CellAlignment::Right) => format!(
                            "{:-<width$}:",
                            "",
                            width = (*width).saturating_sub(1).max(2)
                        ),
                        None => format!("{:-<width$}", "", width = (*width).max(3)),
                    };
                    output.push_str(&format!(" {} |", d));
                }
                output.push('\n');

                // Body Rows
                for row in &table_data[1..] {
                    output.push('|');
                    for (i, cell) in row.iter().enumerate() {
                        let width = col_widths.get(i).copied().unwrap_or(cell.len());
                        output.push_str(&format!(" {:<width$} |", cell, width = width));
                    }
                    output.push('\n');
                }
            }
        }

        Ok(output)
    }
}

impl FromPlatform for ObsidianBaseAdapter {
    const PLATFORM: Platform = Platform::ObsidianBase;
    type Input = String;

    fn from_platform(input: Self::Input) -> Result<UniversalDocument, ConverterError> {
        Self::parse_markdown_table(&input)
    }
}

impl ToPlatform for ObsidianBaseAdapter {
    const PLATFORM: Platform = Platform::ObsidianBase;
    type Output = String;

    fn to_platform(doc: &UniversalDocument) -> Result<Self::Output, ConverterError> {
        Self::render_markdown_table(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obsidian_table_roundtrip() {
        let input = "| Task | Status | Priority |\n| :--- | :---: | ---: |\n| Setup MCP | Done | High |\n| Add Diff Tool | Done | High |\n";
        let doc = ObsidianBaseAdapter::parse_markdown_table(input).unwrap();
        let rendered = ObsidianBaseAdapter::render_markdown_table(&doc).unwrap();
        assert!(rendered.contains("Setup MCP"));
        assert!(rendered.contains("Add Diff Tool"));
        assert!(rendered.contains("Done"));
    }
}
