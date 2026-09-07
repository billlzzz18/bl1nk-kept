//! Unified Model Context Protocol (MCP) server for bl1nk-kept.
//!
//! This module is only compiled with the `mcp` feature. It bundles every
//! document tool (Search, Convert, Diff, Notion, Markdown, Lark, Obsidian Base, Mermaid)
//! into a single server binary, `bl1nk-kept-mcp`.

pub mod core;
pub mod server;
pub mod tools;
