//! Unified Document, Table, and Diff MCP Tools
//!
//! Exposes the 4 core pipeline capabilities:
//! 1. `search_documents` - Discovery & locator
//! 2. `convert_document` - Prose / Markdown / Notion / Google Docs conversion
//! 3. `convert_table` - Structured Tabular / Obsidian Base / CSV / Sheets conversion
//! 4. `diff_document` - Structural IR diff & ChangeSet preview for prose documents
//! 5. `diff_table` - Structural row/cell diff & ChangeSet preview for tabular databases

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::mcp::core::{
    invalid_args, McpError, McpResult, RequestHandlerExtra, SchemaBuilder, ToolHandler, ToolInfo,
};
use kept_doc::converter::filter::{Filter, ThaiSanitizationFilter};
use kept_doc::converter::lark_sheets::LarkSheetAdapter;
use kept_doc::converter::markdown::MarkdownConverter;
use kept_doc::converter::obsidian_base::ObsidianBaseAdapter;
use kept_doc::converter::{FromPlatform, ToPlatform};
use kept_doc::ir::{UniversalBlock, UniversalDocument};
use kept_doc::sync::Reconciler;

fn internal(e: impl std::fmt::Display) -> McpError {
    McpError::internal(e.to_string())
}

// NOTE-001: หยุดทันทีเมื่อพอตาม limit — ไม่ย่อย tree ทั้งผืนแล้วค่อยตัดทิ้ง
fn collect_markdown_files(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<PathBuf>,
    budget: &mut usize,
) {
    if depth > max_depth || *budget == 0 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if *budget == 0 {
                return;
            }
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    collect_markdown_files(&path, depth + 1, max_depth, out, budget);
                }
            } else if path.extension().is_some_and(|ext| ext == "md") {
                out.push(path);
                *budget -= 1;
            }
        }
    }
}

// =============================================================================
// 1. Search Documents Tool
// =============================================================================

#[derive(Default)]
pub struct SearchDocumentsTool;

#[derive(Deserialize)]
struct SearchInput {
    query: Option<String>,
    platform: Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct SearchResultItem {
    id: String,
    title: String,
    platform: String,
    path_or_url: String,
}

#[async_trait]
impl ToolHandler for SearchDocumentsTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: SearchInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;

        let query = input.query.unwrap_or_default().to_lowercase();
        let target_platform = input.platform.unwrap_or_else(|| "all".to_string());
        let limit = input.limit.unwrap_or(20);

        let mut results: Vec<SearchResultItem> = Vec::new();

        // Local workspace markdown files discovery
        if target_platform == "all"
            || target_platform == "local"
            || target_platform == "markdown"
            || target_platform == "obsidian"
        {
            let mut md_files = Vec::new();
            let mut budget = limit * 2;
            collect_markdown_files(Path::new("."), 0, 4, &mut md_files, &mut budget);

            for path in md_files.into_iter() {
                let path_str = path.to_string_lossy().to_string();
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if query.is_empty()
                    || filename.to_lowercase().contains(&query)
                    || path_str.to_lowercase().contains(&query)
                {
                    results.push(SearchResultItem {
                        id: path_str.clone(),
                        title: filename,
                        platform: "local_markdown".to_string(),
                        path_or_url: path_str,
                    });
                }
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(json!({
            "results": results,
            "total_found": results.len(),
            "query": query,
            "platform_filter": target_platform,
        }))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "search_documents",
            Some("Search for documents, markdown files, and platform pages".to_string()),
            SchemaBuilder::new()
                .optional_param("query", "Search keyword or text")
                .optional_param(
                    "platform",
                    "Platform filter (all, local, markdown, notion, obsidian)",
                )
                .build(),
        ))
    }
}

// =============================================================================
// 2. Convert Document Tool
// =============================================================================

#[derive(Default)]
pub struct ConvertDocumentTool;

#[derive(Deserialize)]
struct ConvertDocInput {
    #[serde(alias = "source_format", alias = "from_platform", alias = "from")]
    source_platform: String,
    #[serde(alias = "content", alias = "markdown", alias = "text")]
    source_content: String,
    /// Base64-encoded document bytes; required for binary sources (pdf, docx).
    #[serde(alias = "source_base64", alias = "content_base64", default)]
    source_content_base64: Option<String>,
    #[serde(alias = "target_format", alias = "from_universal", alias = "to")]
    target_platform: String,
    sanitize_thai: Option<bool>,
}

/// Decode a binary source: prefer explicit base64 input, reject raw text.
fn decode_binary_source(input: &ConvertDocInput) -> Result<Vec<u8>, McpError> {
    if let Some(encoded) = &input.source_content_base64 {
        return base64_decode(encoded).ok_or_else(|| {
            McpError::invalid_params("sourceContentBase64 is not valid base64".to_string())
        });
    }
    Err(McpError::invalid_params(format!(
        "binary source platform requires `sourceContentBase64`; raw text cannot carry {} bytes",
        input.source_platform
    )))
}

fn base64_decode(value: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let value = value.trim();
    let value = match value
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(','))
    {
        Some((_, data)) => data,
        None => value,
    };
    let mut buffer = Vec::with_capacity(value.len() * 3 / 4);
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' => {
                let index = ALPHABET.iter().position(|c| *c == byte)?;
                accumulator = (accumulator << 6) | index as u32;
                bits += 6;
                if bits >= 8 {
                    bits -= 8;
                    buffer.push((accumulator >> bits) as u8);
                }
            }
            b'=' | b'\r' | b'\n' | b' ' => {}
            _ => return None,
        }
    }
    Some(buffer)
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut block = [0u8; 4];
        block[1..][..chunk.len()].copy_from_slice(chunk);
        let word = u32::from_be_bytes(block);
        for shift in [18, 12, 6, 0] {
            let index = ((word >> shift) & 0x3F) as usize;
            output.push(ALPHABET[index] as char);
        }
        for _ in chunk.len()..3 {
            output.pop();
        }
    }
    for _ in 0..(output.len() % 4) {
        output.push('=');
    }
    output
}

#[async_trait]
impl ToolHandler for ConvertDocumentTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: ConvertDocInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;

        let src_plat = input.source_platform.to_lowercase();
        let tgt_plat = input.target_platform.to_lowercase();

        // 1. Source -> Universal IR
        let mut doc = match src_plat.as_str() {
            "markdown" | "md" | "github" | "obsidian" => {
                MarkdownConverter::from_platform(input.source_content).map_err(internal)?
            }
            "notion" | "notion_blocks" => {
                // Parse JSON notion blocks if provided as JSON string
                let blocks: Vec<UniversalBlock> = serde_json::from_str(&input.source_content)
                    .map_err(|e| McpError::invalid_params(format!("Invalid Notion JSON: {e}")))?;
                UniversalDocument {
                    metadata: kept_doc::ir::DocumentMetadata::default(),
                    blocks,
                    styles: kept_doc::ir::StyleSheet::default(),
                }
            }
            "pdf" => {
                let bytes = decode_binary_source(&input)?;
                kept_doc::converter::pdf::PdfAdapter::read_bytes(&bytes).map_err(internal)?
            }
            "docx" | "word" => {
                let bytes = decode_binary_source(&input)?;
                kept_doc::converter::docx::DocxAdapter::read_bytes(&bytes).map_err(internal)?
            }
            other => {
                return Err(McpError::invalid_params(format!(
                    "Unsupported source document platform: {other}"
                )));
            }
        };

        // 2. Optional Thai sanitization filter
        if input.sanitize_thai.unwrap_or(false) {
            let filter = ThaiSanitizationFilter;
            filter.apply(&mut doc).map_err(internal)?;
        }

        let block_count = doc.blocks.len();

        // 3. Universal IR -> Target
        let output_value = match tgt_plat.as_str() {
            "markdown" | "md" | "github" | "obsidian" => {
                let md = MarkdownConverter::from_universal(&doc).map_err(internal)?;
                json!({ "markdown": md })
            }
            "notion" | "notion_blocks" => {
                json!({ "blocks": doc.blocks })
            }
            "docx" | "word" => {
                let bytes =
                    kept_doc::converter::docx::DocxAdapter::write_bytes(&doc).map_err(internal)?;
                json!({
                    "docx_base64": base64_encode(&bytes),
                    "docx_bytes_len": bytes.len(),
                    "status": "docx_generated_successfully"
                })
            }
            "ir" | "universal_ir" => {
                json!({ "document": doc })
            }
            other => {
                return Err(McpError::invalid_params(format!(
                    "Unsupported target document platform: {other}"
                )));
            }
        };

        Ok(json!({
            "success": true,
            "source_platform": src_plat,
            "target_platform": tgt_plat,
            "block_count": block_count,
            "output": output_value,
        }))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "convert_document",
            Some(
                "Unified prose document converter (Markdown <-> Universal IR <-> Notion Blocks)"
                    .to_string(),
            ),
            SchemaBuilder::new()
                .param(
                    "source_platform",
                    "Source platform: markdown, notion, obsidian",
                )
                .param("source_content", "Source text or JSON block content")
                .param("target_platform", "Target platform: markdown, notion, ir")
                .optional_bool_param("sanitize_thai", "Apply Thai text sanitization filter")
                .build(),
        ))
    }
}

// =============================================================================
// 3. Convert Table Tool (Obsidian Base / CSV / Lark Sheets / Notion DB)
// =============================================================================

#[derive(Default)]
pub struct ConvertTableTool;

#[derive(Deserialize)]
struct ConvertTableInput {
    #[serde(alias = "source_format", alias = "from_platform", alias = "from")]
    source_platform: String,
    #[serde(
        alias = "table_data",
        alias = "content",
        alias = "data",
        alias = "csv",
        alias = "markdown_table"
    )]
    source_data: String,
    #[serde(alias = "target_format", alias = "from_universal", alias = "to")]
    target_platform: String,
}

#[async_trait]
impl ToolHandler for ConvertTableTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: ConvertTableInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;

        let src_plat = input.source_platform.to_lowercase();
        let tgt_plat = input.target_platform.to_lowercase();

        // 1. Source -> Universal Table IR
        let doc = match src_plat.as_str() {
            "obsidian_base" | "obsidian" | "markdown_table" | "md_table" => {
                ObsidianBaseAdapter::from_platform(input.source_data).map_err(internal)?
            }
            "csv" | "sheets" | "lark_sheets" | "lark" => {
                LarkSheetAdapter::from_platform(input.source_data).map_err(internal)?
            }
            other => {
                return Err(McpError::invalid_params(format!(
                    "Unsupported source table platform: {other}"
                )));
            }
        };

        let row_count = doc
            .blocks
            .iter()
            .find_map(|b| match b {
                UniversalBlock::Table { rows, .. } => Some(rows.len()),
                _ => None,
            })
            .unwrap_or(0);

        // 2. Universal Table IR -> Target
        let output_value = match tgt_plat.as_str() {
            "obsidian_base" | "obsidian" | "markdown_table" | "md_table" => {
                let md_table = ObsidianBaseAdapter::from_universal(&doc).map_err(internal)?;
                json!({ "table": md_table })
            }
            "csv" | "sheets" | "lark_sheets" | "lark" => {
                let csv = LarkSheetAdapter::from_universal(&doc).map_err(internal)?;
                json!({ "csv": csv })
            }
            "ir" | "universal_ir" => {
                json!({ "document": doc })
            }
            other => {
                return Err(McpError::invalid_params(format!(
                    "Unsupported target table platform: {other}"
                )));
            }
        };

        Ok(json!({
            "success": true,
            "source_platform": src_plat,
            "target_platform": tgt_plat,
            "row_count": row_count,
            "output": output_value,
        }))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "convert_table",
            Some("Unified table/database converter (Obsidian Base <-> CSV <-> Lark Sheets <-> Table IR)".to_string()),
            SchemaBuilder::new()
                .param("source_platform", "Source table platform: obsidian_base, csv, lark_sheets")
                .param("source_data", "Source table content (Markdown table string or CSV string)")
                .param("target_platform", "Target table platform: obsidian_base, csv, lark_sheets, ir")
                .build(),
        ))
    }
}

// =============================================================================
// 4. Diff Document Tool
// =============================================================================

#[derive(Default)]
pub struct DiffDocumentTool;

#[derive(Deserialize)]
struct DiffDocInput {
    #[serde(alias = "original", alias = "left", alias = "base")]
    left_content: String,
    left_platform: Option<String>,
    #[serde(alias = "modified", alias = "right", alias = "new", alias = "target")]
    right_content: String,
    right_platform: Option<String>,
}

#[async_trait]
impl ToolHandler for DiffDocumentTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: DiffDocInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;

        let left_plat = input
            .left_platform
            .unwrap_or_else(|| "markdown".to_string());
        let right_plat = input
            .right_platform
            .unwrap_or_else(|| "markdown".to_string());

        let left_doc = match left_plat.as_str() {
            "markdown" | "md" | "github" | "obsidian" => {
                MarkdownConverter::from_platform(input.left_content).map_err(internal)?
            }
            _ => {
                return Err(McpError::invalid_params(
                    "Unsupported left document platform",
                ))
            }
        };

        let right_doc = match right_plat.as_str() {
            "markdown" | "md" | "github" | "obsidian" => {
                MarkdownConverter::from_platform(input.right_content).map_err(internal)?
            }
            _ => {
                return Err(McpError::invalid_params(
                    "Unsupported right document platform",
                ))
            }
        };

        let changeset = Reconciler::diff(&left_doc, &right_doc).map_err(internal)?;

        Ok(json!({
            "has_changes": !changeset.is_empty(),
            "summary": changeset.summary(),
            "ops_count": changeset.ops.len(),
            "changeset": changeset,
        }))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "diff_document",
            Some("Structural diff & ChangeSet generator between two prose documents".to_string()),
            SchemaBuilder::new()
                .param("left_content", "Original/Local document content")
                .param(
                    "right_content",
                    "Target/Remote document content to compare against",
                )
                .optional_param(
                    "left_platform",
                    "Left document platform (default: markdown)",
                )
                .optional_param(
                    "right_platform",
                    "Right document platform (default: markdown)",
                )
                .build(),
        ))
    }
}

// =============================================================================
// 5. Diff Table Tool
// =============================================================================

#[derive(Default)]
pub struct DiffTableTool;

#[derive(Deserialize)]
struct DiffTableInput {
    #[serde(
        alias = "original_table",
        alias = "original",
        alias = "left",
        alias = "base_table"
    )]
    left_table: String,
    left_platform: Option<String>,
    #[serde(
        alias = "modified_table",
        alias = "modified",
        alias = "right",
        alias = "target_table"
    )]
    right_table: String,
    right_platform: Option<String>,
}

#[async_trait]
impl ToolHandler for DiffTableTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: DiffTableInput =
            serde_json::from_value(args).map_err(|e| invalid_args("Invalid args", e))?;

        let left_plat = input
            .left_platform
            .unwrap_or_else(|| "obsidian_base".to_string());
        let right_plat = input
            .right_platform
            .unwrap_or_else(|| "obsidian_base".to_string());

        let left_doc = match left_plat.as_str() {
            "obsidian_base" | "obsidian" | "markdown_table" => {
                ObsidianBaseAdapter::from_platform(input.left_table).map_err(internal)?
            }
            "csv" | "sheets" | "lark" => {
                LarkSheetAdapter::from_platform(input.left_table).map_err(internal)?
            }
            _ => return Err(McpError::invalid_params("Unsupported left table platform")),
        };

        let right_doc = match right_plat.as_str() {
            "obsidian_base" | "obsidian" | "markdown_table" => {
                ObsidianBaseAdapter::from_platform(input.right_table).map_err(internal)?
            }
            "csv" | "sheets" | "lark" => {
                LarkSheetAdapter::from_platform(input.right_table).map_err(internal)?
            }
            _ => return Err(McpError::invalid_params("Unsupported right table platform")),
        };

        let changeset = Reconciler::diff(&left_doc, &right_doc).map_err(internal)?;

        Ok(json!({
            "has_changes": !changeset.is_empty(),
            "summary": changeset.summary(),
            "ops_count": changeset.ops.len(),
            "changeset": changeset,
        }))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "diff_table",
            Some("Structural diff & ChangeSet generator between two tables/databases (Obsidian Base / CSV)".to_string()),
            SchemaBuilder::new()
                .param("left_table", "Original table content (Obsidian Base Markdown table or CSV)")
                .param("right_table", "Target table content to compare against")
                .optional_param("left_platform", "Left platform: obsidian_base, csv (default: obsidian_base)")
                .optional_param("right_platform", "Right platform: obsidian_base, csv (default: obsidian_base)")
                .build(),
        ))
    }
}
