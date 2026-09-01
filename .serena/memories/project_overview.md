# Project Overview: bl1nk-kept

## 1. Project Purpose & Role

`bl1nk-kept` is a high-performance Rust workspace and task-first CLI tool (`kept`) for inspecting keywords, filesystems, duplicate files, and offline documents through a unified evidence-based architecture. <!-- rumdl-disable-line line-length -->

## 2. Workspace Crate Architecture

The project follows a strict single-responsibility separation across crates:

- **`crate/kept-core`**: Core domain logic, FQL query engine, persistent scan index, search engine (BM25, Thai bigram tokenization, n-gram fuzzy matching), duplicate detection pipeline, and data foundation contracts. <!-- rumdl-disable-line line-length -->
- **`crate/kept-doc`**: Pure library for offline document conversion through Universal IR, Notion-flavored Markdown (NFM), frontmatter metadata, and sync models. **Must not contain any binary targets.** <!-- rumdl-disable-line line-length -->
- **`crate/kept-cli`**: The user-facing binary (`kept`) providing task-first commands (`scan`, `find`, `review`, `duplicates`, `search`, `convert`, `config`, `doctor`, `group`). <!-- rumdl-disable-line line-length -->
- **`crate/kept-mcp`**: Dedicated binary crate (`bl1nk-kept-mcp`) providing the Model Context Protocol (MCP) server for AI assistants. <!-- rumdl-disable-line line-length -->

## 3. Key Architectural Boundaries & Invariants

- **Draft-07 JSON Schema Contract**: `schema/keyword-registry.schema.json` is pinned to Draft-07 JSON Schema format (using `schemars = "=0.8.22"`). Drift is strictly checked via `just schema-check`. <!-- rumdl-disable-line line-length -->
- **Docs Directory Constraint**: `docs/adr/` is the ONLY active subtree allowed in `docs/`. Architectural diagrams and exports live in `diagrams/`. <!-- rumdl-disable-line line-length -->
- **User Config**: `config.yaml` is user-owned and never rewritten implicitly. Defaults are broad, and scopes must be absolute paths chosen by the user. <!-- rumdl-disable-line line-length -->
- **No Unapproved Mutations**: Destructive file operations (e.g. rename, delete, duplicate trash/rollback) remain strictly guarded under approval boundaries. <!-- rumdl-disable-line line-length -->

## 4. Verification & Pre-Commit Gates

Always verify code changes with `just check`, which executes:

1. `fix-eol`: Line ending normalization (LF)
2. `fmt`: `cargo fmt --all -- --check`
3. `test`: `cargo test --workspace`
4. `clippy`: `cargo clippy --workspace --all-targets -- -D warnings`
5. `repo-contract`: `python -m unittest tests/test_repository_contract.py tests/test_repository_tools.py`
6. `cli-smoke`: `cargo build -q -p kept-cli --bin kept && python -m unittest tests/test_public_cli_smoke.py`
7. `schema-check`: Ensures Rust type schema matches `schema/keyword-registry.schema.json`
8. `links`: `python tools/check_markdown_links.py`
9. `version-check`: `python tools/check_version_contract.py`
