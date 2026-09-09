//! The unified bl1nk-kept MCP server.
//!
//! A single server exposes every tool across Search, Convert, Diff, Notion,
//! Markdown, Lark, Obsidian Base, and Mermaid. The binary entry point
//! lives in `src/bin/bl1nk-kept-mcp.rs`.

use std::sync::Arc;

use crate::mcp::core::{init_logging, Server, ServerCapabilities};
use crate::mcp::tools;
use kept_doc::client::NotionClient;

/// Build the unified MCP server with all bl1nk-kept tools registered.
///
/// Stateless conversion/rendering tools are always available. The live Notion
/// API tools (search, get/create page, list children, trash) are registered
/// only when `NOTION_TOKEN` is set, since they need an authenticated client.
pub fn build() -> Result<Server, Box<dyn std::error::Error>> {
    let fff_manager = std::sync::Arc::new(tools::filesystem::FffManager::new());

    let mut builder = Server::builder()
        .name("bl1nk-kept-mcp")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(ServerCapabilities::tools_only())
        // Filesystem & FFF Discovery Tools
        .tool(
            "filesystem_find",
            tools::filesystem::FilesystemFindTool {
                manager: fff_manager.clone(),
            },
        )
        .tool(
            "filesystem_grep",
            tools::filesystem::FilesystemGrepTool {
                manager: fff_manager.clone(),
            },
        )
        .tool(
            "filesystem_multi_grep",
            tools::filesystem::FilesystemMultiGrepTool {
                manager: fff_manager.clone(),
            },
        )
        .tool(
            "filesystem_rescan",
            tools::filesystem::FilesystemRescanTool {
                manager: fff_manager.clone(),
            },
        )
        .tool(
            "filesystem_status",
            tools::filesystem::FilesystemStatusTool {
                manager: fff_manager.clone(),
            },
        )
        // Unified Document & Table Pipelines
        .tool("search_documents", tools::unified::SearchDocumentsTool)
        .tool("convert_document", tools::unified::ConvertDocumentTool)
        .tool("convert_table", tools::unified::ConvertTableTool)
        .tool("diff_document", tools::unified::DiffDocumentTool)
        .tool("diff_table", tools::unified::DiffTableTool)
        // Markdown
        .tool("parse_markdown", tools::markdown::ParseMarkdownTool)
        .tool("to_markdown", tools::markdown::ToMarkdownTool)
        // Notion + Universal IR (stateless conversion)
        .tool("convert_md_to_ir", tools::notion::ConvertMdToIrTool)
        .tool("convert_ir_to_md", tools::notion::ConvertIrToMdTool)
        .tool("list_platforms", tools::notion::ListPlatformsTool)
        // Lark / Feishu Sheets
        .tool("csv_to_ir", tools::lark::CsvToIrTool)
        .tool("ir_to_csv", tools::lark::IrToCsvTool)
        .tool("list_lark_platforms", tools::lark::ListLarkPlatformsTool)
        // Mermaid & Semantic Diagram / Agent Graph
        .tool(
            "document_to_diagram",
            tools::doc_to_diagram::DocumentToDiagramTool,
        )
        .tool("render_mermaid_svg", tools::mermaid::RenderMermaidSvgTool)
        .tool("list_diagram_types", tools::mermaid::ListDiagramTypesTool);

    if let Ok(token) = std::env::var("NOTION_TOKEN") {
        use tools::notion_live::{
            CreatePageTool, GetBlocksTool, GetPageBlocksTool, GetPageTool, SearchTool, TrashTool,
        };
        let client = Arc::new(NotionClient::new(token)?);
        builder = builder
            .tool(
                "get_notion_page_blocks",
                GetPageBlocksTool {
                    client: client.clone(),
                },
            )
            .tool(
                "notion_search",
                SearchTool {
                    client: client.clone(),
                },
            )
            .tool(
                "notion_get_page",
                GetPageTool {
                    client: client.clone(),
                },
            )
            .tool(
                "notion_create_page",
                CreatePageTool {
                    client: client.clone(),
                },
            )
            .tool(
                "notion_get_block_children",
                GetBlocksTool {
                    client: client.clone(),
                },
            )
            .tool("notion_trash", TrashTool { client });
    }

    Ok(builder.build()?)
}

/// Build and run the server over stdio until the client disconnects.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    build()?.run_stdio().await?;
    Ok(())
}
