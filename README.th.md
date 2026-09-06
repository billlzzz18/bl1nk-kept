<!-- markdownlint-disable MD013 -->
# bl1nk-kept

[English](README.md) · [Specification](SPEC.md) · [คู่มือคำสั่ง](get-start.md) · [Schema](schema/README.md) · [Benchmarks](benchmarks/README.md)

**bl1nk-kept** คือชุดเครื่องมือและ Rust workspace แบบ local-first ที่ให้บริการทั้ง CLI (`kept`) และ stdio MCP server (`bl1nk-kept-mcp`) สำหรับจัดการ keyword registry, วิเคราะห์ระบบไฟล์, ตรวจจับไฟล์ซ้ำแบบหลายขั้นตอน และแปลงเอกสารแบบ offline

| Crate | ความรับผิดชอบ |
| --- | --- |
| `kept-core` | Data model ของ registry, BM25 / Thai bigram search, ScanIndex snapshots, ระบบตรวจจับไฟล์ซ้ำ 4 ขั้นตอน, semantic search และ data foundations |
| `kept-doc` | เครื่องมือแปลงเอกสารแบบ offline ผ่าน Universal IR รองรับ Markdown, Notion Markdown (NFM), DOCX และ PDF (Pure Library) |
| `kept-cli` | Command-line interface หลัก (`kept`) พร้อมเมนู interactive, ระบบ doctor ตรวจสอบสภาพแวดล้อม และ task-first execution |
| `kept-mcp` | Stdio Model Context Protocol (MCP) server `bl1nk-kept-mcp` ให้บริการ tools ด้าน filesystem, เอกสาร และตารางสำหรับ AI Agent |

## ความสามารถหลัก

**1. ตรวจจับไฟล์ซ้ำและสร้างหลักฐานยืนยัน (Progressive Duplicate Detection)**
- Pipeline ตรวจสอบ 4 ขั้นตอนแบบ read-only ปลอดภัยต่อไฟล์ต้นทาง: `size bucket` → `partial SHA-256` → `full SHA-256` → `group evidence`
- จัดหมวดหมู่ชัดเจน: `same_name`, `near_name`, `same_content` และ `hard_link`
- รับประกันความปลอดภัย: ไม่มีการแก้ไขหรือลบไฟล์ต้นทางของผู้ใช้โดยอัตโนมัติ

**2. ค้นหาภาษาไทยแบบไฮบริด (Thai-Aware & Hybrid Semantic Search)**

- ผสาน BM25 inverted index เข้ากับ Thai bigram tokenization, synonym expansion และ n-gram fuzzy candidate filtering
- คะแนนและผลลัพธ์โปร่งใส: near match ยังคงเป็น candidate ที่ตรวจสอบได้ ไม่ถูกแปลงเป็นข้อมูลจริงโดยพลการ
- รองรับ Dense Vector Search และ Reranking ผ่าน Ollama และ Jina พร้อมแยกแจกแจงคะแนนชัดเจน

**3. วิเคราะห์ระบบไฟล์และกฎการตั้งชื่อ (Filesystem Analytics & Naming Rules)**
- บันทึกสถานะโฟลเดอร์ลงใน `ScanIndex` snapshots พร้อมเก็บ structured `ScanIssue` diagnostics
- ตรวจสอบกฎการตั้งชื่อตาม profile แบบ read-only รองรับ absolute path scoping, priority resolution และ conflict detection
- เมนู Interactive TUI ใน terminal สำหรับตรวจดูการใช้พื้นที่, กลุ่มไฟล์ซ้ำ และข้อผิดพลาดในการตั้งชื่อ

**4. แปลงเอกสารแบบ Offline ผ่าน Universal IR**
- ใช้ Intermediate Representation (IR) ในการแปลงโครงสร้างระหว่าง GitHub Flavored Markdown (GFM), Notion Markdown (NFM), DOCX และ PDF โดยไม่ต้องพึ่งพา network

## การติดตั้งและเริ่มต้นใช้งาน

### คอมไพล์จาก Source

```bash
cargo build --release -p kept-cli --bin kept
```

### การใช้งานทั่วไป

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

### การตั้งค่าคอนฟิก (Configuration)

ไฟล์ตั้งค่าถูกเก็บในตำแหน่งมาตรฐานของระบบปฏิบัติการ (`%APPDATA%/kept/config.yaml` บน Windows, `~/.config/kept/config.yaml` บน Linux/macOS):

- `kept config`: ดูสรุปคอนฟิก, profiles และลำดับ scope hierarchy
- `kept config profile`: จัดการ profile กฎการตั้งชื่อ, ตัวพิมพ์เล็ก/ใหญ่, ตัวคั่น, shortcuts และค่า similarity threshold
- `kept config scope`: ผูก profile เข้ากับ absolute path พร้อมกำหนด priority และข้อยกเว้น
- `kept doctor --fix`: ตรวจสอบและกู้คืน config ที่เสียหายอย่างปลอดภัย พร้อมสำรองไฟล์เดิมอัตโนมัติ (`.invalid*.bak`)

## Model Context Protocol (MCP)

รัน stdio MCP server สำหรับเชื่อมต่อกับเครื่องมือภายนอก (Claude Code, Cursor, Windsurf, Zed):

```bash
cargo run --release -p kept-mcp --bin bl1nk-kept-mcp
```

ให้บริการเครื่องมือแปลงเอกสาร, สกัดตาราง และสืบค้นระบบไฟล์ผ่าน JSON-RPC มาตรฐาน

## เอกสารประกอบและสถาปัตยกรรม

| เอกสาร | รายละเอียด |
| --- | --- |
| [คู่มือคำสั่ง CLI](get-start.md) | คู่มือการใช้งานคำสั่งย่อย พารามิเตอร์ และ flags ทั้งหมดของ `kept` |
| [ข้อกำหนดผลิตภัณฑ์](SPEC.md) | สัญญาณเชิงเทคนิค สถาปัตยกรรม และขอบเขตความรับผิดชอบของแต่ละ crate |
| [Registry Schema](schema/README.md) | สัญญาณ Draft-07 JSON Schema ที่สร้างขึ้นโดยตรงจาก Rust type model |
| [ผลการวัดประสิทธิภาพ](benchmarks/README.md) | ระเบียบวิธีทดสอบ, ผลประเมิน Thai tokenizer, ชุดข้อมูล JSONL และกราฟ |
| [คู่มือการมีส่วนร่วม](CONTRIBUTING.md) | แนวทางการพัฒนา, การทดสอบก่อน commit และสไตล์ไกด์ |

## License

MIT
