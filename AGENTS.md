# bl1nk-kept — Agent Instructions

## 0. Rust Core Standards

### Safety & Error Handling
- **No panic / avoid `unwrap()`:** Use `?`, `match`, or `if let` exclusively
- **Don't swallow errors with `let _ =`:** Propagate with `?` or log — never ignore silently
- **Guard index access:** Avoid `arr[i]`; use `.get(i)` for bounds safety

### Coding Philosophy & Ergonomics
- **Correctness before performance:** Code must be correct and readable first; optimize only hot paths with benchmarks
- **Full variable names:** No cryptic abbreviations (e.g., `query` not `q`, `buffer` not `buf`)
- **No gratuitous sub-files:** Extend existing modules unless it's a genuine new logical component
- **Variable shadowing for async/clones:** Limit clone scope:

```rust
let client = client.clone();
tokio::spawn(async move {
    client.execute().await;
});
```

### Module Structure
- **No `mod.rs`:** Use modern path convention (e.g., `src/scanner.rs` + `src/scanner/fff.rs`)

### Agent Discipline
- **Strict scope adherence:** Work only within agreed boundaries
- **Don't guess vague terms as preferred actions:**
  - "Test" → `cargo test`, Dogfooding, CLI execution, or result verification — observe context
  - "Error/issue" → agent hallucination, work mismatch, or hang — not just syntax errors
- **Record findings:** Reading/searching without synthesizing insights or artifacts = unproductive
- **No re-reading unchanged files:** If read in this session and unmodified, use existing context

---

## 1. Workspace Overview

| Crate | Purpose |
|-------|---------|
| `kept-core` | Core logic: keyword validation, search, filesystem, observation model, Judge engine |
| `kept-cli` | CLI binary (`kept`) — user command surface |
| `kept-mcp` | MCP server binary (`bl1nk-kept-mcp`) — long-running MCP service |
| `kept-doc` | Document sync/conversion library (Notion, Markdown, PDF, DOCX) |
| `kept-grammar` | Grammar types, keyword validation, naming profiles, config resolution |
| `kept-agent` | Agent integration crate (internal) |

---

## 2. Start Here

1. **Read `.agents/MEMORY.md`:** Identify Requirement ID; check `MISSING`/`PARTIAL` status
2. **Read `TODO.md`:** Pick highest-priority checkbox matching the requirement
3. **Read `SPEC.md` and `plan.md`:** Understand boundaries and contracts before editing
4. **Read `.learnings/ERRORS.md`:** Avoid previously logged mistakes

---

## 3. Work Loop & Acceptance Criteria

1. **Strict TDD:** Write failing test matching the target before production code
2. **Minimal implementation:** Change only what's needed to pass the test
3. **Verify for real:** Run `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`
4. **Update status:** Mark `TODO.md`; log public changes in `CHANGELOG.md`

**Command shortcuts via `just`:**
- `just check` — full quality gate (fmt, clippy, tests, contracts, schema)
- `just test` — `cargo test --workspace`
- `just clippy` — `cargo clippy --workspace --all-targets -- -D warnings`
- `just fmt` — `cargo fmt --all -- --check`
- `just cli-smoke` — build CLI + run public smoke tests
- `just schema` / `just schema-check` — export/verify JSON Schema

---

## 4. Comment & Language Standards

- **Internal rationale:** `// NOTE-001:`, `// NOTE-002:` — sequential, Thai
- **Public Rustdoc (`///`):** English only — public APIs, structs, CLI help
- **Error messages:** English only
- **Project docs & guidelines:** Thai

---

## 5. External Tools (MCP Servers)

| Tool | Binary | Purpose |
|------|--------|---------|
| FFF MCP | `C:\Users\Admin\AppData\Local\fff-mcp\bin\fff-mcp.exe` | Filesystem search, find, grep, multi-grep, rescan |
| Serena MCP | `C:\Users\Admin\.local\bin\serena.exe start-mcp-server` | LSP, AST symbols, diagnostics, targeted edits |
| SQZ MCP | `C:\Users\Admin\.cargo\bin\sqz-mcp.exe` | Token compression, context dedup, file reading |

Configured in `.mcp.json` for stdio transport.

---

## 6. Key Architecture Facts

### Observation Model (kept-core)
All data access goes through `Observation` with:
- **Source:** `File`, `Grep`, `TreeSitter`, `Parser`, `Fts`, `Bm25`, `Vector`, etc.
- **Target URI:** Unambiguous resource locator (`file://`, `symbol://`, `search://`, `document://`, `context://`)
- **Revision & ContentIdentity:** Timestamp/token + content hash for change detection

### Look vs View
- `look` — lightweight: identity, size, revision, outline (no full content)
- `view` — materialize content by range or symbol when needed

### Judge Engine Treatments
- `PASS` — new content, deliver full
- `REFERENCE` — seen before, unchanged (13-token pointer)
- `DELTA` — seen before, changed (diff only)
- `COMPRESS` — structure-aware compression when worthwhile
- `WARN` / `BLOCK` — repeated redundant requests (≥3x threshold)

### FFF Engine
Filesystem operations use `fff-search` (not `std::fs::read_dir`):
- Traversal, `.gitignore`/`.ignore`, Git status, fuzzy path search, content grep, live watcher
- Canonical root required; results as deterministic relative paths

### Duplicate Verification Pipeline (fixed order)
```
size bucket → partial SHA-256 → full SHA-256 → group evidence
```
Categories: `same_name`, `near_name`, `same_content`, `hard_link` — with verifiable evidence/confidence

---

## 7. Testing & Verification

### Test Organization
- Unit tests: `#[cfg(test)]` modules in source files
- Integration tests: `tests/*.py` (Python) + `tests/*.rs` (Rust)
- Run single crate: `cargo test -p kept-core`
- Run all: `cargo test --workspace`
- Nextest config: `nextest.toml` (CI retries=1, 60s slow timeout)

### Benchmarks
- Component: FFF vs ripgrep (throughput, latency, memory, cold/warm cache)
- Treatment: token savings per `PASS`/`COMPRESS`/`SELECT`/`DELTA`/`REFERENCE`
- Workflow: end-to-end tasks (understand repo, fix bug, refactor, repeated inquiry)
- Client impact: context reduction %, repeated reads %, tool calls %, time-to-completion

### Golden Rules
- Every public behavior change needs focused test + CLI/MCP proof
- After CLI changes: test source archive + `just cli-smoke` from extracted archive
- `just check` must pass 100% before PR/handoff

---

## 8. Config & Environment

- **Rust toolchain:** `rust-toolchain.toml` — stable, minimal profile, rustfmt + clippy
- **Cargo config:** `.cargo/config.toml` — HTTP timeout 600s, retry 10, git CLI fetch, aliases (`fmt-check`, `test-all`, `lint`)
- **Clippy restriction lints:** `unwrap_used` and `expect_used` = warn (workspace level)
- **Line endings:** LF enforced; `just fix-eol` / `just check-eol`

---

## 9. Release & Versioning

- Workspace version in `Cargo.toml` (currently `0.3.1`)
- `just version-bump [new_version]` — increments patch by default
- Schema export: `just schema` → `schema/keyword-registry.schema.json`
- Release artifacts: `just package` (source archive)

---

## 10. Important Constraints

- **No business logic duplication across crates** — each capability has single owner
- **All public capabilities require focused tests** — no exceptions
- **Context Admission Gate:** MCP/CLI must route through Judge Engine before delivering to agents
- **Vault operations:** Git-first packages, lockfile with immutable commit SHAs, OS user-data package store
- **Thai search:** BM25 + Bigram tokenizer + synonym expansion + n-gram fuzzy candidates
- **Document IR:** `kept-doc` is library-only; converters must return typed diagnostics, not silently drop data