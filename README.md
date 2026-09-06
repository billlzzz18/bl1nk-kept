# bl1nk-kept

[ภาษาไทย](README.th.md) · [Specification](SPEC.md) · [CLI Guide](get-start.md) · [Schema](schema/README.md) · [Benchmarks](benchmarks/README.md)

**bl1nk-kept** is a local-first Rust workspace providing a high-performance CLI (`kept`) and stdio MCP server (`bl1nk-kept-mcp`) for keyword registries, filesystem analytics, progressive duplicate detection, and offline document conversion.

| Crate | Responsibility |
| --- | --- |
| `kept-core` | Keyword registry model, BM25 / Thai bigram search, ScanIndex snapshots, progressive 4-stage duplicate detection, semantic search, and data foundations |
| `kept-doc` | Offline document conversion engine mapping Markdown, Notion Markdown (NFM), DOCX, and PDF to Universal IR (pure library) |
| `kept-cli` | Primary command-line interface (`kept`) with interactive menus, doctor diagnostics, and task-first execution |
| `kept-mcp` | Stdio Model Context Protocol (MCP) server `bl1nk-kept-mcp` exposing filesystem, document, and table tools for AI agents |

## Why bl1nk-kept

**1. Progressive Duplicate Detection & Evidence Chain**
- 4-stage non-destructive verification pipeline: `size bucket` → `partial SHA-256` → `full SHA-256` → `group evidence`.
- Clear categorization: `same_name`, `near_name`, `same_content`, and `hard_link`.
- Read-only inspection guarantees: user source files are never mutated implicitly.

**2. Thai-Aware & Hybrid Semantic Search**
- BM25 inverted index integrated with Thai bigrams, synonym expansion, and n-gram fuzzy candidate filtering.
- Transparent score breakdowns: near matches remain explicit candidates rather than silently polluting canonical registries.
- Support for local Ollama and hosted Jina embedding/rerank models with discrete score explanations.

**3. Filesystem Analytics & Naming Rules**
- Persistent `ScanIndex` snapshots caching directory states with structured `ScanIssue` diagnostics.
- Read-only naming rule analysis with profile inheritance, absolute path scoping, priority resolution, and conflict detection.
- Interactive terminal review menu for inspecting space usage, duplicate groups, and naming violations.

**4. Offline Universal IR Document Conversion**
- Structured Intermediate Representation (IR) converting between GitHub Flavored Markdown (GFM), Notion Markdown (NFM), DOCX, and PDF without network dependencies.

## Start

### Build from Source

```bash
cargo build --release -p kept-cli --bin kept
```

### Common Workflows

```bash
# Initialize user configuration and verify environment
kept setup
kept doctor

# Scan directory, create persistent snapshot, and inspect interactively
kept scan ./workspace

# Search and filter indexed files (fast lookup without rescanning)
kept find ./workspace --type pdf --min-size 50mb
kept find ./workspace --query "annual report"

# Inspect duplicates and analyze space distribution
kept duplicates ./workspace
kept review ./workspace

# Query keyword registry with Thai BM25 / fuzzy matching
kept search "คำค้นหา"
```

### User Configuration

User settings reside in OS-standard locations (`%APPDATA%/kept/config.yaml` on Windows, `~/.config/kept/config.yaml` on Linux/macOS):

- `kept config`: Inspect active configuration, naming profiles, and scope hierarchies.
- `kept config profile`: Manage naming rules, casing, separators, shortcuts, and similarity thresholds.
- `kept config scope`: Bind profiles to explicit absolute paths with priority and exception rules.
- `kept doctor --fix`: Validate environment and safely recover broken configs using timestamped backups (`.invalid*.bak`).

## Model Context Protocol (MCP)

Run the stdio MCP server for agentic integrations (Claude Code, Cursor, Windsurf, Zed):

```bash
cargo run --release -p kept-mcp --bin bl1nk-kept-mcp
```

Exposes tools for document conversion, table extraction, and filesystem inspection via standard JSON-RPC.

## Documentation & Architecture

| Document | Purpose |
| --- | --- |
| [CLI Reference](get-start.md) | Full documentation for all `kept` CLI subcommands, arguments, and flags |
| [Specification](SPEC.md) | Technical invariants, architectural boundaries, and crate responsibilities |
| [Registry Schema](schema/README.md) | Machine-readable Draft-07 JSON Schema generated from Rust type models |
| [Benchmarks](benchmarks/README.md) | Benchmark methodology, Thai tokenizer evaluations, raw JSONL, and charts |
| [Contributing](CONTRIBUTING.md) | Development workflow, pre-commit validation, and test commands |

## License

MIT
