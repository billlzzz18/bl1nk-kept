# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Build, Test & Lint Commands

Cargo workspace commands:

- Build entire workspace: `cargo build --workspace`
- Run all tests: `cargo test --workspace`
- Run single package test: `cargo test -p <package-name>` (e.g. `cargo test -p kept-core`, `cargo test -p kept-cli`) <!-- rumdl-disable-line line-length -->
- Run specific test: `cargo test -p <package-name> -- <test_name>`
- Lint workspace (deny warnings): `cargo clippy --workspace --all-targets -- -D warnings` <!-- rumdl-disable-line line-length -->
- Format check: `cargo fmt --all -- --check`
- Format apply: `cargo fmt --all`

Run CLI directly:

- `cargo run -p kept-cli -- <command>` (e.g. `cargo run -p kept-cli -- doctor`, `cargo run -p kept-cli -- scan .`) <!-- rumdl-disable-line line-length -->
- Export public JSON schema: `cargo run -q -p kept-core --example export_schema`

Repository contract & validation checks (Python 3.12+):

- Contract & Tool unit tests: `python -m unittest tests/test_repository_contract.py tests/test_repository_tools.py`
- Markdown link integrity: `python tools/check_markdown_links.py`
- Version contract verification: `python tools/check_version_contract.py`

## High-Level Architecture

`bl1nk-kept` is a Rust workspace (`kept`) for keyword registries, filesystem analytics, duplicate detection, and offline document conversion.

### Crates & Core Responsibilities

```text
                      ┌──────────────┐
                      │   kept-cli   │ (CLI command interface `kept`)
                      └──┬────────┬──┘
                         │        │
           ┌─────────────┘        └─────────────┐
           ▼                                    ▼
┌──────────────────────┐             ┌──────────────────────┐
│      kept-core       │             │       kept-doc       │
│ Registry model, BM25 │             │ Universal IR &       │
│ Thai bigram search,  │             │ offline document     │
│ ScanIndex, duplicate │             │ conversion           │
│ detection pipeline   │             └──────────────────────┘
└──────────────────────┘
           ▲
           │
┌──────────────────────┐
│       kept-mcp       │ (stdio MCP server for document & table tools)
└──────────────────────┘
```

- **`crate/kept-core`**: Core domain logic and data foundations.
  - **Keyword Registry**: Versioned JSON/YAML/CSV registry, validation, BM25 inverted index with Thai bigrams and n-gram fuzzy candidate retrieval.
  - **Filesystem Analytics**: Persistent `ScanIndex` caching scan snapshots with structured `ScanIssue` diagnostics.
  - **Duplicate Detection**: 4-stage evidence pipeline (`size bucket` → `partial SHA-256` → `full SHA-256` → `group evidence`), reporting `same_name`, `near_name`, `same_content`, `hard_link`.
- **`crate/kept-doc`**: Offline document conversion engine mapping documents to a Universal IR (Intermediate Representation) JSON/Markdown AST.
- **`crate/kept-cli`**: Command-line application binary exposing `setup`, `doctor`, `scan`, `find`, `review`, `duplicates`, `config`, `group`, `search`, `convert`, `evidence`, and `corpus`.
- **`crate/kept-mcp`**: Stdio Model Context Protocol (MCP) server `bl1nk-kept-mcp` providing tool interfaces for agentic workflows.

## Key Invariants & Contracts

- **No Destructive Operations**: Scan, find, review, search, and duplicate inspection paths must never mutate scanned user files.
- **Config Management**: User config is stored in OS-appropriate directories (`%APPDATA%/kept/config.yaml` on Windows, `~/.config/kept/config.yaml` on Linux/macOS). `doctor --fix` must backup invalid configs before restoring a starter.
- **Schema Contracts**: Draft-07 JSON Schema defined in `schema/keyword-registry.schema.json`. Must match output of `export_schema` example.
- **Commit Format**: Conventional Commits `<type>(<scope>): <summary in imperative mood>`.
