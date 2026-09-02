<!-- markdownlint-disable MD013 -->
# bl1nk-kept

[ภาษาไทย](README.th.md) · [Specification](SPEC.md) · [CLI Guide](get-start.md) · [Schema](schema/README.md) · [Benchmarks](benchmarks/README.md)

**bl1nk-kept** is a high-performance Rust workspace and CLI tool (`kept`) for keyword registries, filesystem analytics, progressive duplicate detection, and offline document conversion.

| Crate | Role |
| --- | --- |
| `kept-core` | Registry model, BM25 / Thai bigram search, ScanIndex, duplicate detection, and data foundations |
| `kept-doc` | Offline document conversion through Universal IR and Notion Markdown (NFM) |
| `kept-cli` | The user-facing command-line binary `kept` |
| `kept-mcp` | Model Context Protocol (MCP) server `bl1nk-kept-mcp` for AI agent tool integration |

## Why bl1nk-kept

**One evidence path for names, files, and documents.** `kept` starts with a keyword registry and a filesystem index, then keeps the signals separate: lexical similarity is not content equality, and a duplicate claim is backed by size, partial hash, full hash, and group evidence.

**Built for Thai-aware retrieval without hiding uncertainty.** BM25, Thai bigrams, synonym compatibility, and configurable n-gram fuzzy retrieval work together while near matches remain candidates rather than silently becoming canonical data.

**Offline document work stays inspectable.** Universal IR gives Markdown conversion a typed target instead of treating documents as opaque text, while native extraction, page diagnostics, and optional OCR remain distinct stages.

**Defaults must be explainable.** Public corpus provenance, repeated experiments, raw benchmark artifacts, and a generated public schema turn implementation choices into inputs that can be inspected and revised.

## Start

### Installation & Build

```bash
cargo build --release -p kept-cli --bin kept
```

### Common Commands

```bash
kept setup
kept doctor
kept scan ./workspace
kept review ./workspace
kept find ./workspace --type pdf --min-size 50mb
kept duplicates ./workspace
kept search "keyword"
```

### Configuration

`config.yaml` is user-owned:

- `kept config` summarizes profiles and scopes.
- `kept config defaults`, `kept config profile`, and `kept config scope` manage configuration through task-level commands.
- `kept doctor --fix` recovers a missing or invalid configuration after preserving a backup.
- `kept review` analyzes naming conventions against configured profiles in read-only mode.

## Documentation

| Guide | Description |
| --- | --- |
| [CLI Reference](get-start.md) | Complete reference for all `kept` CLI subcommands and options |
| [Specification](SPEC.md) | Product specification, invariants, and architecture boundaries |
| [Registry Schema](schema/README.md) | Machine-readable Draft-07 JSON Schema and generation notes |
| [Benchmarks](benchmarks/README.md) | Performance benchmark methodology, raw datasets, and charts |
| [Contributing](CONTRIBUTING.md) | Guidelines for development, testing, and contribution |

## License

MIT
