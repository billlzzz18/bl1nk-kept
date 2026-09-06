# แนวทางการร่วมพัฒนา bl1nk-kept (Contributing Guide)

เริ่มจาก `AGENTS.md` เสมอ เอกสารนั้นระบุลำดับการอ่าน, แหล่งข้อมูลความจริง (Source-of-truth), และวงรอบการพัฒนาแบบ TDD (Test-Driven Development)

---

## 1. ขอบเขตความรับผิดชอบของแต่ละ Crate (Crate Boundaries)

| Crate | ความรับผิดชอบหลัก |
|---|---|
| `kept-core` | Observation model (`Target`, `Revision`), FFF scanner adapter, Context Registry, Search & Retrieval (BM25/FTS/Vector), และ Judge Engine |
| `kept-doc` | Universal Document IR, Parsers (Markdown, NFM, DOCX, PDF, HTML) และ Outline extraction (Pure Library) |
| `kept-cli` | CLI command surface (`kept`) สำหรับจัดการ workspace, registry, filesystem และ context commands |
| `kept-mcp` | Stdio MCP server (`bl1nk-kept-mcp`) ส่งมอบ context และ tools ให้ AI agent ผ่าน Judge Admission Gate |

**กฎเหล็ก:** ห้ามคัดลอก Business Logic ข้าม crate, ทุก public capability ต้องมี focused test รองรับเสมอ

---

## 2. คำสั่งจำเป็นในการพัฒนา (Workflow Commands)

ใช้ `just` เป็น task runner หลักของโปรเจกต์:

```bash
# ตรวจสอบคุณภาพทั้งหมดก่อน commit (Format, Lint, Tests, Contracts, Schema)
just check

# รันเฉพาะชุดทดสอบทั้งหมดใน workspace
cargo test --workspace

# อัปเดตและตรวจจับ drift ของ schema
just schema
just schema-check

# จัดการเวอร์ชันของ workspace
just version-bump <semver>

# สร้างแพ็กเกจ source archive สำหรับ release
just package
```

---

## 3. กฎการเขียนโค้ดและการทดสอบ (Engineering Standards)

1. **Test-Driven Development (TDD):**
   - เขียน failing test เพื่อกำหนด contract และพฤติกรรมที่ต้องการก่อนเสมอ
   - แก้ไขโค้ดให้น้อยที่สุดเท่าที่จำเป็นเพื่อให้ test ผ่าน (Green)
2. **Context & Observation Contract:**
   - การดึงข้อมูลหรือสำรวจไฟล์ต้องสร้างผ่าน `Observation` และระบุ `Target` URI เสมอ
   - แยกแยะ `look` (metadata/outline เบาแรง) ออกจาก `view` (materialized content) อย่างชัดเจน
3. **Rust Toolchain & MSRV:**
   - โค้ดต้องคอมไพล์ผ่านบน Rust Stable (MSRV ตามที่ระบุใน `Cargo.toml`)
   - ผ่าน `cargo fmt` และ `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings)
4. **ความสะอาดของ Repository & CI Cost Guard:**
   - ละเว้นการ push artifact ชั่วคราว หรือรัน CI บนเอกสารโดยไม่จำเป็น
   - การเสนอไอเดียใหม่ให้บันทึกลง `docs/ideas/` ก่อนนำเข้า `plan.md`

---

## 4. การส่งมอบงาน (Handoff & Verification)

ก่อนเปิด PR หรือส่งต่องาน:
1. ยืนยันว่า `just check` ผ่าน 100%
2. หากมีการเปลี่ยนแปลง Public Contract ให้อัปเดต `SPEC.md` และบันทึกใน `CHANGELOG.md`
3. บันทึกบทเรียนหรือข้อจำกัดที่พบลง `.agents/MEMORY.md` เพื่อส่งต่อบริบทให้ Agent เซสชันถัดไป
