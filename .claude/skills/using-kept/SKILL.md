---
name: using-kept
description: Use when working in this repo — guides through kept setup and usage for filesystem indexing, search, duplicates, and document conversion.
triggers:
  - kept setup
  - kept CLI
  - kept scan
  - kept find
  - kept duplicates
  - kept convert
  - filesystem indexing
  - document conversion
role: kept CLI reference — setup, commands, MCP tools, gotchas
scope: kept-cli, kept-mcp, and workspace tooling
output-format: CLI commands and configuration steps
---

## Prerequisites

- Rust toolchain (stable) — `rustup show`
- `cargo` on PATH

## Setup (run once)

### 1. Build the CLI

```powershell
cargo build -p kept-cli        # creates target/debug/kept.exe
$env:PATH += ";$PWD\target\debug"
```

### 2. Build MCP server (if using MCP tools)

```powershell
cargo build -p kept-mcp        # creates target/debug/bl1nk-kept-mcp.exe
```

Or install from release:
```powershell
./install-mcp.ps1              # downloads to %LOCALAPPDATA%\kept\bin\
```

### 3. Initialize config

```powershell
kept setup                     # creates ~/.config/kept/config.yaml
kept doctor --fix              # verify environment, fix broken config
```

## CLI Commands

### Filesystem & Indexing

| Command | Purpose |
|---|---|
| `kept scan <root>` | Index workspace into ScanIndex |
| `kept find <root> -q "pattern"` | Search indexed files by filter facts or FQL |
| `kept review` | Interactive review menu for existing scan index |
| `kept duplicates <root> --action export-plan` | 4-stage SHA-256 duplicate detection |
| `kept duplicates <root> --action trash --yes` | Trash duplicates (non-interactive) |
| `kept duplicates <root> --action delete --yes` | Delete duplicates (⚠️ permanent) |
| `kept duplicates <root> --action rollback` | Rollback last duplicate mutation |

### Document Conversion

| Command | Purpose |
|---|---|
| `kept convert <in> <out>` | Document IR conversion (GFM/NFM/DOCX/PDF) |

### Search & Registry

| Command | Purpose |
|---|---|
| `kept search <registry.json> <query>` | Thai-aware BM25 keyword search |
| `kept group` | Inspect and manage keyword registry groups |

### Configuration

| Command | Purpose |
|---|---|
| `kept config` | View or modify user config.yaml and naming rules |
| `kept doctor` | Inspect configuration and runtime environment health |

### Evidence & Corpus

| Command | Purpose |
|---|---|
| `kept evidence` | Run the offline evidence and correction loop |
| `kept corpus` | Import, validate, snapshot, and replay reviewed gold assertions |

## MCP Tools (bl1nk-kept-mcp)

Requires `bl1nk-kept-mcp` running as MCP server. Register with Claude Code:
```powershell
claude mcp add -s user kept -- "$PWD\target\debug\bl1nk-kept-mcp.exe"
```

### Filesystem Tools

- `filesystem_find` — index-backed file search with metadata, scores, git status
- `filesystem_grep` — content search with context lines
- `filesystem_multi_grep` — search multiple patterns with OR semantics
- `filesystem_rescan` — force full rescan (index auto-refreshes via watcher)
- `filesystem_status` — report index state, file count, scanning status

### Document & Search Tools

- `convert_document` — universal document conversion (GFM/NFM/DOCX/PDF)
- `convert_table` — table format conversion
- `search_documents` — lexical/semantic search for documents and pages

## Common Gotchas

| Symptom | Fix |
|---------|-----|
| `kept: command not found` | Add `target/debug` to PATH or use full path |
| `ไม่พบ config` | Run `kept setup` first |
| `Naming policy: no config` | Run `kept setup` or `kept doctor --fix` |
| Duplicate scan empty | Run `kept scan <root>` first to build index |
| Non-interactive mutation fails | Must use `--action <name> --yes` together |
| MCP tools unavailable | Build + register MCP server: see Setup step 2 |

## Red Flags

| Don't | Do instead |
|-------|-----------|
| Read files directly | Use `kept find` or MCP filesystem tools |
| Skip `kept setup` | Run it once first |
| Grep manually | Use `filesystem_grep` |
| Delete duplicates without checking | Run `kept duplicates --action export-plan` first |
| Edit config.yaml by hand | Use `kept config` commands |
| Run mutation without `--yes` in CI | Always pair `--action` with `--yes` |
