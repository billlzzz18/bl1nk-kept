# bl1nk-kept — Developer & Agent Guide

คู่มือหลักสำหรับ Claude Code, AI Agents และ Developers ในการพัฒนา, ทดสอบ และดูแลระบบ `bl1nk-kept`

---

## 1. Overview & Core Mission

`bl1nk-kept` คือ Local-first Context Engine & Cognitive Guardrail สำหรับ AI Agents และ Knowledge Management พัฒนาด้วยภาษา Rust (Workspace version: `0.4.0`)

### เสาหลักของระบบ (Core Pillars)

1. **Cognitive & Semantic Guardrail:**
   - **Intent Disambiguation:** ป้องกัน Agent ตีความคำกำกวมเป็น Action ที่ตนถนัดฝ่ายเดียว (เช่น "ทดสอบ" = Unit Test vs Dogfooding)
   - **Zero-Regression Learning:** บันทึกข้อเท็จจริงที่เคยแก้ไขลงใน Immutable SQLite Correction Ledger เพื่อป้องกันไม่ให้ Agent ทำผิดซ้ำ
   - **Behavioral Accountability & Token Hygiene:** ตรวจจับและยับยั้งการอ่านไฟล์ซ้ำซ้อน (In-Context Amnesia) และการกวาดอ่านข้อมูลสะสมโดยไม่สร้างผลลัพธ์ (Unproductive Acquisition)
2. **Intelligent Context & Knowledge Vault:**
   - ค้นหา จัดทำดัชนี แปลง ตรวจสอบ เปรียบเทียบ และคัดกรอง Context ด้วย **FFF Acquisition Core** (`fff-search`) ร่วมกับ **Judge Engine** เพื่อลดการใช้โทเค็นสูงสุด (Token Savings > 99%)

---

## 2. Workspace Architecture & Crates

| Crate | Binary / Lib | บทบาทและหน้าที่หลัก |
|---|---|---|
| `kept-core` | Library | Central Hub: Observation Model, Judge Engine, Foundation Registry, Scanner/FFF Adapter, Policy Engine, Source Graph (AST), Semantic Search, Token Counter |
| `kept-grammar` | Library | Grammar Types, Keyword Validation Rules, Naming Profiles, Config Resolution |
| `kept-doc` | Library Only | Universal IR (Intermediate Representation) และตัวแปลงเอกสาร (Notion, Markdown, PDF, DOCX) — ไม่มี Binary |
| `kept-cli` | Binary (`kept`) | User CLI Surface: `kept scan`, `kept find`, `kept review`, `kept duplicates`, `kept search`, `kept doctor`, `kept plugin` |
| `kept-mcp` | Binary (`bl1nk-kept-mcp`) | Long-running stdio MCP Server: Tools สำหรับ Filesystem, Document IR, Notion Live, Diagrams, และ Watcher |
| `kept-agent` | Library | Agent Integration Layer: Hooks, Config, Status Detection (Claude, Antigravity) |

> **Single Owner Invariant:** ไม่มี Business Logic ซ้ำซ้อนข้าม Crate — `kept-cli` และ `kept-mcp` ทำหน้าที่เป็นเพียง Presentation/Transport layer ที่ delegate ไปยัง `kept-core`

---

## 3. Core Architectural Subsystems

### 3.1 Observation Model & URI Scheme

ทุกข้อมูลที่เข้าสู่ระบบต้องถูกห่อหุ้มเป็น `Observation` พร้อม Target URI:
- `file:///path/to/file` — ไฟล์ในระบบ Local Filesystem
- `symbol://crate/module/symbol` — สัญลักษณ์โค้ด (Function, Struct, Trait)
- `document:///path/to/doc` — แหล่งข้อมูลเอกสาร (Notion, PDF, DOCX)
- `search://query` — ผลลัพธ์จากการค้นหา
- `context://target@revision` — Context ที่เคยผ่าน Admission แล้ว (สำหรับ Dedup)

### 3.2 Look vs View

- **`look` (Lightweight Scan):** ดึงเฉพาะ Metadata, Size, Revision, Hash, และ Outline โครงสร้าง (ใช้โทเค็น 0 หรือน้อยมาก)
- **`view` (Content Materialization):** ดึงเนื้อหาจริงเฉพาะ Range หรือ Symbol ที่ต้องการเมื่อจำเป็นต้องอ่าน

### 3.3 Judge Engine & Admission Gate

Context Admission Gate ทำหน้าที่ตัดสินใจการส่งมอบ Context ให้กับ Agent:
- `PASS` — ข้อมูลใหม่ ส่งมอบเนื้อหาเต็ม
- `REFERENCE` — เคยเห็นแล้วและไม่เปลี่ยน ส่ง 13-token pointer (ประหยัดโทเค็นได้ ~99.8%)
- `DELTA` — เคยเห็นแล้วแต่มีการแก้ไข ส่งเฉพาะส่วน Diff
- `COMPRESS` — ย่อโครงสร้างเอกสารตาม Structure-aware rules
- `WARN` / `BLOCK` — ตรวจจับการอ่านไฟล์ซ้ำซาก (≥3 ครั้ง) หรือข้อมูลที่ขัดแย้งกับ Correction Ledger

### 3.4 FFF Acquisition Engine

- ใช้ `fff-search` สำหรับ Traversal, Git status, Ignore filtering (`.gitignore`, `.ignore`), Fuzzy search, และ Live file watcher
- **ห้ามใช้** `std::fs::read_dir` สำหรับ Traversal หลักของระบบ
- ทุก Path ต้องอิงจาก Canonical Root และส่งคืนผลลัพธ์เป็น Deterministic Relative Path

### 3.5 Duplicate Verification Pipeline
การตรวจหาไฟล์ซ้ำต้องผ่านลำดับขั้นตอนที่แน่นอน:

```text
1. Size Bucket Match  ──>  2. Partial SHA-256  ──>  3. Full SHA-256  ──>  4. Group Evidence
```

รายงานต้องแยกชัดเจน: `same_name`, `near_name`, `same_content`, `hard_link`

### 3.6 AST & Multi-language Parsing

- ใช้ `tree-sitter` ในการสกัด Symbol Outline (Rust, Python, JavaScript, TypeScript, Go)
- Fallback ด้วย Regex เมื่อโครงสร้างไฟล์ยังไม่สมบูรณ์

---

## 4. Delivery Loop (Strict TDD)

ทุก Unit of Work ต้องปิดวงจรครบ 5 ขั้นตอน:
1. **Red:** เขียน Failing Test ก่อนเริ่มเขียน Production Code
2. **Green:** เขียน Minimal Implementation ให้ Test ผ่าน
3. **Refactor:** จัดการ Code Safety (ลบ `unwrap`, ใช้ `?`), จัด Module Path, ใส่ Comment `// NOTE-xxx:` ภาษาไทย
4. **Verify:** รัน Test และ Linter ทั้ง Workspace ให้ผ่าน 100%
5. **Reconcile:** อัปเดตสถานะใน `TODO.md` และบันทึกสิ่งที่เปลี่ยนแปลงใน `CHANGELOG.md`

### Status Legend ใน TODO.md

- `[x]` — เสร็จสมบูรณ์ (มีโค้ด + เทสต์ผ่าน)
- `[ ]` — ยังไม่ได้ทำ
- `[~]` — ทำบางส่วน (ยังไม่ครบ Acceptance criteria)

---

## 5. Build, Test & Quality Commands

```bash
# Quality Gate ทั้งหมด (รันก่อนส่งมอบงาน / PR)
just check

# Build & Test
cargo build --workspace
cargo test --workspace
cargo test -p kept-core
cargo test -p kept-cli
cargo test -p kept-mcp
cargo test -p kept-doc
cargo test -p kept-grammar

# Linter & Formatting
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# CLI Smoke & Schema Verification
just cli-smoke
just schema-check
just schema
```

---

## 6. Rust Coding & Safety Standards

- **Zero Panic / Avoid `unwrap()` & `expect()`:** ใน Production Code ให้ใช้ `?`, `match`, หรือ `if let` เท่านั้น
- **Workspace Restriction Lints:** `clippy::unwrap_used` และ `clippy::expect_used` ถูกตั้งค่าเป็น `warn`
- **Error Handling Architecture:**
  - `thiserror` สำหรับ Library Crates (`kept-core`, `kept-doc`, `kept-grammar`)
  - `anyhow` สำหรับ Application Binary Crates (`kept-cli`, `kept-mcp`)
- **Don't Swallow Errors:** ห้ามใช้ `let _ =` ปิดกั้น Error จากการทำงานที่อาจล้มเหลว ให้ propagate หรือ log เสมอ
- **Guard Index Access:** หลีกเลี่ยง `arr[i]`; ใช้ `.get(i)` เพื่อป้องกัน Index Out of Bounds
- **Full Variable Names:** ใช้ชื่อตัวแปรที่สื่อความหมายชัดเจน (เช่น `query`, `buffer`, `observation` ไม่ใช้ `q`, `buf`)
- **Modern Module Structure:** ห้ามใช้ `mod.rs` ให้ใช้รูปแบบ Modern Path (เช่น `src/scanner.rs` + `src/scanner/duplicate.rs`)
- **Variable Shadowing for Async:**

  ```rust
  let client = client.clone();
  tokio::spawn(async move {
      client.execute().await;
  });
  ```

---

## 7. Comment & Language Standards

- **Internal Rationale:** `// NOTE-001:`, `// NOTE-002:` เรียงเลขตามประเด็น เป็น **ภาษาไทย**
- **Public Rustdoc (`///`):** **ภาษาอังกฤษล้วน** สำหรับ Public APIs, Structs, CLI Help
- **Error Messages:** **ภาษาอังกฤษล้วน**
- **Project Docs & Guidelines:** **ภาษาไทย**

---

## 8. External Tools & MCP Integrations

| Tool / Server | Executable / Transport | หน้าที่และขอบเขต |
|---|---|---|
| `bl1nk-kept-mcp` | Stdio Server | MCP Server ประจำโปรเจกต์ (Logging ต้องออก `stderr` เท่านั้น ห้ามเขียนลง `stdout`) |
| FFF MCP | `fff-mcp.exe` | ค้นหาไฟล์, Fast Grep, Multi-grep, Rescan ผ่านระบบไฟล์ความเร็วสูง |
| Serena MCP | `serena.exe` | AST Symbols, LSP Diagnostics, Semantic Code Navigation |
| SQZ MCP | `sqz-mcp.exe` | Token Compression, Context Deduplication |

---

## 9. Critical Gotchas & Invariants

1. **Dependency Locking Restrictions:**
   - `schemars`: ล็อกที่ `0.8` ตาม Schema Export API ปัจจุบัน (ห้าม bump เป็น 1.x โดยยังไม่ migrate โค้ดใน `kept-core/src/schema.rs`)
   - `tree-sitter`: ล็อก runtime `0.25.10` ให้ตรงกับ `tree-sitter-rust 0.24` (ห้าม bump runtime แยกจาก grammar crates)
2. **Stale-Index Safety Gate:** `simulate_duplicate_mutation` ต้องใช้ Real `ScanIndex` จาก Snapshot เสมอ (ห้ามใช้ Empty Index)
3. **CLI Mutation Safety:** คำสั่งที่แก้ไขไฟล์จริงต้องผ่าน confirmation gate เสมอ (ใน Non-interactive mode ต้องระบุ `--action <name> --yes`)
4. **Stdio MCP Transport Cleanliness:** ห้ามมี `println!()` หรือ Log ออก `stdout` ใน `kept-mcp` เด็ดขาด เพราะจะทำให้ JSON-RPC Protocol เสียหาย

---

## Command output

Command output here is condensed to save tokens, keeping every signal and
dropping costly noise. Treat it as the complete result: run commands
normally, and batch related commands into one call to avoid extra turns.
Truncated results state their recovery path in their own output. Re-run a
command as `rtk proxy <cmd>` only when its result is unusable: empty when
output was clearly expected, contradicting its exit code, or garbled.
