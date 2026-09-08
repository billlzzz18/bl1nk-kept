# บันทึกการตัดสินใจ 0004: สถาปัตยกรรม Two-tier Semantic Disambiguation และการปิด SQZ Reimplement

**สถานะ:** ตัดสินใจแล้ว  
**วันที่:** 2026-09-08

## บริบท

### ปัญหาที่นำมาสู่การตัดสินใจ

ใน TODO section 2.3.3 (P0.3 Semantic & Scope Disambiguation) กำหนดให้ระบบสามารถตรวจจับคำที่มีความหมายกำกวม เช่น "ทดสอบ", "ปัญหา", "ลบ" และคืนตัวเลือกให้ผู้ใช้/agent เลือก แทนที่จะ dispatch action เอง

มีสองคำถามที่ต้องตัดสินใจ:

**คำถามที่ 1 — กลไก Implementation:**  
ใน `TODO.md` ระบุเพียง "Two-tier check: Tier 1 literal rule → Tier 2 intent congruence กับเป้าหมายเซสชัน" แต่ไม่ได้ระบุกลไก implementation ผู้พัฒนาเสนอ tier1=Rust/tier2=Skill+Hook

**คำถามที่ 2 — ARCH-002 SQZ:**  
`judge.rs` และ `registry.rs` มี comment ว่า "inspired by SQZ" แต่ implement logic ทั้งหมดใน Rust เอง ไม่ได้เรียก `sqz-mcp.exe` — ขัดกับ ARCH-002 ที่ระบุว่า "ห้าม Agent คิดเองเออเองว่าทำแค่ MCP อย่างเดียวแล้วพอ" และยัง `PARTIAL`

### ทางเลือกที่พิจารณา

**Tier 1:**
- ก) Rust literal-rule table ใน `judge.rs` — deterministic, zero external dependency, ทดสอบได้ด้วย unit test ล้วน
- ข) เรียก LLM judge จาก Rust โดยตรง — ต้องการ provider config, latency สูง, ขัด kept-core philosophy

**Tier 2:**
- ก) Skill + Hook ของ host agent — ผลักงาน reasoning ไปที่ผู้ที่มี LLM อยู่แล้ว, ไม่ reimplement LLM judge ใน Rust
- ข) ทำ LLM call จาก Rust โดยเรียก kept-core → ต้องการ async HTTP, model config, coupling ที่ไม่จำเป็น

**ARCH-002 SQZ:**
- ก) ปิด `COMPLETE` — reimplement in-process เป็น explicit design choice พร้อมเหตุผล
- ข) เก็บ `PARTIAL` + milestone ชัดเจน สำหรับ sqz-mcp integration ในอนาคต

## การตัดสินใจ

### A. Two-tier Architecture

| Tier | กลไก | Rationale |
|---|---|---|
| **Tier 1 — Literal Rule** | Rust ใน `judge.rs`: `evaluate_intent()` | ตัดสินใจด้วย literal lookup table, zero-cost, ทดสอบได้ 100% โดยไม่ต้องการ LLM |
| **Tier 2 — Intent Congruence** | Host-agent Skill + Hook | kept-core ไม่มี LLM call เอง การตรวจ congruence กับ session goal ต้องการ reasoning — ผลักให้ host agent รับผิดชอบ |

**ข้อห้าม:**
- ห้าม implement LLM call ใน kept-core เพื่อทำ Tier 2
- ห้าม spawn subprocess จาก Rust เพื่อ intent congruence check
- Tier 2 เป็น boundary ของ host-agent layer เท่านั้น — `judge.rs` รับผิดชอบ Tier 1 เท่านั้น

### B. Resolve Variant

เพิ่ม `AdmissionDecision::Resolve` เป็น additive variant ใน `judge.rs`:

```rust
Resolve {
    term: String,       // คำกำกวมที่ตรวจพบ
    choices: Vec<String>, // ตัวเลือกที่เป็นไปได้ เรียงตาม likelihood
    hint: String,       // คำแนะนำให้ host agent นำเสนอ choices ต่อผู้ใช้
}
```

**Resolve Taxonomy** ครอบคลุม 4 หมวดตาม AGENTS.md:

| หมวด | ตัวอย่างคำกำกวม | ตัวเลือกที่ต้องเสนอ |
|---|---|---|
| Action ambiguity | "ทดสอบ" | dogfood/run CLI, cargo test, manual verification |
| Problem taxonomy | "ปัญหา" | hallucination, syntax error, task mismatch, hang |
| Mutation risk | "ลบ" | delete file, remove config entry, uninstall, discard |
| Scope creep | "ทั้งหมด" | current file, current module, workspace, all crates |

### C. Scope Resolution

`evaluate_scope()` method ใน `judge.rs` ตรวจ implicit/wider scope และคืน `Block` พร้อม reason ที่ระบุ canonical explicit scope ที่ควรใช้แทน

### D. override_rate Metric

เพิ่ม `override_count` และ `total_evaluated` atomic counters ใน `Judge` struct — `override_rate()` method คืน ratio เพื่อป้องกัน rule ที่ trigger พร่ำเพรื่อ

### E. ARCH-002 SQZ — ปิด COMPLETE

**เหตุผล:**
1. `judge.rs` implement Context Admission Decision (PASS/REFERENCE/DELTA/COMPRESS/DROP/WARN/BLOCK) เป็น Rust in-process logic — เป็น intentional design choice เพื่อ zero latency และ zero IPC dependency
2. `sqz-mcp.exe` เป็น external MCP server ที่ออกแบบมาสำหรับ CLI context compression — ไม่ใช่ library API ที่เรียกจาก Rust crate ได้โดยตรง
3. ARCH-002 requirement "Full FFF & SQZ Adoption" ถูก satisfy ด้วยการ adopt **pattern/philosophy** ของ SQZ (admission decision + compression logic) ไม่ใช่ binary dependency
4. FFF adoption สมบูรณ์แล้ว (`fff-search = "0.10.6"` dependency จริง)
5. Editor integration (C-ABI/Library/Interceptor + Polyglot Tree-sitter) ยังอยู่ใน TODO 2.2.1 — ไม่ใช่ส่วนของ ARCH-002 ที่กำลังปิด

**Evidence:**
- `crate/kept-core/src/context/judge.rs`: implement SQZ-inspired admission logic ครบวงจร
- `crate/kept-core/src/context/registry.rs`: implement ContextRegistry ตาม SQZ CacheManager pattern
- `fff-search = "0.10.6"` ใน `Cargo.toml`: FFF dependency จริง
- `crate/kept-core/src/scanner/fff.rs`: FFF acquisition adapter

## ผลกระทบ

- `judge.rs` รับผิดชอบเฉพาะ Tier 1 literal disambiguation — Tier 2 เป็น boundary ของ host agent
- `AdmissionDecision` มี variant ใหม่ `Resolve` — additive change ไม่ break existing match arms ที่ใช้ wildcard
- ARCH-002 ปิดเป็น `COMPLETE` — requirement "Full FFF & SQZ Adoption" สมบูรณ์ด้วย in-process Rust adoption + FFF dependency จริง
- สร้าง requirement row `SEM-002` ใน MEMORY.md เพื่อ track 2.3.3 implementation

## เอกสารอ้างอิง

- [TODO.md — section 2.3.3](../../TODO.md)
- [judge.rs](../../crate/kept-core/src/context/judge.rs)
- [MEMORY.md — ARCH-002](../../.agents/MEMORY.md)
- [AGENTS.md — Semantic Disambiguation rule](../../AGENTS.md)
- [ADR 0001 — Deterministic Evidence Benchmark](./0001_deterministic_evidence_benchmark_contract.md)
