# TODO.md แผนงาน kept

---

## 🏆 ลำดับความสำคัญและ Gate Conditions (Priority Tiers)

*   **`P0 — Core Credibility & Intelligence Baseline`** (ความถูกต้องของ Core + Test Debt + Tree-sitter + Judge Engine)
*   **`P1 — Retrieval & Document Fidelity`** (Local Vector Cache + Hybrid Search + kept-doc Universal IR)
*   **`P2 — Ecosystem Expansion`** (kept-vault Package Manager, Notion Reconciler, TUI)

> ⚠️ **Gate Rule:** ห้ามเริ่มงานในหมวด **P2** จนกว่าทุกข้อในหมวด **P0** จะถูก Implement จริง, ผ่าน TDD Red-Green, และผ่าน `just check` 100% ครบถ้วน เพื่อป้องกัน Scope Creep และรักษาเสถียรภาพของ Core

---

## P0 — Core Credibility & Intelligence Baseline

## 1. Handoff, สัญญา และแผนงานที่ใช้งานจริง

- [ ] ปรับ `AGENTS.md` ให้มี repository map ครอบคลุม FFF filesystem engine, Tree-sitter AST, Context Registry และ Judge Engine
- [ ] ระบุ ownership ตามโค้ดจริง: `kept-core` (Observation/Judge/Search), `kept-cli` (Command surface), `kept-mcp` (MCP service), `kept-doc` (Document conversion library)
- [ ] เพิ่มงานที่เริ่มจริงลง `TODO.md` เป็น checkbox ระดับ behavior ไม่รวม report หรือ ticket ซ้ำซ้อน
- [ ] อัปเดต `SPEC.md` เฉพาะ public CLI/MCP/package contract ที่ implementation และ smoke proof ผ่านแล้ว
- [ ] อัปเดต `CHANGELOG.md` เฉพาะ behavior สาธารณะที่เปลี่ยนจริงใน session นั้น
- [ ] reconcile `.agents/MEMORY.md` ทุก requirement ที่แตะ พร้อม red test, focused test, command proof และเอกสารที่เปลี่ยน
- [ ] รักษา flow ทุก slice เป็น TDD red → minimal implementation → focused green → command-level proof → `just check`

## 2. Test Debt, Observation Contract, Tree-sitter และ Judge Engine ใน kept-core

### 2.0 Test Debt & Code Quality Gate (ปิดความเสี่ยงก่อนขยายระบบ)
- [ ] เพิ่ม Unit Tests ครอบคลุม cascading scope/naming logic ใน `crate/kept-core/src/policy.rs` โดยตรง (เป้าหมาย ≥ 20 unit test cases)
- [ ] แยก Unit Tests ตรงใน `crate/kept-core/src/scanner/duplicate.rs` และ `mutation.rs` เพื่อทดสอบ execute & rollback simulation ระดับฟังก์ชัน
- [ ] ตั้งค่า Restriction Lints ใน `clippy.toml` หรือ root crate ให้เตือน `clippy::unwrap_used` และ `clippy::expect_used` เป็น warning เพื่อควบคุมและทยอยลดจุด panic ใน production code

### 2.1 Foundation: Observation, Identity & Evidence Data Model (เสร็จแล้ว - Layer 0 Contract)
- [x] ทำ TDD red สำหรับ `Target` URI parser และ format invariants (`file://`, `symbol://`, `search://`, `context://`) ใน `crate/kept-core/tests/observation_contract.rs`
- [x] สร้าง `Target` enum และ URI parser (`crate/kept-core/src/observation/target.rs`): parse canonical URI, resolve schemes, deterministic display
- [x] สร้าง `Revision` และ `ContentIdentity` models (`crate/kept-core/src/observation/identity.rs`): แยก mtime/version จาก content hash (BLAKE3/SHA-256)
- [x] สร้าง `Source`, `Evidence`, `Provenance`, `Observation` struct types (`crate/kept-core/src/observation/types.rs`): รองรับ serialization/deserialization แบบ deterministic
- [x] รัน focused contract tests ยืนยัน Observation model serialization และ identity invariants ผ่าน 100%

### 2.2 FFF Acquisition & Filesystem Engine (เสร็จแล้ว - File Acquisition Primitive)
- [x] ทำ TDD red สำหรับ fixture ที่มี Git repository, `.gitignore`, `.ignore`, hidden files, `node_modules`, `venv`, `.venv`, `__pycache__`, `target`, symlink, binary file, modified file และ untracked file
- [x] เขียน test ยืนยันว่า scan คืน file set ตาม FFF ignore semantics และ path ทุกตัวเป็น relative path แบบ deterministic
- [x] เขียน test ยืนยัน metadata ที่ kept ต้องใช้: path, name, extension, size, modified time, binary state และ Git status
- [x] ขยาย `crate/kept-core/src/scanner/fff.rs` เพิ่ม FFF acquisition layer (`look` สำหรับ outline/metadata vs `view` สำหรับ content read)
- [x] ใช้ FFF mode สำหรับ agent, content indexing, Git status cache และ ignore engine ใน adapter
- [x] สร้าง typed errors สำหรับ FFF initialization failure, invalid root, scan timeout และ index-not-ready
- [x] ขยาย `FileRecord` ให้เก็บ `isBinary` และ `gitStatus` แบบ backward-compatible
- [x] แทน `std::fs::read_dir` recursive traversal ใน `scanner/scan.rs` ด้วย FFF adapter

### 2.3 Symbol Extraction & Structural Acquisition (Tree-sitter AST)
- [ ] ทำ TDD red ทดสอบความแม่นยำของ symbol parsing ใน `source_graph.rs` ด้วย fixtures:
  - Multi-line function signatures
  - String literals และ Block comments ที่มีคำว่า `"fn "` หรือ `"struct "`
  - Generic types, Traits, Enums, Type Aliases และ Macros
- [ ] นำ `tree-sitter` และ `tree-sitter-rust` เข้ามาใน `Cargo.toml` เพื่อ parse AST สำหรับภาษา Rust
- [ ] ออกแบบ fallback หรือ multi-language AST parser สำหรับภาษาสำคัญอื่นๆ
- [ ] ผูกผลลัพธ์ AST เข้ากับ `StructurePayload` และ `StructureOutlineItem` ของ `Observation`

### 2.4 Context Registry & Judge Engine Baseline (SQZ Integration)
- [ ] Vendor โมดูล `cmd_formatters` และ `delta_encoder` จาก `references/campbellr/sqz` เข้าสู่ `kept-core` / `kept-judge`
- [ ] ทำ TDD red สำหรับ `ContextRegistry` เก็บ snapshot revision และ session history ลง SQLite backend เพื่อป้องกัน duplicate context reads
- [ ] สร้าง `ContextRegistry` (`crate/kept-core/src/context/registry.rs`) รองรับ `context://<target>@<rev>` tracking
- [ ] ทำ TDD red สำหรับ `Judge` admission decisions: `Pass`, `Reference`, `Delta`, `Compress`, `Warn`, `Block`
- [ ] สร้าง `JudgeEngine` ประเมิน Decision ตาม policy:
  - New observation → `PASS` (หรือ `SELECT` symbol)
  - Unchanged revision → `REFERENCE` (คืน token อ้างอิง ไม่ส่งเนื้อหาซ้ำ)
  - Changed revision → `DELTA` (ส่งเฉพาะ diff/changed symbols)
  - Redundant repeated reads → `WARN` / `BLOCK`

## 3. ย้าย kept scan, find, review, duplicates และ incremental index ไปใช้ FFF-backed index

- [ ] ทำ TDD red สำหรับ `kept scan <root>` ให้สร้าง snapshot จาก FFF file inventory และรายงาน file count, total size, issues และ refresh delta แบบ deterministic
- [ ] แก้ `create_or_refresh_scan` ให้รอ FFF initial scan/index readiness ก่อนเขียน snapshot
- [ ] รักษา `kept scan --output`, `--json`, default user-state snapshot และ portable snapshot ให้ทำงานต่อเนื่อง
- [ ] ทำ TDD red ให้ `kept scan` เคารพ `.gitignore`; `--include-hidden` ต้องไม่ bypass ignore rule
- [ ] ทำ TDD red สำหรับ `kept find --query` ด้วย typo path query แล้ว assert ว่าคืน fuzzy match, score และ Git status
- [ ] รักษา `kept find --type`, `--name`, `--path`, `--min-size`, `--max-size`, `--after`, `--before` และ FQL facts เดิม
- [ ] ให้ FFF คัด candidate paths ก่อน แล้วให้ `FilterSet` ตัดสิน extension, size, modified time, duplicate facts และ custom filters ต่อ
- [ ] ทำ TDD red สำหรับ `kept review` ให้ใช้ FFF-backed snapshot เดียวกับ scan และยังแสดง space summary, scan issue, integrity finding และ naming finding ได้
- [ ] คง duplicate verification `size bucket → partial SHA-256 → full SHA-256` ไว้ โดยใช้ candidate records จาก FFF-backed snapshot
- [ ] ทำ persistent incremental index optimization โดยใช้ FFF watcher events และ refresh plan

## 4. รวม FFF, filesystem tools และ MCP contract เข้า bl1nk-kept-mcp

- [ ] นำ FFF manager และ Context Admission เข้า `kept-mcp`
- [ ] ปรับ MCP tool ให้ส่งมอบ Context ผ่าน `JudgeEngine` เสมอ
- [ ] ทำ integration test สำหรับ MCP Client รับผลลัพธ์ `PASS`, `REFERENCE`, `DELTA`

## 5. ตรวจ lifecycle จริง, dogfooding และ Thai Bigram correctness

- [ ] ตรวจจับ regression บน large workspace (10K-100K files)
- [ ] ยืนยัน Thai bigram search และ BM25 ranking บน corpus จริง

---

## P1 — Retrieval & Document Fidelity

## 6. ทำ Hybrid Semantic Search และ Reranking Engine

- [ ] ออกแบบ Disk-backed Vector Cache เพื่อเก็บ embeddings ลงดิสก์โดยไม่กิน RAM (ประเมิน `zvec` / DiskANN backend)
- [ ] รวม FTS + BM25 + Vector เข้ากับ Single Query Planner
- [ ] ทำ TDD red สำหรับ reranker fallback เมื่อ network provider ภายนอกขัดข้อง

## 7. Universal Document IR และ Offline Converter ใน kept-doc

- [ ] ขยาย converter สำหรับ Markdown, NFM, DOCX, PDF, HTML เข้าสู่ Universal IR
- [ ] สร้าง document outline/slice เพื่อส่งต่อเข้า Observation Layer

---

## P2 — Ecosystem Expansion (ต้องผ่าน Gate P0 ก่อนเริ่ม)

## 8. สร้าง crate kept-vault สำหรับ Git-first vault package manager

- [ ] สร้าง manifest `vault.toml` และ lockfile `vault.lock`
- [ ] เพิ่ม Git package resolver และ content-addressable package store

## 9. เพิ่ม package script runtime และ Vault lifecycle commands

- [ ] ทำ sandboxed script execution สำหรับ vault hooks
- [ ] เพิ่มคำสั่ง `kept vault init`, `install`, `update`, `test`

## 10. Notion Safe Sync และ TUI Interface

- [ ] ทำ Notion safe reconciler (Sync สองทางแบบมี Transaction Ledger)
- [ ] สร้าง TUI Dashboard สำหรับดูสถานะ Context และ Vault
