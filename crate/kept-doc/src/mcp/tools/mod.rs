//! Tool handlers exposed by the unified blink-md MCP server.
//!
//! Each submodule groups the tools for one document domain. They were
//! previously separate per-platform MCP server binaries; they now register
//! into a single server (see [`crate::mcp::server`]).

pub mod lark;
pub mod markdown;
pub mod mermaid;
pub mod notion;
pub mod notion_live;

use serde_json::{json, Value};

use crate::mcp::core::{McpError, McpResult};

/// NOTE-001: รักษา response contract `document` และ `block_count` ให้ conversion tools ทุก platform
/// ใช้ serialization เดียวกัน เพื่อลดความเสี่ยงที่ schema ของ MCP แต่ละ adapter จะค่อย ๆ ต่างกัน
pub(crate) fn serialize_document_response(document: &crate::UniversalDocument) -> McpResult<Value> {
    let document_value = serde_json::to_value(document)
        .map_err(|error| McpError::internal(format!("Serialization failed: {error}")))?;
    Ok(json!({
        "document": document_value,
        "block_count": document.blocks.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::serialize_document_response;
    use crate::converter::markdown::MarkdownConverter;
    use crate::FromPlatform;

    #[test]
    fn shared_document_response_serializes_document_and_block_count() {
        let document = MarkdownConverter::from_platform("# รายงาน\n\nเนื้อหา".to_string())
            .expect("Markdown fixture must convert to Universal IR");

        let response =
            serialize_document_response(&document).expect("Universal IR response must serialize");

        assert_eq!(response["block_count"], document.blocks.len());
        assert!(response["document"]["metadata"].is_object());
        assert!(response["document"]["blocks"].is_array());
    }
}
