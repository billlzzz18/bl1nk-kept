# bl1nk-kept

[Specification](SPEC.md) · [CLI Guide](get-start.md) · [Schema](schema/README.md) · [Benchmarks](benchmarks/README.md)

**bl1nk-kept** คือ Rust workspace แบบ Local-first สำหรับจัดการคลังความรู้ (Vault), ระบบไฟล์, Universal Document IR และ **Intelligent Context Admission (FFF + Tree-sitter + SQZ Judge Engine)** เพื่อให้ AI Agent ได้รับ context ที่กระชับ ตรงจุด และลด token ซ้ำซ้อน

---

## สถาปัตยกรรมหลัก (Architecture Flow)

```text
               User / Agent Intent (Focus, Search, Ask)
                                │
                                ▼
                   ┌─────────────────────────┐
                   │  FFF-based Core Adapter │ (look, view, watch, grep)
                   └────────────┬────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          ▼                     ▼                     ▼
   File Operations       Code Structure        Document IR
   (FFF Primitives)       (Tree-sitter)        (kept-doc)
   look / view / diff   outline / symbol     markdown / docx
          └─────────────────────┬─────────────────────┘
                                ▼
                       Observation Model
             (Target URI, Revision, Content ID)
                                │
                                ▼
                      ┌───────────────────┐
                      │    JUDGE ENGINE   │ (State & Context Registry)
                      └─────────┬─────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          ▼                     ▼                     ▼
     [PASS]                [DELTA]              [REFERENCE]
   (New context)       (Changed symbols)      (Known & unchanged)
          └─────────────────────┬─────────────────────┘
                                ▼
                     Context Output Payload
                                │
                                ▼
                     MCP Client / Agent CLI
```

---

## โครงสร้างและหน้าที่ของแต่ละ Crate

| Crate | บทบาทและหน้าที่หลัก |
| --- | --- |
| [`kept-core`](crate/kept-core) | **Domain & Intelligence Core:** จัดการ Observation Model (`file://`, `symbol://`), FFF Adapter, Context Registry (SQLite/Memory), Search Engine (BM25/FTS/Vector) และ Judge Admission Engine |
| [`kept-doc`](crate/kept-doc) | **Universal Document IR:** แปลงโครงสร้างเอกสาร Markdown, NFM, DOCX, PDF, HTML ให้อยู่ในรูปแบบ Abstract Intermediate Representation (Pure Library) |
| [`kept-cli`](crate/kept-cli) | **Command-line Interface (`kept`):** CLI tool สำหรับ developer และ pipeline automation |
| [`kept-mcp`](crate/kept-mcp) | **Stdio MCP Server (`bl1nk-kept-mcp`):** ช่องทางเชื่อมต่อ AI Agent / Client กับคลัง workspace พร้อมระบบ Context Admission Gate |

---

## คำสั่งสำหรับพัฒนาและทดสอบ

```bash
# คอมไพล์ทุก crate ใน workspace
cargo build --workspace

# รันชุดทดสอบทั้งหมด
cargo test --workspace

# รันเฉพาะ crate
cargo test -p kept-core
cargo test -p kept-doc
cargo test -p kept-cli
cargo test -p kept-mcp

# ตรวจสอบ linter
cargo clippy --workspace
```
