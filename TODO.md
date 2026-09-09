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

## Phase 1: Tree-sitter AST & Semantic Search Wiring (งานถัดไปหลัง Phase 0)

> **Exit Gate:** `kept find` คืน structural symbols จาก AST, `kept search --semantic` ใช้ embedding pipeline จริง

### 1.1 Tree-sitter Symbol Extraction (TODO 2.2.1)

- [ ] นำ `tree-sitter` และ `tree-sitter-rust` เข้า `Cargo.toml`
- [ ] ออกแบบ fixtures ทดสอบ symbol parsing ใน `source_graph.rs`: multi-line signatures, string literals, comments, generics, traits, enums, macros (Red)
- [ ] Implement AST parsing แทน regex string matching ใน `source_graph.rs` (Green)
- [ ] ผูกผลลัพธ์ AST เข้ากับ `StructurePayload` และ `StructureOutlineItem` ของ `Observation` (Green)
- [ ] ออกแบบ fallback/multi-language parser สำหรับ Python, JS/TS, Go (Green)
- [ ] Dogfooding ทดสอบกับโค้ดจริงใน workspace (Verified)

### 1.2 Wire Semantic Search into CLI (SEM-001 partial)

- [ ] เชื่อม re-rank pipeline เข้ากับ `kept search` command (Green)
- [ ] เพิ่ม `kept search --semantic` และ `kept search --hybrid` CLI surfaces (Green)
- [ ] เพิ่ม MCP semantic search tools สำหรับ registry/vault query (Green)
- [ ] อัปเดต `get-start.md` และ `SPEC.md` (Reconcile)

### 1.3 Thai Bigram Tokenizer (TODO Section 5)

- [ ] ออกแบบ corpus tests: คำไทยสมัยใหม่, ไทยปนอังกฤษ, acronym, numeral, path, URL, emoji, punctuation (Red)
- [ ] พัฒนา Thai Bigram / Maximal Matching tokenizer (Green)
- [ ] เปรียบเทียบผล BM25/Thai Bigram ก่อน/หลัง FFF migration (Refactor)

### 1.4 Watcher Lifecycle & Snapshot Migration (TODO Section 5)

- [ ] ออกแบบ tests: watcher events `created/modified/removed` → index query เห็นล่าสุดโดยไม่สร้าง instance ใหม่ (Red)
- [ ] เชื่อม watcher events เข้า index query pipeline (Green)
- [ ] ออกแบบ tests: `.gitignore` เปลี่ยน → FFF rescan state ถูกส่งต่อ (Red)
- [ ] จัดการ propagation ของ ignore changes (Green)
- [ ] ออกแบบ tests: snapshot migration ใช้ schema ข้ามเวอร์ชัน (Red)
- [ ] รักษา migration logic (Green)

---

## Phase 2: Evidence System & Benchmark (หลัง Phase 1)

> **Exit Gate:** Evidence run manifest บันทึกได้จริง, gain scoreboard เปรียบเทียบได้

### 2.1 Evidence Run Manifest (ADR-001)

- [ ] ออกแบบ schema และ validation tests สำหรับ immutable evidence run manifest (Red)
- [ ] ทำ immutable manifest: input snapshot, corpus revision, model/config, environment, result hashes (Green)
- [ ] เก็บ raw JSONL ของทุก evidence run + offline rescore (Green)
- [ ] ทำ append-only correction history แบบ supersede (Green)

### 2.2 Gold Corpus (DATA-001, DATA-002)

- [ ] ออกแบบ corpus collection contract และ review workflow
- [ ] สร้าง raw corpus fixtures: canonical, variant, synonym, homograph, named entity, numeral, ambiguity, boundary
- [ ] ทำ review workflow: provenance, stable locator, assertion kind, reviewer decision
- [ ] Materialize accepted assertions เป็น dictionary (ห้าม promote fuzzy/hypothesis เอง)

### 2.3 Benchmark Infrastructure

- [ ] ทำ gain scoreboard: เปรียบเทียบเฉพาะ runs ที่มี comparable contract
- [ ] ทำ component benchmark: FFF grep vs ripgrep (throughput, latency, memory)
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

### 3.3 PDF Inspection (ADR-002)

- [ ] ทำ `kept doc inspect-pdf` บน `PdfAdapter` ที่มีอยู่ (Green)
- [ ] คืน native-text Markdown, per-page provenance, diagnostics (Green)
- [ ] ออกแบบ PDF fixtures: text, scanned, malformed, password error (Red)

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
| DUP-001 | Duplicate mutation (rename/apply/rollback) | `NOT APPROVED` | Director approval ก่อน implement |
| DATA-001 | Gold corpus 10,000 assertions | `MISSING` | Corpus collection contract |
| DATA-002 | Dictionary materialization | `MISSING` | Review workflow + provenance rules |

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
