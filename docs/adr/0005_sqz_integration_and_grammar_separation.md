# บันทึกการตัดสินใจ 0005: SQZ Integration Strategy และ kept-grammar Crate Separation

**สถานะ:** ตัดสินใจแล้ว
**วันที่:** 2026-09-08

## บริบท

### ปัญหาที่นำมาสู่การตัดสินใจ

**คำถามที่ 1 — SQZ Integration Strategy:**
keeping-core มี comment ว่า "inspired by SQZ" ใน `judge.rs` และ `registry.rs` แต่ไม่ได้ depend จาก SQZ โดยตรง SQZ source อยู่ที่ `D:\01work\Active\references\campbellr\sqz\` มี 67 modules แต่ kept-core เขียนใหม่จากศูนย์ 14 modules — เสียเวลาและได้ code ด้อยกว่าในบางจุด (token counting, content-hash dedup, confidence routing)

**คำถามที่ 2 — Keyword Grammar Separation:**
`policy.rs` (1221 LOC) รวม types, config I/O, validation logic, naming analysis ไว้ในไฟล์เดียว ทำให้:
- kept-core มี responsibility มากเกินไป (business logic + grammar types + config resolution)
- ไม่สามารถ reuse grammar types โดยไม่ depend kept-core ทั้งหมด
- word boundary ระหว่าง "grammar rules" กับ "context admission" ไม่ชัดเจน

### ทางเลือกที่พิจารณา

**SQZ Integration:**
- ก) Vendor ทั้งหมด (`vendor/sqz/`) — เอามาทั้งตัว
- ข) Port เฉพาะ functions ที่จำเป็นเข้า kept-core โดยตรง
- ค) 保持 "inspired by" ไม่ depend จริง (สถานะเดิม)
- ง) Git dependency (`sqz = { git = "..." }`)

**Keyword Grammar:**
- ก) แยก `kept-grammar` crate ใหม่ — types + config I/O + validation
- ข) แยก module `grammar` ภายใน kept-core — ไม่ต้อง workspace ใหม่
- ค) คงเดิม — policy.rs รวมทุกอย่าง

## การตัดสินใจ

### A. SQZ: Port เฉพาะ functions เข้า kept-core โดยตรง

| ทางเลือก | Rationale |
|---|---|
| ~~Vendor ทั้งหมด~~ | **ตัดออก** — dependency conflicts (rusqlite 0.31 vs 0.39, tree-sitter cc pin), 67 modules มากเกินไป, kept-core จะ depend สิ่งที่ไม่ใช้ |
| **Port functions** ✓ | **เลือก** — เอาเฉพาะส่วนที่ kept-core ขาด (token counting, confidence routing, content-hash dedup, regret tracking) แล้ว integrate เข้า module ที่มีอยู่ |
| ~~保持 "inspired by"~~ | **ตัดออก** — ไม่มี backup, ไม่มี git history, code ด้อยกว่า SQZ ในบางจุด |
| ~~Git dependency~~ | **ตัดออก** — SQZ เป็น exe project ไม่ใช่ library, workspace conflicts, kept-core ไม่ควร depend external binary |

**Functions ที่ port:**

| Function | Source SQZ | Target kept-core | Rationale |
|---|---|---|---|
| BPE token counting | `token_counter.rs` | `token_counter.rs` (ใหม่) | kept-core ไม่มี token counting จริง — ใช้ chars/4 estimate |
| Content routing | `confidence_router.rs` + `entropy_analyzer.rs` | `content_router.rs` (ใหม่) | Judge ต้องการ admission mode (Safe/Default/Aggressive) |
| SHA-256 dedup | `cache_manager.rs` | `context/registry.rs` (enhanced) | Registry ขาด content-hash dedup และ LRU eviction |
| Regret tracking | `regret_tracker.rs` | `regret_tracker.rs` (ใหม่) | ไม่มีใน kept-core — เรียนรู้จาก re-reads ทำให้ระบบ self-improving |

**_functions ที่ไม่ port:**

| Function | Rationale |
|---|---|
| `session_store.rs` | Complex SQLite schema + FTS5, deferred ไป P1 |
| `pipeline/stages.rs` | Compression pipeline — kept-core ไม่ compress, ใช้ judgment |
| `tree-sitter modules` | kept-core ไม่ใช้ AST parsing ตอนนี้ (TODO 2.2.1) |
| `toon encoder` | JSON encoding — ไม่ใช่ core responsibility |

### B. Keyword Grammar: แยก `kept-grammar` crate

| ทางเลือก | Rationale |
|---|---|
| **kept-grammar crate** ✓ | **เลือก** — types, config I/O, validation rules เป็น standalone ที่ reuse ได้โดยไม่ depend kept-core |
| ~~Module ภายใน kept-core~~ | **ตัดออก** — ยังคง tight coupling, ไม่ able to reuse โดยไม่ import ทั้ง kept-core |
| ~~คงเดิม~~ | **ตัดออก** — policy.rs 1221 LOC รวมทุกอย่าง ไม่ maintainable |

**kept-grammar มี:**
- Types: `UserConfig`, `NamingProfile`, `NamingSettings`, `PolicyError`, `TokenReplacement`, etc.
- Config I/O: `load_user_config`, `save_user_config`, `default_user_config_path`, `starter_config_yaml`
- Validation: `validate_naming_settings` (grammar rules validation)

**kept-core ยังมี (ไม่ย้าย):**
- `UserConfig::validate()` — depend `crate::semantic::validate_semantic_settings`
- `resolve_naming_rule()`, `analyze_naming()`, `analyze_index_naming()` — depend `crate::scanner::ScanIndex`
- All merge/apply functions — depend types จาก kept-grammar แต่ logic อยู่ kept-core

### C. kept-core re-export strategy

kept-core policy.rs ยังคง types เดิม (ไม่ใช่ re-export จาก kept-grammar) เนื่องจาก:
1. `impl UserConfig` ไม่สามารถ define ใน kept-core ถ้า types มาจาก kept-grammar ( orphan rule)
2. Migration ทำ incrementally — ย้าย types ทีละตัวเมื่อมี breakage
3. ตอนนี้ kept-grammar เป็น standalone crate ที่ tool อื่นใช้ได้โดยไม่ต้อง depend kept-core

## ผลกระทบ

- **kept-core** เพิ่ม modules 4 ตัว: `token_counter.rs`, `content_router.rs`, `regret_tracker.rs`, `context/registry.rs` (enhanced)
- **kept-grammar** crate ใหม่: types + config I/O + validation rules
- **SQZ source** เก็บเฉพาะ reference ที่ `D:\01work\Active\references\campbellr\sqz\` — ไม่ vendor
- **Tests** เพิ่มขึ้น 20+ test cases (token counting, content routing, SHA-256, LRU eviction, regret tracking)
- **Cargo.toml** เพิ่ม `tiktoken-rs = "0.6"` ใน kept-core, `kept-grammar` ใน workspace
- **TODO.md** เพิ่ม sections 2.4 (SQZ Integration), 2.5 (kept-grammar), 2.6 (CLI/MCP Separation)

## หลักฐาน

- `crate/kept-core/src/token_counter.rs` — BPE counting ผ่าน tiktoken-rs
- `crate/kept-core/src/content_router.rs` — Safe/Default/Aggressive routing
- `crate/kept-core/src/regret_tracker.rs` — re-read learning
- `crate/kept-core/src/context/registry.rs` — SHA-256 + LRU eviction
- `crate/kept-grammar/src/types.rs` — grammar types
- `crate/kept-grammar/src/config.rs` — config I/O + validation
- `Cargo.lock` — tiktoken-rs dependency
- `CHANGELOG.md` — entry สำหรับทุกการเปลี่ยนแปลง

## เอกสารอ้างอิง

- [TODO.md — section 2.4-2.6](../../TODO.md)
- [SQZ source](../../../references/campbellr/sqz/) (reference only)
- [ADR 0004 — SQZ Reimplement](./0004_semantic_disambiguation_two_tier_architecture.md)
- [CHANGELOG.md](../../CHANGELOG.md)
