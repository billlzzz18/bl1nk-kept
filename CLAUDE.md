# bl1nk-kept — คำแนะนำ Claude Code

คู่มือสำหรับ Claude Code ในการพัฒนาและทดสอบระบบ `bl1nk-kept`

---

## สถาปัตยกรรมระบบ (System Architecture)
- **Acquisition:** FFF Adapter จัดการ filesystem (`look`, `view`, `grep`, `watch`)
- **Structure:** Tree-sitter วิเคราะห์ Code AST และ `kept-doc` วิเคราะห์ Document IR
- **Model:** แปลงข้อมูลเป็น `Observation` พร้อม `Target` URI (`file://`, `symbol://`, `document://`)
- **Judge & Admission:** เปรียบเทียบกับ Context Registry แล้วส่ง treatment (`PASS`, `REFERENCE`, `DELTA`, `COMPRESS`) ผ่าน `kept-mcp`
- **Tooling:** ใช้ FFF MCP (`C:\Users\Admin\AppData\Local\fff-mcp\bin\fff-mcp.exe`) สำหรับการค้นหาไฟล์และสำรวจโครงสร้างอย่างรวดเร็ว (High-performance file finder)

---

## คำสั่งที่ใช้งานบ่อย (Essential Commands)

### Build & Test
```bash
# คอมไพล์ทั้ง workspace
cargo build --workspace

# รัน unit tests ทั้งหมด
cargo test --workspace

# รันเฉพาะ crate
cargo test -p kept-core
cargo test -p kept-doc
cargo test -p kept-cli
cargo test -p kept-mcp

# รัน static analysis
cargo clippy --workspace
```

### Full Validation Suite
```bash
# รัน validation ทั้งหมดของโปรเจกต์ (format, clippy, tests, schema)
just check
```

---

## Cognitive Guardrail

- อ่าน `docs/specs/cognitive_guardrail_architecture.md` ก่อนแตะ P0.1–P0.3
- Router ต้องตัดสินก่อน acquisition หรือ tool dispatch
- SQLite Correction Ledger เป็น enforcement authority; Vault/Markdown เป็น audit projection

## กฎการโค้ดดิ้ง (Coding Guidelines)
- พัฒนาแบบ TDD (Red -> Green -> Refactor)
- โค้ด Rust ต้องจัดรูปแบบตาม `rustfmt` และผ่าน `clippy` 0 warnings
- เก็บ error handling ชัดเจนผ่าน `thiserror` หรือ `anyhow` ตามความเหมาะสมของเลเยอร์
