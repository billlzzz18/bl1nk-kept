# ADR-0007: MCP Tool Gap Analysis and Additions

## Status

Proposed

## Context

bl1nk-kept MCP server provides tools for filesystem search, grep, and AST navigation. Gap analysis against RustRover IDE tools reveals missing capabilities that would significantly improve developer workflow:

### IDE Tool Gaps
1. **get_file_problems** — No integration with `cargo clippy` diagnostics
2. **git_status** — No git status exposure from scan index
3. **search_symbol** — AST search exists in `source_graph` but not exposed as MCP tool

### Ambiguity Understanding Gap
kept is a **judge**, not a grammar checker. When users communicate through MCP clients, kept needs to understand meaning, not correct errors. Current system lacks:
- Understanding of ambiguous/misspelled terms from conversation
- Mapping between variant forms and intended meanings
- Learning from user language patterns over time

## Decision

Add four new MCP tools to `kept-mcp`:

### IDE Tools

#### 1. `get_file_problems`
- **Purpose**: Run `cargo clippy` and return diagnostics per file
- **Implementation**: Shell out to `cargo clippy --message-format=json`, parse JSON output, return structured diagnostics
- **Output**: `{ file, line, column, severity, message, code }[]`

#### 2. `git_status`
- **Purpose**: Show git status integrated with scan index
- **Implementation**: Run `git status --porcelain=v1`, cross-reference with `ScanIndex` to show tracked/untracked/modified files
- **Output**: `{ path, status: "modified" | "added" | "deleted" | "untracked", in_scan_index: bool }[]`

#### 3. `search_symbol`
- **Purpose**: Expose `source_graph` AST search as MCP tool
- **Implementation**: Wrap `SourceGraph::search()` or `GraphManager::find_symbol()` in MCP tool handler
- **Input**: `{ query: string, language?: string, kind?: "function" | "struct" | "trait" | "impl" }`
- **Output**: `{ name, qualified_name, file, line, kind, signature }[]`

### Ambiguity Understanding System

#### 4. `understand_term`
- **Purpose**: Understand meaning of ambiguous/misspelled terms from conversation
- **Implementation**: Accumulate term variants from MCP client interactions, map to intended meanings
- **Input**: `{ term: string, context?: string }`
- **Output**: `{ term, meaning, confidence, alternatives: [{term, meaning, score}] }`

#### 5. `get_ambiguity_map`
- **Purpose**: Retrieve accumulated ambiguity mappings
- **Implementation**: Query accumulated term database
- **Input**: `{ filter?: "ambiguous" | "frequent" | "recent" }`
- **Output**: `{ entries: [{variant, meaning, confidence, occurrences}] }`

## Consequences

### Positive
- Complete IDE-like workflow without leaving MCP context
- Diagnostics enable AI agents to fix code issues directly
- Git integration provides version control awareness
- AST search enables precise code navigation
- Ambiguity understanding enables kept to judge meaning, not just syntax
- System learns from actual user language patterns

### Negative
- `get_file_problems` requires `cargo` in PATH (runtime dependency)
- `git_status` adds git dependency (currently optional)
- `search_symbol` may overlap with existing `fff grep` for simple cases
- Ambiguity system requires persistent storage for accumulated terms

### Mitigations
- Gate `get_file_problems` behind `#[cfg(feature = "clippy")]` feature flag
- Make `git_status` gracefully degrade when not in git repo
- Document when to use `search_symbol` vs `fff grep`
- Store ambiguity data in `data/glossary/ambiguity.json`

## References

- `kept-mcp/src/mcp/tools/` — existing MCP tool implementations
- `kept-core/src/source_graph/` — AST search implementation
- `kept-core/src/scanner/` — scan index for git status cross-reference
- `kept-core/src/context/judge.rs` — existing ambiguity handling (AMBIGUOUS_TERMS)
- `kept-core/src/foundation.rs` — glossary normalization logic
