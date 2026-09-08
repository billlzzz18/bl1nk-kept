---
name: using-kept
description: Use when working in this repo — guides through kept setup and usage for filesystem indexing, search, duplicates, and document conversion
---

## Setup (run once)

```powershell
kept setup        # create config.yaml
kept doctor --fix # check and repair environment
```

## CLI Commands

### Filesystem & Indexing

| Command | Purpose |
|---|---|
| `kept scan <root>` | Index workspace into ScanIndex |
| `kept find <root> -q "pattern"` | Search indexed files by filter facts or FQL |
| `kept review` | Interactive review menu for existing scan index |
| `kept duplicates <root> --action export-plan` | 4-stage SHA-256 duplicate detection |

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

Prefer over raw filesystem reads when available:

### Filesystem Tools

- `filesystem_find` — index-backed file search with metadata, scores, git status
- `filesystem_grep` — content search with context lines
- `filesystem_multi_grep` — search multiple patterns with OR semantics
- `filesystem_rescan` — trigger on-demand rescan for a canonical root
- `filesystem_status` — report index state, file count, scanning status

### Document & Search Tools

- `convert_document` — universal document conversion (GFM/NFM/DOCX/PDF)
- `convert_table` — table format conversion
- `search_documents` — lexical/semantic search for documents and pages

## Red Flags

| Don't | Do instead |
|-------|-----------|
| Read files directly | Use `kept find` or MCP filesystem tools |
| Skip `kept setup` | Run it once first |
| Grep manually | Use `filesystem_grep` |
| Delete duplicates without checking | Run `kept duplicates --action export-plan` first |
| Edit config.yaml by hand | Use `kept config` commands |