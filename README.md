# bl1nk-kept

[Specification](SPEC.md) · [CLI Guide](get-start.md) · [Schema](schema/README.md) · [Benchmarks](benchmarks/README.md)

**bl1nk-kept** คือ Rust workspace แบบ Local-first ที่เป็นทั้ง **Cognitive & Semantic Guardrail** (ป้องกัน AI Agent ตีความคำสั่งกำกวมผิด และยับยั้งพฤติกรรมการอ่านข้อมูลสูญเปล่า) ควบคู่กับ **Intelligent Context & Knowledge Vault Management** (FFF + Tree-sitter + SQZ Judge Engine) เพื่อให้มนุษย์และ AI ทำงานร่วมกันได้อย่างแม่นยำและประหยัดโทเค็นสูงสุด

---

## Architecture Flow

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

## Crate Responsibilities

| Crate | บทบาทและหน้าที่หลัก |
| --- | --- |
| [`kept-core`](crate/kept-core) | **Domain & Intelligence Core:** จัดการ Observation Model (`file://`, `symbol://`), FFF Adapter, Context Registry (SQLite/Memory), Search Engine (BM25/FTS/Vector) และ Judge Admission Engine |
| [`kept-doc`](crate/kept-doc) | **Universal Document IR:** แปลงโครงสร้างเอกสาร Markdown, NFM, DOCX, PDF, HTML ให้อยู่ในรูปแบบ Abstract Intermediate Representation (Pure Library) |
| [`kept-cli`](crate/kept-cli) | **Command-line Interface (`kept`):** CLI tool สำหรับ developer และ pipeline automation |
| [`kept-mcp`](crate/kept-mcp) | **Stdio MCP Server (`bl1nk-kept-mcp`):** ช่องทางเชื่อมต่อ AI Agent / Client กับคลัง workspace พร้อมระบบ Context Admission Gate |

---

## Key Capabilities

### 1. Progressive Duplicate Detection
- Pipeline ตรวจสอบ 4 ขั้นตอนแบบ read-only ปลอดภัยต่อไฟล์ต้นทาง: `size bucket` → `partial SHA-256` → `full SHA-256` → `group evidence`
- จัดหมวดหมู่ชัดเจน: `same_name`, `near_name`, `same_content` และ `hard_link`
- รับประกันความปลอดภัย: ไม่มีการแก้ไขหรือลบไฟล์ต้นทางของผู้ใช้โดยอัตโนมัติ

### 2. Thai-Aware & Hybrid Semantic Search
- ผสาน BM25 inverted index เข้ากับ Thai bigram tokenization, synonym expansion และ n-gram fuzzy candidate filtering
- คะแนนและผลลัพธ์โปร่งใส: near match ยังคงเป็น candidate ที่ตรวจสอบได้ ไม่ถูกแปลงเป็นข้อมูลจริงโดยพลการ
- รองรับ Dense Vector Search และ Reranking ผ่าน Ollama และ Jina พร้อมแยกแจกแจงคะแนนชัดเจน

### 3. Filesystem Analytics & Naming Rules
- บันทึกสถานะโฟลเดอร์ลงใน `ScanIndex` snapshots พร้อมเก็บ structured `ScanIssue` diagnostics
- ตรวจสอบกฎการตั้งชื่อตาม profile แบบ read-only รองรับ absolute path scoping, priority resolution และ conflict detection
- เมนู Interactive TUI ใน terminal สำหรับตรวจดูการใช้พื้นที่, กลุ่มไฟล์ซ้ำ และข้อผิดพลาดในการตั้งชื่อ

### 4. Universal Document IR (Offline Conversion)
- แปลงโครงสร้างระหว่าง GitHub Flavored Markdown (GFM), Notion Markdown (NFM), DOCX และ PDF โดยไม่ต้องพึ่งพา network

---

## Getting Started

### Compile from Source

```bash
cargo build --release -p kept-cli --bin kept
```

### Basic CLI Usage

```bash
# สร้าง config เริ่มต้นและตรวจความพร้อมของระบบ
kept setup
kept doctor

# สแกนโฟลเดอร์ สร้าง persistent snapshot และเปิดเมนูตรวจสอบ
kept scan ./workspace

# ค้นหาและกรองไฟล์จาก index เดิมอย่างรวดเร็ว (ไม่ต้องสแกนซ้ำ)
kept find ./workspace --type pdf --min-size 50mb
kept find ./workspace --query "รายงานประจำปี"

# ตรวจสอบไฟล์ซ้ำและวิเคราะห์การใช้พื้นที่
kept duplicates ./workspace
kept review ./workspace

# ค้นหาคำใน Keyword Registry ด้วย Thai BM25 / Fuzzy search
kept search "คำค้นหา"
```

### Stdio MCP Server

```bash
cargo run --release -p kept-mcp --bin bl1nk-kept-mcp
```

ให้บริการเครื่องมือแปลงเอกสาร, สกัดตาราง และสืบค้นระบบไฟล์สำหรับ Agent ผ่าน JSON-RPC มาตรฐาน

---

## Development & Testing

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

---

## Documentation

| เอกสาร | รายละเอียด |
| --- | --- |
| [คู่มือคำสั่ง CLI](get-start.md) | คู่มือการใช้งานคำสั่งย่อย พารามิเตอร์ และ flags ทั้งหมดของ `kept` |
| [ข้อกำหนดผลิตภัณฑ์](SPEC.md) | สัญญาเชิงเทคนิค สถาปัตยกรรม และขอบเขตความรับผิดชอบของแต่ละ crate |
| [Registry Schema](schema/README.md) | สัญญา Draft-07 JSON Schema ที่สร้างขึ้นโดยตรงจาก Rust type model |
| [ผลการวัดประสิทธิภาพ](benchmarks/README.md) | ระเบียบวิธีทดสอบ, ผลประเมิน Thai tokenizer, ชุดข้อมูล JSONL และกราฟ |
| [คู่มือการมีส่วนร่วม](CONTRIBUTING.md) | แนวทางการพัฒนา, การทดสอบก่อน commit และสไตล์ไกด์ |
| [คู่มือ Agent](AGENTS.md) | แนวทางการทำงานร่วมกันของ Agent และมาตรฐานโค้ด Rust |

---

## License

MIT
