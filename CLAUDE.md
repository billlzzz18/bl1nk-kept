# bl1nk-kept — คำแนะนำ Claude Code

คู่มือสำหรับ Claude Code ในการพัฒนาและทดสอบระบบ `bl1nk-kept`

---

## สถาปัตยกรรมระบบ (System Architecture)
- **Acquisition:** FFF Adapter จัดการ filesystem (`look`, `view`, `grep`, `watch`)
- **Structure:** Tree-sitter วิเคราะห์ Code AST และ `kept-doc` วิเคราะห์ Document IR
- **Model:** แปลงข้อมูลเป็น `Observation` พร้อม `Target` URI (`file://`, `symbol://`, `document://`)
- **Judge & Admission:** เปรียบเทียบกับ Context Registry แล้วส่ง treatment (`PASS`, `REFERENCE`, `DELTA`, `COMPRESS`) ผ่าน `kept-mcp`

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

# รันตรวจสอบแบบเร็วระหว่างพัฒนา (fmt + test เท่านั้น)
just quick

# รันตรวจสอบเอกสาร (links, version-drift, eol)
just docs
```

### Full Validation Suite
```bash
# รัน validation เต็มรูปแบบก่อน push/release (format, clippy, tests, schema, docs)
just check
```

---

## กฎการโค้ดดิ้ง (Coding Guidelines)
- พัฒนาแบบ TDD (Red -> Green -> Refactor)
- โค้ด Rust ต้องจัดรูปแบบตาม `rustfmt` และผ่าน `clippy` 0 warnings
- เก็บ error handling ชัดเจนผ่าน `thiserror` หรือ `anyhow` ตามความเหมาะสมของเลเยอร์
