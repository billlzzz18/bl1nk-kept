# TODO.md แผนงาน kept

---

## 🏆 ลำดับความสำคัญและ Gate Conditions (Priority Tiers)

* **`P0 — Core Credibility & Intelligence Baseline`**
  * Section 1: Handoff, สัญญา และ Governance ที่ใช้งานจริง
  * Section 2: Observation Contract, Evidence Model, AST และ Judge Engine ใน kept-core
* **`P1 — Filesystem & MCP Pipeline`**
  * Section 3: ย้าย kept scan, find, review, duplicates และ incremental index ไปใช้ FFF-backed index
  * Section 4: รวม FFF, filesystem tools และ MCP contract เข้า bl1nk-kept-mcp
* **`P2 — Retrieval, Evidence & Search Engine`**
  * Section 5: ตรวจ lifecycle จริง, dogfooding และ Thai Bigram correctness
  * Section 6: ทำ Hybrid Semantic Search และ Reranking Engine
  * Section 7: ทำ Evidence run, correction loop, gold corpus และ benchmark จริง
* **`P3 — Document Fidelity & Universal IR`**
  * Section 8: เพิ่ม kept-doc fidelity, PDF inspection และ review queue
* **`P4 — Ecosystem Expansion & Automation`**
  * Section 9: สร้าง crate kept-vault สำหรับ Git-first vault package manager
  * Section 10: เพิ่ม package script runtime และ Vault lifecycle commands
  * Section 11: ทำ vault knowledge package, query และ MCP agent tools
  * Section 12: ทำ Notion Safe Sync, reconciler และ operator experience
  * Section 13: Installer Scripts & Packaging Distribution

> ⚠️ **Gate Rule:** ห้ามเริ่มงานในหมวด **P4** จนกว่าทุกข้อในหมวด **P0** และ **P1** จะถูก Implement จริง, ผ่านวงจร TDD ครบถ้วน (Red → Green → Refactor → Dogfood/Verified → Reconcile Docs), และผ่าน `just check` 100% ครบถ้วน

---

## 🧭 มาตรฐานวงจรการส่งมอบงาน (TDD & Delivery Loop)

เพื่อให้การทำงานมี Accountability ชัดเจนและไม่ปิดงานค้างคา แต่ละ Unit of Work ต้องปิดวงจรให้ครบ 5 ขั้นตอน:
1. **Red**: เขียน failing test / test fixtures / invariants กำกับก่อนแตะ production code
2. **Green**: เขียน minimal implementation ให้ test ผ่านโดยไม่ over-engineer
3. **Refactor**: จัดการ safety (ลบ unwrap, ใช้ `?`, ป้องกัน panic), จัด path module และ comment ด้วย `// NOTE-xxx:` ภาษาไทย
4. **Dogfood & Verify**: รันลองใช้กับ workspace จริง / CLI execution / focused suite / `just check`
5. **Reconcile**: อัปเดต `SPEC.md`, `CHANGELOG.md`, `.agents/MEMORY.md` และเช็คบ็อกซ์ใน `TODO.md`

---

## 🔴 [P0] — Core Stability & Quality Baseline (ฐานรากต้องแน่นก่อน)

### 1. Handoff, สัญญา และ Governance ที่ใช้งานจริง

* [ ] ปรับ `AGENTS.md` เมื่อเริ่ม implementation ให้มี repository map ของ FFF filesystem engine, `kept-vault`, package manifest, lockfile, package store, script runtime และ MCP tool modules ที่สร้างจริง
* [ ] ระบุ ownership ตามโค้ดจริง: `kept-core` เป็น filesystem/vault domain, `kept-cli` เป็น command surface, `kept-mcp` เป็น long-running MCP service, `kept-doc` เป็น document conversion library
* [ ] เพิ่มงานที่เริ่มจริงลง `TODO.md` เป็น checkbox ระดับ behavior ไม่รวม report หรือ ticket ซ้ำซ้อน
* [ ] อัปเดต `SPEC.md` เฉพาะ public CLI/MCP/package contract ที่ implementation และ smoke proof ผ่านแล้ว
* [ ] อัปเดต `CHANGELOG.md` เฉพาะ behavior สาธารณะที่เปลี่ยนจริงใน session นั้น
* [ ] reconcile `.agents/MEMORY.md` ทุก requirement ที่แตะ พร้อม red test, focused test, command proof และเอกสารที่เปลี่ยน
* [ ] รักษา flow ทุก slice เป็น TDD red → minimal implementation → focused green → command-level proof → `just check`

### 2. Observation Contract, Evidence Model, AST และ Judge Engine ใน kept-core

#### 2.0 Test Debt & Code Quality Gate (ปิดความเสี่ยงก่อนขยายระบบ)

* [ ] พัฒนา Unit Tests ครอบคลุม cascading scope/naming logic ใน `crate/kept-core/src/policy.rs` โดยตรง (เป้าหมาย ≥ 20 unit test cases) ผ่าน TDD (Red → Green → Refactor)
* [ ] แยก Unit Tests ตรงใน `crate/kept-core/src/scanner/duplicate.rs` และ `mutation.rs` เพื่อทดสอบ execute & rollback simulation ระดับฟังก์ชัน พร้อม verify ความปลอดภัย
* [ ] ตั้งค่า Restriction Lints ใน `clippy.toml` หรือ root crate ให้เตือน `clippy::unwrap_used` และ `clippy::expect_used` เป็น warning เพื่อควบคุมและทยอยลดจุด panic ใน production code
* [ ] ตรวจสอบคุณภาพและ regression ประจำรอบ: รัน `cargo clippy --workspace -- -D warnings` และ `just check`

#### 2.1 Foundation: Observation, Identity & Evidence Data Model (Layer 0 Contract)

* [x] ออกแบบและทดสอบ `Target` URI parser และ format invariants (`file://`, `symbol://`, `search://`, `context://`) ใน `crate/kept-core/tests/observation_contract.rs` (Red)
* [x] สร้าง `Target` enum และ URI parser (`crate/kept-core/src/observation/target.rs`): parse canonical URI, resolve schemes, deterministic display (Green)
* [x] สร้าง `Revision` และ `ContentIdentity` models (`crate/kept-core/src/observation/identity.rs`): แยก mtime/version จาก content hash (BLAKE3/SHA-256) (Green)
* [x] สร้าง `Source`, `Evidence`, `Provenance`, `Observation` struct types (`crate/kept-core/src/observation/types.rs`): รองรับ serialization/deserialization แบบ deterministic (Green)
* [x] Refactor จัดระเบียบ type exports และ error conversion ใน `crate/kept-core/src/observation/` ให้เป็น clean API boundary (Refactor)
* [x] รัน focused contract tests ยืนยัน Observation model serialization และ identity invariants ผ่าน 100% (Verified)

#### 2.2 FFF Acquisition & Filesystem Engine (File Acquisition Primitive)

* [x] ออกแบบ test fixture ที่มี Git repository, `.gitignore`, `.ignore`, hidden files, `node_modules`, `venv`, `.venv`, `__pycache__`, `target`, symlink, binary file, modified file และ untรacked file (Red)
* [x] เขียน test ยืนยันว่า scan คืน file set ตาม FFF ignore semantics และ path ทุกตัวเป็น relative path แบบ deterministic (Red)
* [x] เขียน test ยืนยัน metadata ที่ kept ต้องใช้: path, name, extension, size, modified time, binary state และ Git status (Red)
* [x] ขยาย `crate/kept-core/src/scanner/fff.rs` เพิ่ม FFF acquisition layer (`look` สำหรับ outline/metadata vs `view` สำหรับ content read) (Green)
* [x] ใช้ FFF mode สำหรับ agent, content indexing, Git status cache และ ignore engine ใน adapter (Green)
* [x] สร้าง typed errors สำหรับ FFF initialization failure, invalid root, scan timeout และ index-not-ready (Green)
* [x] ขยาย `FileRecord` ให้เก็บ `isBinary` และ `gitStatus` แบบ backward-compatible (Green)
* [x] version-bump `PersistentScanSnapshot` และสร้าง migration/read error สำหรับ snapshot รุ่นที่ไม่มี FFF metadata (Green)
* [x] แทน `std::fs::read_dir` recursive traversal ใน `scanner/scan.rs` ด้วย FFF adapter (Green)
* [x] เก็บ `ScanIssue` สำหรับ metadata/indexing failure โดยไม่ทำให้ผล scan ส่วนที่เข้าถึงได้หายไป และลบ traversal เดิม (Refactor)
* [x] ยืนยัน adapter tests ผ่านและไม่มี code path เรียกใช้งาน traversal implementation เดิมแล้ว (Verified)

#### 2.2.1 Symbol Extraction & Structural Acquisition (Tree-sitter AST)

* [ ] ออกแบบ fixtures และ test assertions ทดสอบความแม่นยำของ symbol parsing ใน `source_graph.rs`: multi-line function signatures, string literals/comments ที่มี `"fn "` หรือ `"struct "`, generic types, traits, enums และ macros (Red)
* [ ] นำ `tree-sitter` และ `tree-sitter-rust` เข้ามาใน `Cargo.toml` เพื่อ parse AST สำหรับภาษา Rust แทน string matching (Green)
* [ ] ออกแบบ fallback หรือ multi-language AST parser สำหรับภาษาสำคัญอื่นๆ (Python, JS/TS, Go) (Green)
* [ ] ผูกผลลัพธ์ AST เข้ากับ `StructurePayload` และ `StructureOutlineItem` ของ `Observation` (Green)
* [ ] Refactor ปรับปรุง performance ในการ parse AST และลดการ allocate memory (Refactor)
* [ ] Dogfooding ทดสอบแยก symbols จากโค้ดจริงใน workspace bl1nk-kept และยืนยันความถูกต้องผ่าน `just check` (Verified)

#### 2.3 Context Registry & Judge Engine Baseline (Context Admission Pipeline)

* [x] ออกแบบการทดสอบ `ContextRegistry` เก็บ snapshot revision ที่ agent เคยอ่านแล้ว เพื่อป้องกัน duplicate context reads (Red)
* [x] สร้าง `ContextRegistry` (`crate/kept-core/src/context/registry.rs`) รองรับ `context://<target>@<rev>` tracking (Green)
* [x] ออกแบบการทดสอบ `Judge` admission decisions: `Pass`, `Reference`, `Delta`, `Drop`, `Warn`, `Block` (Red)
* [x] สร้าง `Judge` engine v1 (`crate/kept-core/src/context/judge.rs`) สำหรับตัดสิน treat observations ก่อนส่งให้ agent / context stream (Green)
* [x] Refactor จัดระเบียบ admission evaluation ให้รองรับ score-based decisions และ error handling ที่ปลอดภัย (Refactor)
* [x] รัน context contract tests และ integration suite ยืนยันการทำงานของ Judge ผ่าน 100% (Verified)

#### 2.3.1 P0.1 Behavioral Waste Gate (Memoization & Outcome Accounting)

* [x] ออกแบบการทดสอบเพิ่ม `confidence: f32` ให้กับ Admission Decision เพื่อรองรับ borderline evaluation (Red)
* [x] พัฒนาระบบ Memoization Layer ยืนยัน unchanged target ใน active context คืน `Reference` ก่อน acquisition ทำงานจริง (Green)
* [x] พัฒนาระบบ Behavioral Waste Gate ยืนยัน acquisition ที่ไม่มี durable outcome คืน `RequireOutcome` และสกัดกั้น tool dispatch (Green)
* [x] เพิ่ม outcome linkage สำหรับ `observation`, `decision`, `evidence`, `correction`, `report`, `plan`, และ `discard` พร้อม discard reason (Green)
* [x] ติดตั้ง `wasted_call_rate` metric (ติดตามสถิติการร้องขอข้อมูลซ้ำโดยไม่มี delta) (Refactor)
* [x] เชื่อม gate เข้ากับ `ContextRegistry` และ `Judge` พร้อมรัน integration test ยืนยันการตัดทอน context ซ้ำซ้อน (Verified)

#### 2.3.2 P0.2 Zero-Regression Memory Store

* [x] ออกแบบ schema และ test cases สำหรับ SQLite Correction Ledger แบบ immutable (correction ID, subject, assertions, evidence URI, supersession) (Red)
* [x] พัฒนา SQLite Correction Ledger model แบบ immutable พร้อม migration (Green)
* [x] พัฒนาตัวตรวจจับความขัดแย้ง: เมื่อมี active correction `A ≠ a` ต้องบล็อกคำสั่งที่ขัดแย้ง (`Block`) ก่อนส่งไปทำงานจริง พร้อมแนบหลักฐาน (Green)
* [x] พัฒนากลไก Supersede: แทนที่ข้อเท็จจริงเดิมด้วยหลักฐานใหม่ โดยเก็บประวัติเดิมครบถ้วนและไม่แก้ทับ in-place (Green)
* [ ] พัฒนาระบบ Projection บันทึก Audit Log ลง Vault/Markdown โดยอ่านจาก SQLite แบบ append-only (Green)
* [ ] Refactor ปรับปรุง connection pooling และ safe query execution ใน SQLite wrapper (Refactor)
* [x] Dogfooding ทดสอบกับคำสั่งที่เคยได้รับการแก้ไขจริงในเซสชัน ยืนยันการสกัดกั้นไม่ให้ agent ทำผิดพลาดซ้ำ (Verified)
* [ ] อัปเดต `docs/specs/cognitive_guardrail_architecture.md` และ `.agents/MEMORY.md` บันทึกสถานะการทำงาน (Reconcile)

#### 2.3.3 P0.3 Semantic & Scope Disambiguation

* [ ] ออกแบบ test cases สำหรับคำที่มีความหมายกำกวม (เช่น "ทดสอบ", "ปัญหา", "ลบ") Assert ว่าต้องคืน `Resolve` พร้อมช้อยส์ และไม่ dispatch tool ไปเอง (Red)
* [ ] พัฒนาระบบ Intent Disambiguation: Two-tier check (Tier 1 literal rule -> Tier 2 intent congruence เทียบกับเป้าหมายเซสชัน) (Green)
* [ ] พัฒนาระบบ Scope Resolution: บังคับ canonical explicit scope, สกัดกั้น implicit/wider scope ก่อนเข้าถึง filesystem (Green)
* [ ] ติดตั้ง `override_rate` metric เพื่อวัดสถิติการ override กฎ ป้องกันการร้องเตือนพร่ำเพรื่อ (Refactor)
* [ ] Dogfooding ครอบคลุม taxonomy ทั้ง 4 หมวด และยืนยันว่า agent ไม่เดาคำกำกวมเป็น action โปรดของตัวเอง (Verified)
* [ ] Reconcile บันทึกหลักฐานใน `.agents/MEMORY.md` และรัน `just check` (Reconcile)

---

## 🟠 [P1] — Filesystem & MCP Pipeline (ระบบรับส่งข้อมูลและ Agent Tools)

### 3. ย้าย kept scan, find, review, duplicates และ incremental index ไปใช้ FFF-backed index

* [x] ออกแบบการทดสอบ `kept scan <root>` ให้สร้าง snapshot จาก FFF file inventory และรายงาน file count, total size, issues และ refresh delta แบบ deterministic (Red)
* [x] แก้ `create_or_refresh_scan` ให้รอ FFF initial scan/index readiness ก่อนเขียน snapshot (Green)
* [x] รักษา `kept scan --output`, `--json`, default user-state snapshot และ portable snapshot ให้ทำงานต่อเนื่อง (Green)
* [x] ออกแบบการทดสอบให้ `kept scan` เคารพ `.gitignore`; `--include-hidden` ต้องไม่ bypass ignore rule (Red)
* [x] ออกแบบการทดสอบสำหรับ `kept find --query` ด้วย typo path query แล้ว assert ว่าคืน fuzzy match, score และ Git status (Red)
* [x] รักษา `kept find --type`, `--name`, `--path`, `--min-size`, `--max-size`, `--after`, `--before` และ FQL facts เดิม (Green)
* [x] ให้ FFF คัด candidate paths ก่อน แล้วให้ `FilterSet` ตัดสิน extension, size, modified time, duplicate facts และ custom filters ต่อ (Green)
* [x] เพิ่ม FFF score และ Git status ใน JSON output แบบ additive โดยไม่เปลี่ยน field เดิม (Green)
* [x] ออกแบบการทดสอบ `kept review` ให้ใช้ FFF-backed snapshot เดียวกับ scan และยังแสดง space summary, scan issue, integrity finding และ naming finding ได้ (Red)
* [x] คง duplicate verification `size bucket → partial SHA-256 → full SHA-256` ไว้ โดยใช้ candidate records จาก FFF-backed snapshot (Green)
* [x] ออกแบบการทดสอบยืนยัน same-name, near-name, same-content และ hard-link reports ไม่ regress หลังเปลี่ยน engine (Red)
* [x] ทำ persistent incremental index optimization โดยใช้ FFF watcher events และ refresh plan เพื่อลด traversal/hash งานซ้ำ แต่ยังมี full rescan fallback เมื่อ watcher แจ้ง event loss หรือ ignore rules เปลี่ยน (Green)
* [x] Refactor ปรับปรุง memory footprint ในการ serialize/deserialize snapshots ขนาดใหญ่ (Refactor)
* [x] Dogfooding รันคำสั่ง `scan`, `find`, `review` บน workspace จริง และอัปเดต CLI smoke ครอบคลุมการใช้งานทั้งหมด (Verified)
* [x] อัปเดต `SPEC.md` และ `CHANGELOG.md` สรุปการย้าย index สู่ FFF-backed engine (Reconcile)

### 4. รวม FFF, filesystem tools และ MCP contract เข้า bl1nk-kept-mcp

* [x] ออกแบบการทดสอบ unified MCP tool registration: `filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, `filesystem_status` (Red)
* [x] สร้าง `FffManager` ใน `crate/kept-mcp/src/mcp/` เก็บ FFF instance ต่อ canonical root และ reuse index/watcher ตลอดอายุ MCP server (Green)
* [x] ให้ manager สร้าง instance เมื่อเรียก root ครั้งแรก, รอ readiness ด้วย timeout, reuse instance เดิม และ cleanup worker/watcher/cache เมื่อ server ปิด (Green)
* [x] ทำ `filesystem_find` คืน fuzzy path, size, modified time, binary state, Git status, score, pagination cursor และ total matches (Green)
* [x] ทำ `filesystem_grep` คืน path, line number, column, line content, context, match range และ cursor (Green)
* [x] ทำ `filesystem_multi_grep` ใช้ OR semantics ของ FFF พร้อม separate constraints (Green)
* [x] ทำ `filesystem_rescan` trigger rescan เฉพาะ root ที่ระบุ และคืน scan state ก่อน/หลังทำงาน (Green)
* [x] ทำ `filesystem_status` คืน active root, indexed file count, scanning state, watcher readiness, warmup state และ error ล่าสุด (Green)
* [x] validate canonical root ทุก tool และใช้ root guard ของ FFF สำหรับ filesystem root/home root (Green)
* [x] ออกแบบการทดสอบ root isolation, binary file, unreadable metadata, query ไม่พบ, invalid constraint, stale cursor, initial scan timeout และ watcher overflow (Red)
* [x] ทำ MCP schema/versioning สำหรับ filesystem และ vault tools; ทุก tool มี input/output schema version ที่ตรวจได้ (Green)
* [x] เพิ่ม dry-run และ permission boundary ให้ tools ที่จะเขียน vault/package state; filesystem search tools ยังคง read-only (Green)
* [x] Refactor แยก Crate `crate/kept-mcp` ออกมาต่างหากตามคำสั่ง Director ARCH-001 และบังคับ stdio logging ไปยัง stderr เท่านั้น (Refactor)
* [x] รัน MCP stdio smoke ที่เรียก filesystem tools พร้อม document tools เดิมใน process `bl1nk-kept-mcp` เดียว (Verified)
* [x] Reconcile บันทึกสถานะ ARCH-001 ใน `.agents/MEMORY.md` และอัปเดต docs (Reconcile)

---

## 🟡 [P2] — Retrieval, Evidence & Search Engine (พิสูจน์กับคลังข้อมูลจริง)

### 5. ตรวจ lifecycle จริง, dogfooding และ Thai Bigram correctness

* [ ] ออกแบบการทดสอบ watcher events `created`, `modified`, `removed` แล้ว assert ว่า filesystem query เห็น index ล่าสุดโดยไม่สร้าง instance ใหม่ (Red)
* [ ] เชื่อมต่อ watcher events เข้ากับ index query pipeline ให้สะท้อนการเปลี่ยนแปลงแบบ live (Green)
* [ ] ออกแบบการทดสอบ `.gitignore` เปลี่ยนแล้ว FFF status/rescan state ถูกส่งต่อถึง kept (Red)
* [ ] จัดการ propagation ของ ignore changes สู่ FFF rescan state (Green)
* [ ] ออกแบบการทดสอบ snapshot migration และ regression ที่ `review`, `find`, `duplicates`, `--index` ใช้ snapshot เดียวกันหลัง migration (Red)
* [ ] รักษา snapshot migration logic ให้รองรับ schema ข้ามเวอร์ชัน (Green)
* [ ] ออกแบบ Thai Bigram tokenizer corpus tests กับคำศัพท์ UI สมัยใหม่, คำติดกัน, อังกฤษปนไทย, acronym, numeral, path, URL, emoji และ punctuation (Red)
* [ ] พัฒนาและปรับปรุง tokenizer ให้ตัดคำครอบคลุม corpus cases ทั้งหมด (Green)
* [ ] เปรียบเทียบผล BM25/Thai Bigram ก่อนและหลัง FFF filesystem migration โดยใช้ fixture และ query ชุดเดียวกัน (Refactor)
* [ ] ทำ dogfooding run บน workspace จริงสำหรับ `scan`, `find`, filesystem MCP query, `convert` และ semantic search เมื่อพร้อม (Verified)
* [ ] เก็บ run manifest ของ dogfooding: root type, file count, options, machine/environment, duration, errors, result summary และข้อจำกัด (Verified)
* [ ] รัน `cargo test --workspace`, `just check`, `just cli-smoke` และ source archive smoke หลัง public CLI เปลี่ยน (Verified)
* [ ] อัปเดต `CHANGELOG.md` และ reconcile `.agents/MEMORY.md` (Reconcile)

### 6. ทำ Hybrid Semantic Search และ Reranking Engine

* [ ] ออกแบบ integration tests สำหรับ dense embedding retrieval ด้วย Ollama `bge-m3` และ Jina provider โดยคง provider/model/endpoint/env override contract ที่มีอยู่ (Red)
* [ ] เชื่อม dense vector retrieval กับ BM25 lexical search และ Thai Bigram retrieval เป็น hybrid candidate set เดียว (Green)
* [ ] ออกแบบการทดสอบ disk-backed vector similarity cache พร้อม key ที่ผูกกับ document/content hash, embedding model id, provider และ schema version (Red)
* [ ] ทำ disk-backed vector similarity cache พร้อม key ที่ผูกกับ document/content hash, embedding model id, provider และ schema version (Green)
* [ ] ทำ cache invalidation เมื่อ source hash, package commit, model id หรือ embedding settings เปลี่ยน (Green)
* [ ] ทำ cross-encoder reranker pipeline รองรับ `bge-reranker-v2-m3` และ Jina Rerank (Green)
* [ ] ออกแบบการทดสอบ reranker unavailable, provider timeout, invalid API key, empty candidate set, duplicate candidate และ fallback ที่ยังคืน lexical result ได้ (Red)
* [ ] เพิ่ม `kept search --semantic` และ `kept search --hybrid` โดยคืน source, lexical score, vector score, rerank score และ explanation ที่แยกกัน (Green)
* [ ] เพิ่ม MCP semantic search tools สำหรับ registry และ vault query โดยคืน citations ไม่ใช่เพียง answer text (Green)
* [ ] ออกแบบ regression corpus สำหรับภาษาไทย, ไทยปนอังกฤษ, synonym, typo, named entity, numeral, path และ URL (Red)
* [ ] Refactor ปรับปรุง latency ในการเรียก embedding/rerank service และจัดระบบ connection pooling (Refactor)
* [ ] Dogfooding ทดสอบ live probe เทียบกับ Ollama local / Jina endpoint บนข้อมูลคลังเอกสารจริง (Verified)
* [ ] อัปเดต `get-start.md`, `SPEC.md`, และปิดสถานะ SEM-001 ใน `.agents/MEMORY.md` (Reconcile)

### 7. ทำ Evidence run, correction loop, gold corpus และ benchmark จริง

* [ ] ออกแบบ schema และ validation tests สำหรับ immutable evidence run manifest (Red)
* [ ] ทำ immutable evidence run manifest ที่เก็บ input snapshot, corpus revision, model/config, command/options, environment, timestamp และ result hashes (Green)
* [ ] เก็บ raw JSONL ของ every evidence run และทำ offline rescore โดยไม่แก้ raw history (Green)
* [ ] ทำ append-only correction history ที่ supersede record เดิมได้โดยยัง trace กลับได้ (Green)
* [ ] ทำ `kept:debt` marker parser และ debt ledger สำหรับ source marker ที่ต้องวัด/ยกระดับในอนาคต (Green)
* [ ] ออกแบบ good/bad fixtures และ self-test correctness/safety gates ก่อนยอมรับ benchmark gain (Red)
* [ ] ทำ gain scoreboard ที่เปรียบเทียบเฉพาะ runs ที่มี comparable contract; ต่าง corpus/model/config ต้องคืน `incomparable` (Refactor)
* [ ] ทำ workflow เก็บ raw scan, Markdown, HTML, PDF และ sidecar annotation เมื่อแก้ source ไม่ได้ (Green)
* [ ] ทำ review workflow สำหรับ gold corpus 10,000 assertions พร้อม provenance, stable locator, fragment/context hash, normalized candidate, assertion kind, reviewer decision/reason และ split (Green)
* [ ] ครอบคลุม canonical, variant, synonym, homograph, homophone, semantic relation, transliteration, command/prohibition, named entity, numeral, ambiguity, negative และ boundary cases (Green)
* [ ] materialize เฉพาะ accepted reviewed assertions เป็น dictionary; hypothesis/fuzzy candidates ห้าม promote เอง (Green)
* [ ] เพิ่ม anonymized real-world corpus เมื่อมี export ที่ตัดข้อมูลอ่อนไหวแล้ว (Green)
* [ ] สร้าง real-world benchmark corpus สำหรับ duplicate/search recall/latency จาก distribution จริง (Verified)
* [ ] ทำ Thai synonym governance และ `explain-search` ที่แสดง source relation, rule, score contribution, confidence และเหตุผลที่ไม่ promote candidate (Green)
* [ ] Reconcile บันทึกหลักฐาน benchmark ใน `.agents/MEMORY.md` (DATA-001, DATA-002) (Reconcile)

---

## 🟢 [P3] — Document Fidelity & Universal IR (ประมวลผลเอกสารความรู้)

### 8. เพิ่ม kept-doc fidelity, PDF inspection และ review queue

* [ ] ออกแบบ golden round-trip tests สำหรับ Markdown → Universal IR → Markdown และ explicit unsupported diagnostics (Red)
* [ ] ทำ GitHub Flavored Markdown writer ครอบคลุม quote, list, table, code, divider, image, link และ nesting (Green)
* [ ] ออกแบบการทดสอบ Notion converter สำหรับ property type ที่ไม่รองรับ, mention หาไม่พบ, nested blocks และ conversion ที่ต้องคืน diagnostic แทนข้อมูลเงียบ ๆ (Red)
* [ ] ทำ Notion converter fidelity สำหรับ typed mapping, diagnostics, property mapping และ page/database mention resolution โดยไม่บังคับ network (Green)
* [ ] ทำ `kept doc inspect-pdf <input>` บน `kept-doc::PdfAdapter` ที่มีอยู่แล้ว (Green)
* [ ] คืน native-text Markdown, per-page provenance, page diagnostics และ OCR opt-in ที่ไม่ทำงานเองโดยอัตโนมัติ (Green)
* [ ] ออกแบบ PDF fixture สำหรับ text PDF, scanned PDF, malformed PDF, password/permission error และ per-page extraction failure (Red)
* [ ] สร้าง review queue model สำหรับ duplicate groups, large files, scanned PDF pages, validation errors และ unsupported conversion (Green)
* [ ] เพิ่ม CLI/MCP surfaces สำหรับ list/show/filter review queue โดยใช้ model เดียวกับ scan/vault/document diagnostics (Green)
* [ ] Refactor รักษา `kept-doc` ให้เป็น Pure Library ปราศจาก binaries ตาม ARCH-001 (Refactor)
* [ ] Dogfooding รัน inspect-pdf และ convert กับชุดเอกสารจริงใน repository (Verified)
* [ ] อัปเดต `SPEC.md`, `CHANGELOG.md` และ `get-start.md` พร้อมรัน `just check` (Reconcile)

---

## 🔵 [P4] — Ecosystem Expansion & Automation (ส่วนต่อขยายระบบ)

### 9. สร้าง crate kept-vault สำหรับ Git-first vault package manager

* [ ] เพิ่ม `crate/kept-vault` เป็น workspace member โดยไม่สร้าง binary ใหม่ (Green)
* [ ] ออกแบบการทดสอบ `kept-vault.toml` ที่ประกอบด้วย package id, Git URL, requested ref, resolved commit, optional subdirectory, content roots และ script declarations (Red)
* [ ] สร้าง manifest model, validation errors, parser และ deterministic serializer (Green)
* [ ] ออกแบบการทดสอบ `kept-vault.lock` ให้ทุก requested ref resolve เป็น immutable commit SHA และ lockfile เรียง deterministic (Red)
* [ ] สร้าง Git resolver สำหรับ clone, fetch, resolve branch/tag/ref เป็น commit SHA และ checkout package ตาม SHA ใน lockfile (Green)
* [ ] สร้าง package store ใน OS user-data directory แยกจาก vault root, source checkout และ temporary build output (Green)
* [ ] ทำ `kept vault init <path>` ให้สร้าง `raw/`, `wiki/`, `index.md`, `log.md`, `.kept/`, `kept-vault.toml` และ schema/template instruction files (Green)
* [ ] ทำ `kept vault package add <git-url> [--ref <ref>] [--subdir <path>]` ให้เพิ่ม package declaration (Green)
* [ ] ทำ `kept vault package install` ให้ resolve source refs, เขียน lockfile และ materialize checkout ใน package store (Green)
* [ ] ทำ `kept vault package list` แสดง package id, source URL, requested ref, resolved commit, subdirectory และ content roots (Green)
* [ ] ทำ `kept vault package status` แสดง checkout state, Git dirty state, lock drift, available scripts และ content root state (Green)
* [ ] ทำ `kept vault package update [package]` ให้ resolve ref ใหม่และ update lockfile โดยไม่รัน scripts เอง (Green)
* [ ] ทำ `kept vault package remove <package>` ให้ลบ declaration/lock entry; แยก package cache garbage collection เป็น command เฉพาะ (Green)
* [ ] Refactor ป้องกัน path traversal ในการแตก package และจัดการ lock concurrency (Refactor)
* [ ] Dogfooding ทดสอบ package lifecycle บน Git repository ตัวอย่าง (Verified)
* [ ] อัปเดตเอกสารคำสั่ง vault ใน `SPEC.md` และ `get-start.md` (Reconcile)

### 10. เพิ่ม package script runtime และ Vault lifecycle commands

* [ ] ออกแบบการทดสอบ script declaration ที่ใช้ structured command/args array และ reject shell string ที่ต้องพึ่ง implicit shell parsing (Red)
* [ ] รองรับ scripts เช่น `["cargo", "test"]`, `["just", "check"]`, `["make", "build"]`, `["bun", "run", "build"]` และ runtime commands ที่ package ประกาศ (Green)
* [ ] ทำ `kept vault run <package>:<script> [-- <args...>]` ให้รันจาก checkout ที่ตรงกับ locked commit SHA (Green)
* [ ] แสดง package id, locked commit, working directory และ command ก่อนเริ่ม child process (Green)
* [ ] stream stdout/stderr แบบ live และคืน exit code เดิมให้ caller (Green)
* [ ] ออกแบบการทดสอบ package ไม่มีใน manifest, package ไม่มีใน lockfile, checkout หาย, commit ไม่ตรง, script ไม่มี, executable ไม่พบ, process failed และ argument forwarding (Red)
* [ ] ทำ `kept vault run --list <package>` เพื่อแสดง scripts พร้อม command และ working directory (Green)
* [ ] บันทึก runtime execution แบบ append-only ใน `log.md`: package id, commit, script, timestamp, exit code และ artifact path ที่ runtime รายงาน (Green)
* [ ] ทำ `kept vault ingest <source-or-package>` อ่าน source/content roots, เก็บ raw source, สกัดสาระ, สร้างหรืออัปเดต entity/concept pages, อัปเดต `index.md` และเพิ่ม log entry (Green)
* [ ] ทำ `kept vault lint` ตรวจ orphan notes, broken wikilinks, dead links, contradiction markers, manifest/lock drift, missing checkout และ missing content roots (Green)
* [ ] Refactor ปรับปรุง child process handling และ exit code propagation (Refactor)
* [ ] ทำ CLI smoke fixture ที่ add Git package → install → run cargo/just/make script → ingest → lint (Verified)
* [ ] Reconcile docs และ changelog (Reconcile)

### 11. ทำ vault knowledge package, query และ MCP agent tools

* [ ] ออกแบบการทดสอบ `contentRoots` ให้รับเฉพาะ paths ภายใต้ package checkout/subdirectory ที่ package declaration ระบุ (Red)
* [ ] ใช้ FFF index กับ package checkout, raw source และ wiki pages เพื่อค้นหา source ของ vault แบบ live (Green)
* [ ] ทำ `kept vault query <query>` ให้ใช้ FFF fuzzy path/content retrieval ก่อน lexical search, semantic retrieval และ reranker (Green)
* [ ] คืน citations ที่มี package id, locked commit, relative path, line range, excerpt และ retrieval score (Green)
* [ ] ทำ `kept vault query` สังเคราะห์คำตอบจาก wiki/source pages และบันทึกผลเป็น wiki note พร้อม citation/backlink/log entry (Green)
* [ ] ทำ MCP tools `vault_ingest`, `vault_query`, `vault_lint` บน `bl1nk-kept-mcp` (Green)
* [ ] ออกแบบการทดสอบ source นอก content roots, missing package checkout, citation ชี้ commit ผิด, broken link หลัง package update และ query ที่ไม่มีผลลัพธ์ (Red)
* [ ] Refactor จัดระเบียบ MCP schema และ token cost efficiency (Refactor)
* [ ] ทำ end-to-end fixture: add package → install → FFF index → query with citations → ingest → lint → update package → lint drift (Verified)
* [ ] Reconcile MCP specs และ user guides (Reconcile)

### 12. ทำ Notion Safe Sync, reconciler และ operator experience

* [ ] ทำ Notion Safe Sync model สำหรับ stable remote IDs, idempotency key, revision precondition, dry-run diff, conflict policy, audit log และ rollback design (Green)
* [ ] ออกแบบการทดสอบ retry request เดิมแล้วห้ามสร้าง duplicate remote block/page (Red)
* [ ] ออกแบบการทดสอบ remote revision เปลี่ยนระหว่าง sync แล้วต้องคืน conflict ที่ระบุ local/remote state (Red)
* [ ] เปลี่ยน reconciler จาก `block_{index}` ไปใช้ stable remote IDs (Green)
* [ ] ทำ idempotent detection ของ move, insert, delete และ content update (Green)
* [ ] ทำ dry-run output ที่แสดง action plan ก่อนทุก remote mutation (Green)
* [ ] สร้าง Interactive TUI สำหรับ scan, review queue และ action plan โดยใช้ domain model เดียวกับ CLI/MCP (Green)
* [ ] ออกแบบการทดสอบ TUI keyboard flow, non-interactive fallback และ state restoration (Red)
* [ ] Refactor แยก terminal drawing ออกจาก core synchronizer (Refactor)
* [ ] Dogfooding ทดสอบ TUI กับ action plan จริงใน terminal (Verified)
* [ ] รัน focused tests ต่อ slice, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`, `just check`, `just cli-smoke`, MCP stdio smoke และ source archive smoke ก่อนปิด public behavior (Verified)
* [ ] Reconcile สถานะลงใน `.agents/MEMORY.md` และ `CHANGELOG.md` (Reconcile)

### 13. Installer Scripts & Packaging Distribution

* [x] ออกแบบการทดสอบ verification test สำหรับ installation script assertions บน Linux/macOS/Windows (Red)
* [x] ปรับ `install-mcp.sh` ให้ติดตั้ง release asset `bl1nk-kept-mcp` จาก GitHub บน Linux/macOS/Windows shell (Green)
* [x] เพิ่ม `install-mcp.ps1` ให้ติดตั้ง release asset `bl1nk-kept-mcp.exe` บน Windows (Green)
* [x] เพิ่ม release workflow ให้ build และแนบ MCP binary asset พร้อม SHA-256 checksum (Green)
* [x] เพิ่ม release-gate hook จับ `git tag` และ tag push พร้อมหน่วงเวลา 40 วินาที (Green)
* [x] เพิ่ม `release-verifier` agent ตรวจ checklist, แก้ TODO และวนตรวจซ้ำก่อนปล่อย release (Green)
* [x] ให้ release gate ส่ง deterministic preflight และ detailed findings กลับ main agent ก่อน verifier ตัดสิน PASS/FAIL (Green)
* [x] แยก pre-commit ให้รัน `just check` ก่อน commit; release gate ไม่แทนที่ pre-commit (Refactor)
* [x] เพิ่มคำสั่ง `/commit-push-tag` ให้ main agent เก็บงาน ตรวจ แอด คอมมิต ติดแท็ก และส่งต่อ release verifier ตามลำดับ (Green)
* [x] รัน smoke test ทดสอบ installer script บนทุก OS platform (Verified)
* [x] บันทึกคำแนะนำใน `README.md` และ `get-start.md` (Reconcile)
