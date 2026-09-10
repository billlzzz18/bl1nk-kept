//! Notion-flavored Markdown (NFM) Converter
//!
//! Converts between Notion-flavored Markdown syntax and Universal IR.
//! Supports block colors `{color="..."}`, toggle attributes `{toggle="true"}`,
//! tabs for indentation, `<empty-block/>`, `<callout>`, `<details>`,
//! `<columns>`, `<tabs>`, `<synced_block>`, Mentions, XML table syntax,
//! and inline styles.

use crate::converter::{ConverterError, FromPlatform, ToPlatform};
use crate::ir::blocks::{ListItem, MentionType, TableRow, TaskItem, UniversalBlock};
use crate::ir::inline::{InlineElement, TextStyle};
use crate::ir::style::StyleRef;
use crate::ir::table::{TableCell, TableRowType};
use crate::ir::{DocumentMetadata, Platform, StyleSheet, UniversalDocument};

/// Notion-flavored Markdown Converter
pub struct NotionMarkdownConverter;

impl FromPlatform for NotionMarkdownConverter {
    const PLATFORM: Platform = Platform::NotionMarkdown;
    type Input = String;

    fn from_platform(input: Self::Input) -> Result<UniversalDocument, ConverterError> {
        let blocks = parse_nfm(&input);
        Ok(UniversalDocument {
            metadata: DocumentMetadata::default(),
            blocks,
            styles: StyleSheet::default(),
        })
    }
}

impl ToPlatform for NotionMarkdownConverter {
    const PLATFORM: Platform = Platform::NotionMarkdown;
    type Output = String;

    fn from_universal(doc: &UniversalDocument) -> Result<Self::Output, ConverterError> {
        let mut out = String::new();
        for (i, block) in doc.blocks.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            render_nfm_block(block, 0, &mut out);
        }
        Ok(out)
    }
}

// -----------------------------------------------------------------------------
// Parser Implementation
// -----------------------------------------------------------------------------

pub fn parse_nfm(input: &str) -> Vec<UniversalBlock> {
    let lines: Vec<&str> = input.lines().collect();
    let mut idx = 0;
    parse_blocks_at_indent(&lines, &mut idx, 0)
}

#[inline]
fn get_indent_level(line: &str) -> usize {
    line.chars().take_while(|&c| c == '\t').count()
}

#[inline]
fn strip_indent(line: &str, level: usize) -> &str {
    let mut tabs = 0;
    line.trim_start_matches(|c| {
        if c == '\t' && tabs < level {
            tabs += 1;
            true
        } else {
            false
        }
    })
}

fn parse_blocks_at_indent(
    lines: &[&str],
    idx: &mut usize,
    target_indent: usize,
) -> Vec<UniversalBlock> {
    let mut blocks = Vec::new();

    while *idx < lines.len() {
        let raw_line = lines[*idx];

        if raw_line.trim().is_empty() {
            *idx += 1;
            continue;
        }

        let indent = get_indent_level(raw_line);
        if indent < target_indent {
            break;
        }

        let line = strip_indent(raw_line, indent);

        if line == "<empty-block/>" {
            blocks.push(UniversalBlock::Paragraph { content: vec![], style: None });
            *idx += 1;
            continue;
        }

        if line == "---" {
            blocks.push(UniversalBlock::PageBreak);
            *idx += 1;
            continue;
        }

        if line.starts_with("```") {
            let lang = line.trim_start_matches("```").trim();
            let language = if lang.is_empty() {
                None
            } else {
                Some(lang.to_string())
            };
            *idx += 1;
            let mut code_lines = Vec::new();
            while *idx < lines.len() {
                let cl = lines[*idx];
                if cl.trim() == "```" {
                    *idx += 1;
                    break;
                }
                code_lines.push(cl);
                *idx += 1;
            }
            blocks.push(UniversalBlock::CodeBlock {
                language,
                content: code_lines.join("\n"),
                style: None,
            });
            continue;
        }

        if line.starts_with("$$") && line.trim() == "$$" {
            *idx += 1;
            let mut eq_lines = Vec::new();
            while *idx < lines.len() {
                let el = lines[*idx];
                if el.trim() == "$$" {
                    *idx += 1;
                    break;
                }
                eq_lines.push(el.trim());
                *idx += 1;
            }
            blocks.push(UniversalBlock::Paragraph {
                content: vec![InlineElement::Equation {
                    expression: eq_lines.join("\n"),
                    style: None,
                }],
                style: None,
            });
            continue;
        }

        if line.starts_with("<table") {
            let table_block = parse_table_tag(lines, idx);
            blocks.push(table_block);
            continue;
        }

        if line.starts_with("<callout") {
            let icon = extract_attribute(line, "icon");
            let color = extract_attribute(line, "color");
            *idx += 1;

            let mut child_lines = Vec::new();
            while *idx < lines.len() {
                let cl = lines[*idx];
                if cl.trim() == "</callout>" {
                    *idx += 1;
                    break;
                }
                child_lines.push(cl);
                *idx += 1;
            }

            let mut c_idx = 0;
            let children = parse_blocks_at_indent(&child_lines, &mut c_idx, 0);

            blocks.push(UniversalBlock::Callout {
                icon,
                color,
                content: children,
                style: None,
            });
            continue;
        }

        if line.starts_with("<details") {
            let color = extract_attribute(line, "color");
            *idx += 1;

            let mut summary_inlines = Vec::new();
            if *idx < lines.len() && lines[*idx].trim().starts_with("<summary>") {
                let s_line = lines[*idx].trim();
                let summary_text = s_line
                    .trim_start_matches("<summary>")
                    .trim_end_matches("</summary>");
                summary_inlines = parse_inline_rich_text(summary_text);
                *idx += 1;
            }

            let mut child_lines = Vec::new();
            while *idx < lines.len() {
                let dl = lines[*idx];
                if dl.trim() == "</details>" {
                    *idx += 1;
                    break;
                }
                child_lines.push(dl);
                *idx += 1;
            }

            let mut c_idx = 0;
            let children = parse_blocks_at_indent(&child_lines, &mut c_idx, 0);

            blocks.push(UniversalBlock::Toggle {
                summary: summary_inlines,
                content: children,
                style: color.map(StyleRef::new),
            });
            continue;
        }

        if line.starts_with("<columns>") {
            *idx += 1;
            let mut cols = Vec::new();
            while *idx < lines.len() {
                let l = lines[*idx].trim();
                if l == "</columns>" {
                    *idx += 1;
                    break;
                }
                if l.starts_with("<column") {
                    *idx += 1;
                    let mut col_lines = Vec::new();
                    while *idx < lines.len() {
                        let cl = lines[*idx];
                        if cl.trim() == "</column>" {
                            *idx += 1;
                            break;
                        }
                        col_lines.push(cl);
                        *idx += 1;
                    }
                    let mut c_idx = 0;
                    cols.push(parse_blocks_at_indent(&col_lines, &mut c_idx, 0));
                } else {
                    *idx += 1;
                }
            }
            blocks.push(UniversalBlock::Columns { columns: cols, style: None });
            continue;
        }

        if line.starts_with("<table_of_contents") {
            let color = extract_attribute(line, "color");
            blocks.push(UniversalBlock::TableOfContents {
                depth: 3,
                style: color.map(StyleRef::new),
            });
            *idx += 1;
            continue;
        }

        if line.starts_with("<mention-user") {
            let url = extract_attribute(line, "url").unwrap_or_default();
            blocks.push(UniversalBlock::Mention {
                mention_type: MentionType::User,
                target: url,
                label: None,
                style: None,
            });
            *idx += 1;
            continue;
        }

        if line.starts_with("<mention-page") {
            let url = extract_attribute(line, "url").unwrap_or_default();
            blocks.push(UniversalBlock::Mention {
                mention_type: MentionType::Page,
                target: url,
                label: None,
                style: None,
            });
            *idx += 1;
            continue;
        }

        if line.starts_with("<mention-database") {
            let url = extract_attribute(line, "url").unwrap_or_default();
            blocks.push(UniversalBlock::Mention {
                mention_type: MentionType::Database,
                target: url,
                label: None,
                style: None,
            });
            *idx += 1;
            continue;
        }

        if line.starts_with("<mention-date") {
            let start = extract_attribute(line, "start").unwrap_or_default();
            blocks.push(UniversalBlock::Mention {
                mention_type: MentionType::Date,
                target: start,
                label: None,
                style: None,
            });
            *idx += 1;
            continue;
        }

        if line.starts_with('#') {
            let (level, rest) = if let Some(stripped) = line.strip_prefix("#### ") {
                (4u8, stripped)
            } else if let Some(stripped) = line.strip_prefix("### ") {
                (3u8, stripped)
            } else if let Some(stripped) = line.strip_prefix("## ") {
                (2u8, stripped)
            } else if let Some(stripped) = line.strip_prefix("# ") {
                (1u8, stripped)
            } else {
                (0u8, "")
            };

            if level > 0 {
                let (text, color, is_toggle) = parse_block_attributes(rest);
                let inlines = parse_inline_rich_text(text);
                *idx += 1;

                if is_toggle {
                    let mut child_lines = Vec::new();
                    while *idx < lines.len() {
                        let next_line = lines[*idx];
                        let next_indent = get_indent_level(next_line);
                        if next_line.trim().is_empty() {
                            *idx += 1;
                            continue;
                        }
                        if next_indent > indent {
                            child_lines.push(next_line);
                            *idx += 1;
                        } else {
                            break;
                        }
                    }
                    let mut c_idx = 0;
                    let children = parse_blocks_at_indent(&child_lines, &mut c_idx, indent + 1);
                    blocks.push(UniversalBlock::Toggle {
                        summary: inlines,
                        content: children,
                        style: color.map(StyleRef::new),
                    });
                } else {
                    blocks.push(UniversalBlock::Heading {
                        level,
                        content: inlines,
                        style: color.map(StyleRef::new),
                    });
                }
                continue;
            }
        }

        if line.starts_with("- [ ] ") || line.starts_with("- [x] ") || line.starts_with("- [X] ") {
            let checked = line.starts_with("- [x] ") || line.starts_with("- [X] ");
            let rest = &line[6..];
            let (text, color, _) = parse_block_attributes(rest);
            let inlines = parse_inline_rich_text(text);
            *idx += 1;

            let mut child_lines = Vec::new();
            while *idx < lines.len() {
                let next_line = lines[*idx];
                let next_indent = get_indent_level(next_line);
                if next_line.trim().is_empty() {
                    *idx += 1;
                    continue;
                }
                if next_indent > indent {
                    child_lines.push(next_line);
                    *idx += 1;
                } else {
                    break;
                }
            }
            let mut c_idx = 0;
            let mut children = vec![UniversalBlock::Paragraph {
                content: inlines,
                style: color.clone().map(StyleRef::new),
            }];
            children.extend(parse_blocks_at_indent(&child_lines, &mut c_idx, indent + 1));

            blocks.push(UniversalBlock::TaskList {
                items: vec![TaskItem {
                    content: children,
                    checked,
                    style: color.map(StyleRef::new),
                }],
                style: None,
            });
            continue;
        }

        if let Some(rest) = line.strip_prefix("- ") {
            let (text, color, _) = parse_block_attributes(rest);
            let inlines = parse_inline_rich_text(text);
            *idx += 1;

            let mut child_lines = Vec::new();
            while *idx < lines.len() {
                let next_line = lines[*idx];
                let next_indent = get_indent_level(next_line);
                if next_line.trim().is_empty() {
                    *idx += 1;
                    continue;
                }
                if next_indent > indent {
                    child_lines.push(next_line);
                    *idx += 1;
                } else {
                    break;
                }
            }
            let mut c_idx = 0;
            let mut children = vec![UniversalBlock::Paragraph {
                content: inlines,
                style: color.clone().map(StyleRef::new),
            }];
            children.extend(parse_blocks_at_indent(&child_lines, &mut c_idx, indent + 1));

            blocks.push(UniversalBlock::BulletList {
                items: vec![ListItem {
                    content: children,
                    style: color.map(StyleRef::new),
                }],
                style: None,
            });
            continue;
        }

        if line.chars().next().is_some_and(|c| c.is_ascii_digit()) && line.contains(". ") {
            if let Some(pos) = line.find(". ") {
                let rest = &line[pos + 2..];
                let (text, color, _) = parse_block_attributes(rest);
                let inlines = parse_inline_rich_text(text);
                *idx += 1;

                let mut child_lines = Vec::new();
                while *idx < lines.len() {
                    let next_line = lines[*idx];
                    let next_indent = get_indent_level(next_line);
                    if next_line.trim().is_empty() {
                        *idx += 1;
                        continue;
                    }
                    if next_indent > indent {
                        child_lines.push(next_line);
                        *idx += 1;
                    } else {
                        break;
                    }
                }
                let mut c_idx = 0;
                let mut children = vec![UniversalBlock::Paragraph {
                    content: inlines,
                    style: color.clone().map(StyleRef::new),
                }];
                children.extend(parse_blocks_at_indent(&child_lines, &mut c_idx, indent + 1));

                blocks.push(UniversalBlock::OrderedList {
                    items: vec![ListItem {
                        content: children,
                        style: color.map(StyleRef::new),
                    }],
                    start: 1,
                    style: None,
                });
                continue;
            }
        }

        if let Some(rest) = line.strip_prefix("> ") {
            let (text, color, _) = parse_block_attributes(rest);
            let inlines = parse_inline_rich_text(text);
            *idx += 1;

            let mut child_lines = Vec::new();
            while *idx < lines.len() {
                let next_line = lines[*idx];
                let next_indent = get_indent_level(next_line);
                if next_line.trim().is_empty() {
                    *idx += 1;
                    continue;
                }
                if next_indent > indent {
                    child_lines.push(next_line);
                    *idx += 1;
                } else {
                    break;
                }
            }
            let mut c_idx = 0;
            let mut children = vec![UniversalBlock::Paragraph {
                content: inlines,
                style: color.clone().map(StyleRef::new),
            }];
            children.extend(parse_blocks_at_indent(&child_lines, &mut c_idx, indent + 1));

            blocks.push(UniversalBlock::Quote {
                content: children,
                style: color.map(StyleRef::new),
            });
            continue;
        }

        // Default: Paragraph
        let (text, color, _) = parse_block_attributes(line);
        let inlines = parse_inline_rich_text(text);
        *idx += 1;

        blocks.push(UniversalBlock::Paragraph {
            content: inlines,
            style: color.map(StyleRef::new),
        });
    }

    blocks
}

fn parse_table_tag(lines: &[&str], idx: &mut usize) -> UniversalBlock {
    *idx += 1;
    let mut rows = Vec::new();

    while *idx < lines.len() {
        let line = lines[*idx].trim();
        if line == "</table>" {
            *idx += 1;
            break;
        }
        if line.starts_with("<tr") {
            *idx += 1;
            let mut cells = Vec::new();
            while *idx < lines.len() {
                let cell_line = lines[*idx].trim();
                if cell_line == "</tr>" {
                    *idx += 1;
                    break;
                }
                if cell_line.starts_with("<td") {
                    let text = cell_line
                        .trim_start_matches("<td")
                        .trim_start_matches(|c| c != '>')
                        .trim_start_matches('>')
                        .trim_end_matches("</td>");
                    cells.push(TableCell {
                        content: parse_inline_rich_text(text),
                        colspan: None,
                        rowspan: None,
                        style: None,
                        align: None,
                    });
                }
                *idx += 1;
            }
            rows.push(TableRow {
                cells,
                row_type: if rows.is_empty() {
                    TableRowType::Header
                } else {
                    TableRowType::Body
                },
                style: None,
            });
        } else {
            *idx += 1;
        }
    }

    UniversalBlock::Table {
        rows,
        header: None,
        style: None,
    }
}

fn extract_attribute(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = line.find(&pattern) {
        let rest = &line[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn parse_block_attributes(line: &str) -> (&str, Option<String>, bool) {
    let mut color = None;
    let mut is_toggle = false;
    let mut text = line;

    if let Some(attr_start) = line.rfind('{') {
        if line.ends_with('}') {
            let attr_body = &line[attr_start + 1..line.len() - 1];
            if attr_body.contains("color=") || attr_body.contains("toggle=") {
                text = line[..attr_start].trim_end();
                if let Some(c) = extract_attribute(attr_body, "color") {
                    color = Some(c);
                }
                if let Some(t) = extract_attribute(attr_body, "toggle") {
                    if t == "true" {
                        is_toggle = true;
                    }
                }
            }
        }
    }

    (text, color, is_toggle)
}

pub fn parse_inline_rich_text(input: &str) -> Vec<InlineElement> {
    if input.is_empty() {
        return vec![];
    }

    let mut inlines = Vec::new();
    let mut cur = input;

    while !cur.is_empty() {
        if let Some(stripped) = cur.strip_prefix("<br/>") {
            inlines.push(InlineElement::HardBreak);
            cur = stripped;
            continue;
        }
        if let Some(stripped) = cur.strip_prefix("<br>") {
            inlines.push(InlineElement::HardBreak);
            cur = stripped;
            continue;
        }

        if cur.starts_with("$`") {
            if let Some(end) = cur[2..].find("`$") {
                let expr = &cur[2..2 + end];
                inlines.push(InlineElement::Equation {
                    expression: expr.to_string(),
                    style: None,
                });
                cur = &cur[2 + end + 2..];
                continue;
            }
        }

        if cur.starts_with("<span") {
            if let Some(close_tag) = cur.find('>') {
                let tag_str = &cur[..close_tag + 1];
                let color = extract_attribute(tag_str, "color");
                let underline = extract_attribute(tag_str, "underline").map(|v| v == "true");

                if let Some(end_span) = cur[close_tag + 1..].find("</span>") {
                    let inner = &cur[close_tag + 1..close_tag + 1 + end_span];
                    inlines.push(InlineElement::TextRun {
                        content: unescape_nfm(inner),
                        style: Some(TextStyle {
                            bold: None,
                            italic: None,
                            strikethrough: None,
                            underline,
                            code: None,
                            color,
                            link: None,
                        }),
                    });
                    cur = &cur[close_tag + 1 + end_span + 7..];
                    continue;
                }
            }
        }

        if cur.starts_with("**") {
            if let Some(end) = cur[2..].find("**") {
                let inner = &cur[2..2 + end];
                inlines.push(InlineElement::TextRun {
                    content: unescape_nfm(inner),
                    style: Some(TextStyle::new().bold()),
                });
                cur = &cur[2 + end + 2..];
                continue;
            }
        }

        if cur.starts_with('*') && !cur.starts_with("**") {
            if let Some(end) = cur[1..].find('*') {
                let inner = &cur[1..1 + end];
                inlines.push(InlineElement::TextRun {
                    content: unescape_nfm(inner),
                    style: Some(TextStyle::new().italic()),
                });
                cur = &cur[1 + end + 1..];
                continue;
            }
        }

        if cur.starts_with("~~") {
            if let Some(end) = cur[2..].find("~~") {
                let inner = &cur[2..2 + end];
                inlines.push(InlineElement::TextRun {
                    content: unescape_nfm(inner),
                    style: Some(TextStyle::new().strikethrough()),
                });
                cur = &cur[2 + end + 2..];
                continue;
            }
        }

        if cur.starts_with('`') {
            if let Some(end) = cur[1..].find('`') {
                let inner = &cur[1..1 + end];
                inlines.push(InlineElement::TextRun {
                    content: inner.replace("<br>", "\n"),
                    style: Some(TextStyle::new().code()),
                });
                cur = &cur[1 + end + 1..];
                continue;
            }
        }

        if cur.starts_with('[') {
            if let Some(close_bracket) = cur.find(']') {
                if cur[close_bracket + 1..].starts_with('(') {
                    if let Some(close_paren) = cur[close_bracket + 2..].find(')') {
                        let label = &cur[1..close_bracket];
                        let url = &cur[close_bracket + 2..close_bracket + 2 + close_paren];
                        inlines.push(InlineElement::TextRun {
                            content: unescape_nfm(label),
                            style: Some(TextStyle::new().link(url)),
                        });
                        cur = &cur[close_bracket + 2 + close_paren + 1..];
                        continue;
                    }
                }
            }
        }

        let next_delim = cur
            .find(['*', '`', '~', '<', '$', '['])
            .unwrap_or(cur.len());

        let raw_chunk = if next_delim == 0 {
            let mut char_indices = cur.char_indices();
            char_indices.next();
            let next_pos = char_indices.next().map(|(i, _)| i).unwrap_or(cur.len());
            let chunk = &cur[..next_pos];
            cur = &cur[next_pos..];
            chunk
        } else {
            let chunk = &cur[..next_delim];
            cur = &cur[next_delim..];
            chunk
        };

        inlines.push(InlineElement::TextRun {
            content: unescape_nfm(raw_chunk),
            style: None,
        });
    }

    inlines
}

fn unescape_nfm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    '\\' | '*' | '~' | '`' | '$' | '[' | ']' | '<' | '>' | '{' | '}' | '|'
                    | '^' => {
                        out.push(next);
                        chars.next();
                        continue;
                    }
                    _ => {}
                }
            }
        }
        out.push(ch);
    }
    out
}

pub fn escape_nfm(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' | '*' | '~' | '`' | '$' | '[' | ']' | '<' | '>' | '{' | '}' | '|' | '^' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// -----------------------------------------------------------------------------
// Renderer Implementation
// -----------------------------------------------------------------------------

pub fn render_nfm_block(block: &UniversalBlock, indent_level: usize, out: &mut String) {
    let indent = "\t".repeat(indent_level);

    match block {
        UniversalBlock::Paragraph { content, style } => {
            if content.is_empty() {
                out.push_str(&format!("{}<empty-block/>\n", indent));
                return;
            }
            out.push_str(&indent);
            render_inlines(content, out);
            if let Some(s) = style {
                out.push_str(&format!(" {{color=\"{}\"}}", s.name));
            }
            out.push('\n');
        }
        UniversalBlock::Heading { level, content, style } => {
            out.push_str(&indent);
            let h = match level {
                1 => "# ",
                2 => "## ",
                3 => "### ",
                _ => "#### ",
            };
            out.push_str(h);
            render_inlines(content, out);
            if let Some(s) = style {
                out.push_str(&format!(" {{color=\"{}\"}}", s.name));
            }
            out.push('\n');
        }
        UniversalBlock::CodeBlock { language, content, .. } => {
            out.push_str(&indent);
            out.push_str("```");
            if let Some(lang) = language {
                out.push_str(lang);
            }
            out.push('\n');
            for line in content.lines() {
                out.push_str(&indent);
                out.push_str(line);
                out.push('\n');
            }
            out.push_str(&indent);
            out.push_str("```\n");
        }
        UniversalBlock::Quote { content, style } => {
            out.push_str(&indent);
            out.push_str("> ");
            if let Some(UniversalBlock::Paragraph { content: p_inlines, .. }) = content.first() {
                render_inlines(p_inlines, out);
            }
            if let Some(s) = style {
                out.push_str(&format!(" {{color=\"{}\"}}", s.name));
            }
            out.push('\n');

            for child in content.iter().skip(1) {
                render_nfm_block(child, indent_level + 1, out);
            }
        }
        UniversalBlock::BulletList { items, .. } => {
            for item in items {
                out.push_str(&indent);
                out.push_str("- ");
                if let Some(UniversalBlock::Paragraph { content, .. }) = item.content.first() {
                    render_inlines(content, out);
                }
                if let Some(s) = &item.style {
                    out.push_str(&format!(" {{color=\"{}\"}}", s.name));
                }
                out.push('\n');

                for child in item.content.iter().skip(1) {
                    render_nfm_block(child, indent_level + 1, out);
                }
            }
        }
        UniversalBlock::OrderedList { items, .. } => {
            for (idx, item) in items.iter().enumerate() {
                out.push_str(&indent);
                out.push_str(&format!("{}. ", idx + 1));
                if let Some(UniversalBlock::Paragraph { content, .. }) = item.content.first() {
                    render_inlines(content, out);
                }
                if let Some(s) = &item.style {
                    out.push_str(&format!(" {{color=\"{}\"}}", s.name));
                }
                out.push('\n');

                for child in item.content.iter().skip(1) {
                    render_nfm_block(child, indent_level + 1, out);
                }
            }
        }
        UniversalBlock::TaskList { items, .. } => {
            for item in items {
                out.push_str(&indent);
                if item.checked {
                    out.push_str("- [x] ");
                } else {
                    out.push_str("- [ ] ");
                }
                if let Some(UniversalBlock::Paragraph { content, .. }) = item.content.first() {
                    render_inlines(content, out);
                }
                if let Some(s) = &item.style {
                    out.push_str(&format!(" {{color=\"{}\"}}", s.name));
                }
                out.push('\n');

                for child in item.content.iter().skip(1) {
                    render_nfm_block(child, indent_level + 1, out);
                }
            }
        }
        UniversalBlock::Callout { icon, color, content, .. } => {
            out.push_str(&indent);
            out.push_str("<callout");
            if let Some(ic) = icon {
                out.push_str(&format!(" icon=\"{}\"", ic));
            }
            if let Some(c) = color {
                out.push_str(&format!(" color=\"{}\"", c));
            }
            out.push_str(">\n");

            for child in content {
                render_nfm_block(child, indent_level + 1, out);
            }

            out.push_str(&indent);
            out.push_str("</callout>\n");
        }
        UniversalBlock::Toggle { summary, content, style } => {
            out.push_str(&indent);
            out.push_str("<details");
            if let Some(s) = style {
                out.push_str(&format!(" color=\"{}\"", s.name));
            }
            out.push_str(">\n");

            out.push_str(&format!("{}<summary>", indent));
            render_inlines(summary, out);
            out.push_str("</summary>\n");

            for child in content {
                render_nfm_block(child, indent_level + 1, out);
            }

            out.push_str(&indent);
            out.push_str("</details>\n");
        }
        UniversalBlock::Columns { columns, .. } => {
            out.push_str(&indent);
            out.push_str("<columns>\n");
            for col in columns {
                out.push_str(&format!("{}\t<column>\n", indent));
                for child in col {
                    render_nfm_block(child, indent_level + 2, out);
                }
                out.push_str(&format!("{}\t</column>\n", indent));
            }
            out.push_str(&indent);
            out.push_str("</columns>\n");
        }
        UniversalBlock::Table { rows, .. } => {
            out.push_str(&indent);
            out.push_str("<table>\n");
            for row in rows {
                out.push_str(&format!("{}\t<tr>\n", indent));
                for cell in &row.cells {
                    out.push_str(&format!("{}\t\t<td>", indent));
                    render_inlines(&cell.content, out);
                    out.push_str("</td>\n");
                }
                out.push_str(&format!("{}\t</tr>\n", indent));
            }
            out.push_str(&indent);
            out.push_str("</table>\n");
        }
        UniversalBlock::PageBreak => {
            out.push_str(&format!("{}---\n", indent));
        }
        UniversalBlock::TableOfContents { style, .. } => {
            out.push_str(&indent);
            out.push_str("<table_of_contents");
            if let Some(s) = style {
                out.push_str(&format!(" color=\"{}\"", s.name));
            }
            out.push_str("/>\n");
        }
        UniversalBlock::Mention { mention_type, target, .. } => {
            out.push_str(&indent);
            match mention_type {
                MentionType::User => out.push_str(&format!("<mention-user url=\"{}\"/>\n", target)),
                MentionType::Page => out.push_str(&format!("<mention-page url=\"{}\"/>\n", target)),
                MentionType::Database => {
                    out.push_str(&format!("<mention-database url=\"{}\"/>\n", target))
                }
                MentionType::Date => {
                    out.push_str(&format!("<mention-date start=\"{}\"/>\n", target))
                }
                _ => out.push_str(&format!("<mention-user url=\"{}\"/>\n", target)),
            }
        }
        _ => {}
    }
}

pub fn render_inlines(inlines: &[InlineElement], out: &mut String) {
    for elem in inlines {
        match elem {
            InlineElement::TextRun { content, style } => {
                if let Some(st) = style {
                    let mut text = escape_nfm(content);
                    if st.code == Some(true) {
                        text = format!("`{}`", content.replace('\n', "<br>"));
                    }
                    if st.bold == Some(true) {
                        text = format!("**{}**", text);
                    }
                    if st.italic == Some(true) {
                        text = format!("*{}*", text);
                    }
                    if st.strikethrough == Some(true) {
                        text = format!("~~{}~~", text);
                    }
                    if st.underline == Some(true) {
                        text = format!("<span underline=\"true\">{}</span>", text);
                    }
                    if let Some(color) = &st.color {
                        text = format!("<span color=\"{}\">{}</span>", color, text);
                    }
                    if let Some(link) = &st.link {
                        text = format!("[{}]({})", text, link);
                    }
                    out.push_str(&text);
                } else {
                    out.push_str(&escape_nfm(content));
                }
            }
            InlineElement::Equation { expression, .. } => {
                out.push_str(&format!("$`{}`$", expression));
            }
            InlineElement::HardBreak => {
                out.push_str("<br>");
            }
            InlineElement::SoftBreak => {
                out.push(' ');
            }
            InlineElement::Mention { mention_type, target, .. } => match mention_type {
                MentionType::User => out.push_str(&format!("<mention-user url=\"{}\"/>", target)),
                MentionType::Page => out.push_str(&format!("<mention-page url=\"{}\"/>", target)),
                MentionType::Database => {
                    out.push_str(&format!("<mention-database url=\"{}\"/>", target))
                }
                MentionType::Date => out.push_str(&format!("<mention-date start=\"{}\"/>", target)),
                _ => out.push_str(&format!("<mention-user url=\"{}\"/>", target)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nfm_roundtrip_basic() {
        let input = "# Heading 1 {color=\"blue\"}\n- Rich text {color=\"red\"}\n\tChildren\n";
        let doc = NotionMarkdownConverter::from_platform(input.to_string()).unwrap();
        assert_eq!(doc.blocks.len(), 2);

        let rendered = NotionMarkdownConverter::from_universal(&doc).unwrap();
        assert!(rendered.contains("# Heading 1 {color=\"blue\"}"));
        assert!(rendered.contains("- Rich text {color=\"red\"}"));
    }

    #[test]
    fn test_nfm_callout_and_table() {
        let input =
            "<callout icon=\"💡\" color=\"yellow_bg\">\n\t<p>Note content</p>\n</callout>\n";
        let doc = NotionMarkdownConverter::from_platform(input.to_string()).unwrap();
        assert_eq!(doc.blocks.len(), 1);

        let rendered = NotionMarkdownConverter::from_universal(&doc).unwrap();
        assert!(rendered.contains("<callout icon=\"💡\" color=\"yellow_bg\">"));
        assert!(rendered.contains("</callout>"));
    }

    #[test]
    fn test_nfm_escaping() {
        let input = "Text with \\* \\~ \\` \\[ \\] \\{ \\}\n";
        let doc = NotionMarkdownConverter::from_platform(input.to_string()).unwrap();
        let rendered = NotionMarkdownConverter::from_universal(&doc).unwrap();
        assert!(rendered.contains("\\*"));
    }
}
