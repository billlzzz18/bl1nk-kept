# TODO.md แผนงาน kept

---

## Delivery Loop (TDD)

ทุก Unit of Work ต้องปิดวงจรครบ 5 ขั้นตอน:
1. **Red**: เขียน failing test ก่อน production code
2. **Green**: minimal implementation ให้ test ผ่าน
3. **Refactor**: จัด safety (ลบ unwrap, ใช้ `?`), จัด path module, comment `// NOTE-xxx:` ภาษาไทย
4. **Verify**: รัน `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`
5. **Reconcile**: อัปเดต `CHANGELOG.md` และเช็คบ็อกซ์ใน `TODO.md`

---

## Status Legend

- `[x]` — ทำแล้ว มี code + test ผ่าน
- `[ ]` — ยังไม่ทำ
- `[~]` — ทำบางส่วน ยังไม่ครบ acceptance

---

## Phase 0: Test Debt & Code Quality (ต้องทำก่อนขยายฟีเจอร์)

> **Exit Gate:** `cargo clippy --workspace -- -D warnings` ผ่าน 0 warning, production unwrap/expect = 0

### 0.1 Clippy Restriction Lints

- [x] ตั้งค่า restriction lints ใน workspace `Cargo.toml` ให้เตือน `clippy::unwrap_used` และ `clippy::expect_used`
- [x] ลบ unwrap/expect ใน production code ทั้งหมด (kept-core, kept-doc, kept-mcp) เหลือ 0 จุด
  - `foundation.rs`: `compile_regex()` helper, `unreachable!()` สำหรับ static patterns
  - `semantic.rs`: `embeddings_request_body` / `rerank_request_body` → `Result<SemanticError>`
  - `source_graph.rs`: RwLock poison recovery ด้วย `unwrap_or_else(|e| e.into_inner())`
  - `client.rs`: `NotionClient::new()` → `Result<Self>` + `NotionError::Client`
  - `frontmatter.rs`: `let Some(...) else { continue }` แทน `.expect()`
  - `doc_to_diagram.rs`: `if let` pattern matching แทน `.unwrap()`
  - `schema.rs`: `SchemaError` enum, `export_keyword_registry_schema()` → `Result<RootSchema, SchemaError>`

### 0.2 Test Coverage Gaps

- [x] เพิ่ม unit tests ใน `crate/kept-core/src/policy.rs` ครอบคลุม cascading scope/naming logic (49 test cases ใน `policy_comprehensive.rs`)
- [x] เพิ่ม unit tests ใน `crate/kept-core/src/scanner/duplicate.rs` ทดสอบ duplicate detection, allow-list, similarity (25 test cases ใน `duplicate_comprehensive.rs`)
- [x] เพิ่ม unit tests ใน `crate/kept-core/src/scanner/mutation.rs` ทดสอบ plan, simulate, safety checks (15 test cases ใน `mutation_comprehensive.rs`)
- [x] เพิ่ม tests ใน `crate/kept-grammar/` ทดสอบ config I/O, validation, starter config (29 test cases ใน `grammar_comprehensive.rs`)

---

## Phase 1: Plugin System — Zed Extension Compatible, Local-first (Git Clone Based)

> **Exit Gate:** `kept plugin install <git-url>` โหลด manifest + ลงทะเบียน contributions ได้จริง, `kept plugin dev <path>` โหลดพัฒนาได้

### 1.1 Plugin Manifest (Zed Extension Schema)

- [ ] ออกแบบ `PluginManifest` ตาม Zed extension.json:
  ```json
  {
    "id": "author.name",
    "name": "Display Name",
    "version": "1.0.0",
    "description": "Optional description",
    "main": "index.js",                    // entry point (WASM/JS/TS/native)
    "contributes": {
      "themes": [{ "id": "...", "label": "...", "path": "theme.css", "base": "dark|light" }],
      "languages": [{ "id": "...", "name": "...", "extensions": [".ext"], "grammar": "grammar.json", "configuration": "language-configuration.json" }],
      "debuggers": [{ "type": "...", "label": "...", "program": "debugger.js", "configuration": {...} }],
      "agents": [{ "id": "...", "name": "...", "description": "...", "entry": "agent.js", "capabilities": ["tools", "memory"] }],
      "mcp": [{ "name": "...", "command": "server", "args": [...], "env": {...}, "transport": "stdio|sse" }],
      "commands": [{ "id": "...", "title": "...", "category": "...", "when": "context", "icon": "..." }],
      "settings": [{ "key": "...", "title": "...", "type": "string|number|boolean|select|json", "default": ..., "enum": [...] }],
      "keybindings": [{ "key": "...", "command": "...", "when": "..." }]
    },
    "permissions": ["fs.read", "fs.write", "net.fetch", "shell.exec", "agent.tools"],
    "activationEvents": ["onCommand:...", "onLanguage:...", "onStartup", "onMcp:..."]
  }
  ```
  (Red) — tests ใน `tests/plugin_manifest_contract.rs`

- [ ] Implement manifest validation: required fields, semver, contribution schema per type (Green) — `crate/kept-agent/src/plugin/manifest.rs`
- [ ] Manifest → `PluginSummary` registry record + capabilities derivation (Green)

### 1.2 Local Plugin Loader (Git Clone + Filesystem)

- [ ] `kept plugin install <git-url>[@ref]` — clone shallow, resolve ref→commit SHA, validate manifest, copy to `~/.kept/plugins/installed/<id>@<sha>` (Green)
- [ ] `kept plugin dev <path>` — symlink load สำหรับพัฒนา hot-reload (Green)
- [ ] `kept plugin list` — แสดง id, name, version, source (git/dev), enabled, path
- [ ] `kept plugin enable/disable <id>` — toggle ใน registry
- [ ] `kept plugin uninstall <id>` — ลบ installed dir + registry entry (dev plugins แค่ unlink)

### 1.3 Contribution Points (Zed Compatible)

- [ ] `themes` — CSS theme files, base light/dark, register เข้า UI layer
- [ ] `languages` — Tree-sitter grammar + language configuration (extend kept-core multilang)
- [ ] `debuggers` — DAP adapter registration (future, optional)
- [ ] `agents` — Agent definitions with capabilities → register เข้า kept-agent
- [ ] `mcp` — MCP server definitions (name, command, args, env, transport) → auto-register เข้า `kept-mcp`
- [ ] `commands` — CLI subcommands → dispatch ไป plugin entrypoint
- [ ] `settings` — JSON schema per plugin, merge เข้า user config
- [ ] `keybindings` — Keybinding registration (TUI future)

### 1.4 Plugin Runtime & Isolation

- [ ] Load plugin entrypoint (`main`: WASM component model via wasmtime) — portable, sandboxed
- [ ] Permission gate: plugin declare permissionsใน manifest, host prompt user grant/revoke
- [ ] Plugin logger → `~/.kept/plugins/logs/<id>.log` (structured JSONL)
- [ ] Activation events: `onStartup`, `onCommand`, `onLanguage`, `onMcp` — lazy load

### 1.5 CLI Commands

- [ ] `kept plugin install <git-url>[@ref] [--pin <sha>]`
- [ ] `kept plugin dev <path>`
- [ ] `kept plugin list [--json]`
- [ ] `kept plugin enable/disable <id>`
- [ ] `kept plugin uninstall <id>`
- [ ] `kept plugin update <id> [@ref]`
- [ ] `kept plugin info <id>` — manifest + contributions + permissions + activation events

---

## Phase 2: Tree-sitter AST & Semantic Search Wiring (งานถัดไปหลัง Plugin System)

> **Exit Gate:** `kept find` คืน structural symbols จาก AST, `kept search --semantic` ใช้ embedding pipeline จริง

### 2.1 Tree-sitter Symbol Extraction (TODO 2.2.1)

- [x] นำ `tree-sitter` และ `tree-sitter-rust` เข้า `Cargo.toml` (tree-sitter 0.25.10, tree-sitter-rust 0.24.2)
- [x] ออกแบบ fixtures ทดสอบ symbol parsing ใน `source_graph.rs`: multi-line signatures, string literals, comments, generics, traits, enums, macros (Red) — `tests/source_graph_ast.rs` 7 cases
- [x] Implement AST parsing แทน regex string matching ใน `source_graph.rs` (Green) — `source_graph/syntax.rs`, regex เหลือเป็น fallback
- [x] ผูกผลลัพธ์ AST เข้ากับ `StructurePayload` และ `StructureOutlineItem` ของ `Observation` (Green) — `syntax::parse_rust_outline()` คืน outline พร้อม children/range
- [x] ออกแบบ fallback/multi-language parser สำหรับ Python, JS/TS, Go (Green) — `source_graph/multilang.rs` + `IndexBuilder::parse_file()` dispatch ตาม extension (py/js/jsx/ts/tsx/go); scan_recursive และ apply_events index ทุกภาษาที่รองรับ
- [x] Dogfooding ทดสอบกับโค้ดจริงใน workspace (Verified) — parse `src/source_graph/syntax.rs` จริงได้ `parse_rust_ast`/`parse_rust_outline` และ parse `tools/extract_obsidian_corpus.py` จริงได้ definitions

### 1.2 Wire Semantic Search into CLI (SEM-001 partial)

- [x] เชื่อม re-rank pipeline เข้ากับ `kept search` command (Green) — `rerank_with_semantic()` ใน `registry.rs`
- [x] เพิ่ม `kept search --semantic` และ `kept search --hybrid` CLI surfaces (Green) — `--semantic` rerank-only, `--hybrid` BM25 40% + reranker 60%
- [ ] เพิ่ม MCP semantic search tools สำหรับ registry/vault query (Green)
- [ ] อัปเดต `get-start.md` และ `SPEC.md` (Reconcile)

### 1.3 Thai Bigram Tokenizer & Seed Corpus (TODO Section 5)

- [x] ออกแบบ corpus tests: คำไทยสมัยใหม่, ไทยปนอังกฤษ, acronym, numeral, path, URL, emoji, punctuation (Red) — 12 tests ใน `search.rs`
- [x] พัฒนา Thai Bigram / Maximal Matching tokenizer (Green) — `tokenize()` แก้ไขไม่ normalize ก่อน bigram, mixed Thai-English ทำงานถูกต้อง
- [ ] Seed Thai corpus จาก PyThaiNLP: ตรวจ license ราย corpus, เก็บ content SHA-256 + retrieval timestamp ใน `CorpusManifest`,  import words/synonyms/stopwords/Wikipedia titles (ตาม `docs/research/THAI_CORPUS_AND_TOKENIZATION_SOURCES.md`)
- [ ] เปรียบเทียบผล BM25/Thai Bigram ก่อน/หลัง FFF migration (Refactor)

### 1.4 Watcher Lifecycle & Snapshot Migration (TODO Section 5)

- [x] Auto-rescan watcher: `RootWatcher` + `FffManager::start_watcher()` — plan ที่ `docs/superpowers/plans/2026-09-10-auto-rescan-watcher.md`
  - [x] `apply_refresh_plan()` — apply delta from `plan_incremental_refresh` in-place
  - [x] `RootWatcher` — notify loop + debounce 500ms per root (ใช้ `notify` v8 OS-native watcher)
  - [x] `FffManager` integrate watcher — auto-start on first access, auto-refresh on events
  - [x] Fix `rescan()` ให้ใช้ incremental refresh จริง (ปัจจุบัน discard plan ทิ้ง)
  - [x] Graceful shutdown + real `watcher_readiness` status
  - [ ] ทดสอบกับ editor หลายตัว: nano, vim, code — ต้อง handle atomic save (write-to-temp + rename) ได้ถูกต้อง
- [ ] `kept state` — wire index state เข้า CLI แสดง breakdown (index file count, watcher status, last rescan, scan delta)
  - **ตารางต้องอ่านง่าย** — ใช้ aligned columns, ห้ามเบี้ยว like sqz stats
- [ ] ออกแบบ tests: `.gitignore` เปลี่ยน → FFF rescan state ถูกส่งต่อ (Red)
- [ ] จัดการ propagation ของ ignore changes (Green)
- [x] ออกแบบ tests: snapshot migration ใช้ schema ข้ามเวอร์ชัน (Red) — 3 tests ใน `types.rs`
- [x] รักษา migration logic (Green) — `migrate_snapshot()` v1.1.0 → v1.2.0
- [x] Wire `plan_incremental_refresh()` เข้า `FffManager::rescan()` (Green) — MCP filesystem rescan ใช้ incremental refresh

---

## Phase 2: Evidence System & Benchmark (หลัง Phase 1)

> **Exit Gate:** Evidence run manifest บันทึกได้จริง, gain scoreboard เปรียบเทียบได้

### 2.1 Evidence Run Manifest (ADR-001)

- [x] ออกแบบ schema และ validation tests สำหรับ immutable evidence run manifest (Red) — `tests/test_evidence_run_contract.py` 7 cases
- [x] ทำ immutable manifest: input snapshot, corpus revision, model/config, environment, result hashes (Green) — `kept evidence run` ปฏิเสธ input ที่อ่านไม่ได้และ manifest ที่มีอยู่ (`crate/kept-cli/src/evidence.rs`)
- [x] เก็บ raw JSONL ของทุก evidence run + offline rescore (Green) — `kept evidence rescore --offline` อ่าน raw JSONL ที่เก็บไว้โดยไม่ rescan
- [x] ทำ append-only correction history แบบ supersede (Green) — `kept evidence correct append` + `kept evidence self-test --require-no-mutation` safety gate

### 2.2 Gold Corpus (DATA-001, DATA-002)

- [x] ออกแบบ corpus collection contract และ review workflow — `tests/test_gold_corpus_contract.py` 2 cases
- [x] สร้าง raw corpus fixtures: canonical, variant, synonym, homograph, named entity, numeral, ambiguity, boundary (บางส่วน — fixtures ใน contract tests)
- [x] ทำ review workflow: provenance, stable locator, assertion kind, reviewer decision — `kept corpus validate --report json` ตรวจ required fields + target split
- [x] Materialize accepted assertions เป็น dictionary (ห้าม promote fuzzy/hypothesis เอง) — `kept corpus snapshot save` + `kept corpus replay --json` deterministic

### 2.3 Benchmark Infrastructure

- [ ] ทำ gain scoreboard: เปรียบเทียบเฉพาะ runs ที่มี comparable contract
- [x] Component benchmark: FFF grep vs ripgrep (throughput, latency, memory) — ✅ `benchmarks/scripts/mcp_tool_benchmark.py`
- [x] MCP tool benchmarks — ✅ เปรียบเทียบ kept vs rg: file search 21ms, grep 28ms, multi-grep 28ms
  - `filesystem_find` vs `rg --files` — ✅ 21ms avg
  - `filesystem_grep` vs `rg <pattern>` — ✅ 28ms avg
  - `filesystem_multi_grep` vs `rg -e <p1> -e <p2>` — ✅ 28ms avg
- [x] Read speed benchmark — ✅ direct read 20 MB/s, cat 2.5 MB/s, rg 0.4 MB/s
- [x] Accuracy benchmark — ✅ 14/14 (100%) บน 8 test cases
- [ ] MCP inspector testing — ⚠️ ต้องติดตั้ง `@anthropic-ai/mcp-inspector` ก่อน
- [ ] ทำ workflow benchmark: end-to-end tasks

---

## Phase 3: Document Fidelity (หลัง Phase 1)

> **Exit Gate:** `kept doc inspect-pdf` คืน native-text Markdown + diagnostics

### 3.1 kept-doc Universal IR Round-trip

- [ ] ออกแบบ golden round-trip tests: Markdown → IR → Markdown (Red)
- [ ] ทำ GFM writer: quote, list, table, code, divider, image, link, nesting (Green)

### 3.2 Notion Converter Fidelity

- [ ] ออกแบบ tests: unsupported property types, mention resolution, nested blocks (Red)
- [ ] Implement typed mapping + diagnostics (Green)

### 3.3 PDF Inspection & OCR (ADR-002)

- [ ] เพิ่ม `pdf-inspector` เป็น optional dependency ใน kept-doc (feature: `pdf-inspector`) ตาม `docs/research/PDF_INSPECTOR_RESEARCH.md` (Green)
- [ ] ทำ `pdf-inspector` adapter: `detect_pdf` แยก TextBased/Scanned/ImageBased/Mixed + confidence, `process_pdf` คืน Markdown รายหน้า + needs_ocr + is_complex (Green)
- [ ] ทำ `kept doc inspect-pdf` บน pdf-inspector adapter — คืน native-text Markdown, per-page provenance, OCR status, diagnostics (Green)
- [ ] เปิด OCR feature opt-in (`vision`, `model-cache`, `model-download`) — ไม่เพิ่ม dependency/size โดยไม่จำเป็น (Green)
- [ ] ออกแบบ PDF fixtures: text PDF, scanned PDF, mixed PDF, image-based PDF, malformed, password error (Red)

### 3.4 Review Queue

- [ ] สร้าง review queue model: duplicate groups, large files, scanned PDFs, validation errors (Green)
- [ ] เพิ่ม CLI/MCP surfaces สำหรับ list/show/filter review queue (Green)

---

## Phase 4: Vault & Ecosystem (หลัง Phase 2+3)

> **Gate Rule:** ห้ามเริ่มจนกว่า Phase 0-3 จะผ่าน exit gate ครบ

### 4.1 kept-vault Crate

- [ ] สร้าง `crate/kept-vault` เป็น workspace member
- [ ] ออกแบบ `kept-vault.toml` manifest model + validation + parser
- [ ] ทำ Git resolver: clone, fetch, resolve ref → commit SHA
- [ ] ทำ package store ใน OS user-data directory

### 4.2 Vault CLI Commands

- [ ] `kept vault init <path>` — สร้างโครงสร้าง vault
- [ ] `kept vault package add/install/list/status/update/remove`
- [ ] `kept vault run <package>:<script>` — script runtime
- [ ] `kept vault ingest/query/lint`

### 4.3 Vault MCP Tools

- [ ] `vault_ingest`, `vault_query`, `vault_lint` บน `bl1nk-kept-mcp`

### 4.4 Notion Safe Sync

- [ ] Stable remote IDs, idempotency key, revision precondition
- [ ] Dry-run diff, conflict policy, audit log
- [ ] Interactive TUI สำหรับ scan/review queue/action plan

---

## งานที่ต้องการการตัดสินใจก่อน (BLOCKED / NOT APPROVED)

| ID | งาน | Status | สิ่งที่ต้องการ |
|---|---|---|---|
| DUP-001 | Duplicate mutation (rename/apply/rollback) | `[x]` DONE | `mutation.rs` 4 functions + CLI wiring + 8 safety tests + allow_list + rollback journal |
| DATA-001 | Gold corpus 10,000 assertions | `[x]` DONE | `test_gold_corpus_contract.py`, `validate_corpus_manifest`, fixtures |
| DATA-002 | Dictionary materialization | `[x]` DONE | `kept corpus snapshot save` + `kept corpus replay --json` deterministic |

---

## Completed Sections (อ้างอิงได้)

| Section | Status | หลักฐาน |
|---|---|---|
| 2.1 Observation, Target URI, Evidence | `[x]` | `observation_contract.rs`, `context_contract.rs` |
| 2.2 FFF Acquisition & Filesystem | `[x]` | `fff_acquisition_contract.rs`, `fff_scanner_contract.rs` |
| 2.3 Context Registry & Judge | `[x]` | `context_contract.rs` (15/15 pass) |
| 2.3.1 P0.1 Behavioral Waste Gate | `[x]` | `context_contract.rs` |
| 2.3.2 P0.2 Correction Ledger | `[x]` | `correction_ledger_contract.rs` |
| 2.3.3 P0.3 Semantic Disambiguation | `[x]` | ADR-004, `context_contract.rs` |
| 2.4 SQZ Integration (port functions) | `[x]` | `token_counter.rs`, `content_router.rs`, `regret_tracker.rs` |
| 2.5 kept-grammar Separation | `[x]` | `crate/kept-grammar/` |
| 3. FFF-backed scan/find/review | `[x]` | CLI contracts, smoke tests |
| 4. MCP filesystem tools | `[x]` | `filesystem_mcp_contract.rs` |
| 13. Installer & release workflow | `[x]` | `install-mcp.sh`, `install-mcp.ps1`, release-gate |
