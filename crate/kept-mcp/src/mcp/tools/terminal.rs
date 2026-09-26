//! Headless virtual terminal tools backed by `kept-core::terminal::runner`.
//!
//! These tools let an agent run a program and read the *rendered* screen instead
//! of raw escape sequences: carriage-return progress rewrites, cursor addressing
//! and alternate-screen output collapse into the final visible text.

use std::collections::BTreeMap;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use crate::mcp::core::{
    McpError, McpResult, RequestHandlerExtra, SchemaBuilder, ToolHandler, ToolInfo, invalid_args,
};
use kept_core::terminal::runner::{
    DEFAULT_COLUMNS, DEFAULT_HISTORY_LINES, DEFAULT_LINES, TerminalRunOptions, TerminalRunRequest,
    render_terminal_bytes, run_terminal_command,
};

/// Geometry reported by the terminal tools when a caller omits every dimension.
pub const DEFAULT_TERMINAL_GEOMETRY: (usize, usize, usize) =
    (DEFAULT_COLUMNS, DEFAULT_LINES, DEFAULT_HISTORY_LINES);

#[derive(Debug, Deserialize)]
struct TerminalRunInput {
    program: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    working_directory: Option<String>,
    #[serde(default)]
    environment: BTreeMap<String, String>,
    #[serde(default)]
    stdin: Option<String>,
    #[serde(default)]
    columns: Option<usize>,
    #[serde(default)]
    lines: Option<usize>,
    #[serde(default)]
    history_lines: Option<usize>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TerminalRenderInput {
    content: String,
    #[serde(default)]
    columns: Option<usize>,
    #[serde(default)]
    lines: Option<usize>,
    #[serde(default)]
    history_lines: Option<usize>,
}

fn options_from(
    columns: Option<usize>,
    lines: Option<usize>,
    history_lines: Option<usize>,
    timeout_ms: Option<u64>,
) -> TerminalRunOptions {
    let defaults = TerminalRunOptions::default();
    TerminalRunOptions {
        columns: columns.unwrap_or(defaults.columns),
        lines: lines.unwrap_or(defaults.lines),
        history_lines: history_lines.unwrap_or(defaults.history_lines),
        timeout_ms,
    }
}

/// Shared viewport parameters for both terminal tools.
fn geometry_schema(builder: SchemaBuilder) -> SchemaBuilder {
    builder
        .optional_integer_param("columns", "Viewport width in character cells")
        .optional_integer_param("lines", "Viewport height in character cells")
        .optional_integer_param("history_lines", "Scrollback rows kept behind the viewport")
}

/// Run a child process inside a headless virtual terminal.
pub struct TerminalRunTool;

#[async_trait]
impl ToolHandler for TerminalRunTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: TerminalRunInput = serde_json::from_value(args)
            .map_err(|error| invalid_args("Invalid arguments", error))?;

        let options =
            options_from(input.columns, input.lines, input.history_lines, input.timeout_ms);
        let request = TerminalRunRequest {
            program: input.program,
            args: input.args,
            working_directory: input.working_directory.map(PathBuf::from),
            environment: input.environment.into_iter().collect(),
            stdin: input.stdin,
        };

        // NOTE-TERM-004: ใช้ spawn_blocking เพราะ runner รอ process จริง ถ้ารันตรงใน async task
        // จะบล็อก runtime thread ของ MCP server จน client อื่นรอไม่จบ
        let result = tokio::task::spawn_blocking(move || run_terminal_command(&request, &options))
            .await
            .map_err(|error| McpError::internal(format!("Terminal run task failed: {error}")))?
            .map_err(|error| McpError::internal(format!("Terminal run failed: {error}")))?;

        serde_json::to_value(result)
            .map_err(|error| McpError::internal(format!("Serialization failed: {error}")))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "terminal_run",
            Some(
                "Run a program inside a headless virtual terminal and return the rendered screen"
                    .to_string(),
            ),
            geometry_schema(
                SchemaBuilder::new()
                    .param("program", "Executable to run")
                    .optional_array_param("args", "Arguments passed to the executable")
                    .optional_param("working_directory", "Working directory for the child process")
                    .optional_array_param(
                        "environment",
                        "Extra environment variables as [{key, value}] objects",
                    )
                    .optional_param("stdin", "Text written to stdin before it is closed")
                    .optional_integer_param("timeout_ms", "Kill the process after this many ms"),
            )
            .build(),
        ))
    }
}

/// Render a captured ANSI byte stream without spawning a process.
pub struct TerminalRenderTool;

#[async_trait]
impl ToolHandler for TerminalRenderTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let input: TerminalRenderInput = serde_json::from_value(args)
            .map_err(|error| invalid_args("Invalid arguments", error))?;

        let options = options_from(input.columns, input.lines, input.history_lines, None);
        let snapshot = render_terminal_bytes(input.content.as_bytes(), &options);

        serde_json::to_value(snapshot)
            .map_err(|error| McpError::internal(format!("Serialization failed: {error}")))
    }

    fn metadata(&self) -> Option<ToolInfo> {
        Some(ToolInfo::new(
            "terminal_render",
            Some("Render captured ANSI terminal output into a clean screen snapshot".to_string()),
            geometry_schema(
                SchemaBuilder::new().param("content", "Raw terminal byte stream as text"),
            )
            .build(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn run_input_defaults_to_the_shared_geometry() {
        let input: TerminalRunInput =
            serde_json::from_value(json!({ "program": "cargo" })).expect("input must deserialize");

        let options =
            options_from(input.columns, input.lines, input.history_lines, input.timeout_ms);

        assert_eq!(options.columns, DEFAULT_TERMINAL_GEOMETRY.0);
        assert_eq!(options.lines, DEFAULT_TERMINAL_GEOMETRY.1);
        assert_eq!(options.history_lines, DEFAULT_TERMINAL_GEOMETRY.2);
        assert_eq!(options.timeout_ms, None);
    }

    #[test]
    fn run_input_maps_the_environment_object_into_pairs() {
        let input: TerminalRunInput = serde_json::from_value(json!({
            "program": "cargo",
            "environment": { "RUST_LOG": "info" }
        }))
        .expect("input must deserialize");

        let pairs: Vec<(String, String)> = input.environment.into_iter().collect();
        assert_eq!(pairs, vec![("RUST_LOG".to_string(), "info".to_string())]);
    }

    #[test]
    fn render_schema_requires_only_content_and_types_geometry_as_integer() {
        let schema = geometry_schema(
            SchemaBuilder::new().param("content", "Raw terminal byte stream as text"),
        )
        .build();

        assert_eq!(schema["required"], json!(["content"]));
        assert_eq!(schema["properties"]["columns"]["type"], json!("integer"));
        assert_eq!(schema["properties"]["lines"]["type"], json!("integer"));
        assert_eq!(schema["properties"]["history_lines"]["type"], json!("integer"));
    }

    #[test]
    fn render_collapses_erase_line_and_progress_rewrites() {
        let snapshot = render_terminal_bytes(
            b"\x1b[2Kdownloading 10%\r\x1b[2Kdownloading 100%",
            &TerminalRunOptions::default(),
        );

        assert_eq!(snapshot.screen, "downloading 100%");
    }

    #[test]
    fn default_geometry_matches_the_runner_defaults() {
        let defaults = TerminalRunOptions::default();

        assert_eq!(DEFAULT_TERMINAL_GEOMETRY.0, defaults.columns);
        assert_eq!(DEFAULT_TERMINAL_GEOMETRY.1, defaults.lines);
        assert_eq!(DEFAULT_TERMINAL_GEOMETRY.2, defaults.history_lines);
    }
}
