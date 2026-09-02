# bl1nk-kept

[English](README.md) · [Specification](SPEC.md) · [คู่มือคำสั่ง](get-start.md) · [Schema](schema/README.md) · [Benchmarks](benchmarks/README.md)

**bl1nk-kept** คือชุดเครื่องมือและ Rust workspace สำหรับตรวจสอบ keyword, วิเคราะห์ filesystem, ค้นหาไฟล์ซ้ำ (duplicate detection) และแปลงเอกสารแบบ offline ผ่าน CLI เดียวชื่อ `kept`.

| Crate | หน้าที่ |
| --- | --- |
| `kept-core` | Data model ของ registry, BM25 / Thai bigram search, ScanIndex, ระบบตรวจจับไฟล์ซ้ำ และ data foundations |
| `kept-doc` | เครื่องมือแปลงเอกสารแบบ offline ผ่าน Universal IR และ Notion Markdown (NFM) |
| `kept-cli` | Command-line interface หลักชื่อ `kept` สำหรับผู้ใช้งาน |
| `kept-mcp` | Model Context Protocol (MCP) server `bl1nk-kept-mcp` สำหรับเชื่อมต่อ AI Agent |

## ทำไมต้อง bl1nk-kept

**เส้นทางหลักฐานเดียวสำหรับชื่อ ไฟล์ และเอกสาร** `kept` เริ่มจาก keyword registry และ filesystem index แล้วแยกสัญญาณออกจากกัน: ความคล้ายทางภาษาไม่ใช่ความเท่ากันของเนื้อหา และ duplicate ทุกกลุ่มมีหลักฐานยืนยันชัดเจนจาก size, partial hash, full hash และ group evidence

**ค้นหาภาษาไทยโดยไม่ซ่อนความไม่แน่ใจ** BM25, Thai bigram, synonym compatibility และ n-gram fuzzy retrieval ทำงานร่วมกัน โดย near match ยังคงเป็น candidate ที่ตรวจสอบได้ ไม่ถูกแปลงเป็นข้อมูลจริงโดยพลการ

**งานเอกสารแบบ offline ที่ตรวจย้อนกลับได้** Universal IR กำหนดเป้าหมายแบบ typed data สำหรับการแปลง Markdown แทนการมองเอกสารเป็นข้อความทั่วไป และมี pipeline รองรับการแยก native extraction, page diagnostics และ OCR ชัดเจน

**ค่าเริ่มต้นที่อธิบายได้เสมอ** มี public corpus provenance, ผลการทดสอบซ้ำ, raw benchmark artifacts และ JSON schema ที่สร้างขึ้นอัตโนมัติ ทำให้ทุกการตัดสินใจสามารถตรวจสอบและปรับปรุงได้

## การติดตั้งและเริ่มต้นใช้งาน

### คอมไพล์และติดตั้ง

```bash
cargo build --release -p kept-cli --bin kept
```

### คำสั่งที่ใช้งานบ่อย

```bash
kept setup
kept doctor
kept scan ./workspace
kept review ./workspace
kept find ./workspace --type pdf --min-size 50mb
kept duplicates ./workspace
kept search "คำค้นหา"
```

### การตั้งค่า (Configuration)

`config.yaml` เป็นไฟล์ของผู้ใช้:

- `kept config` แสดงสรุป profiles และ scopes ที่ตั้งค่าไว้
- `kept config defaults`, `kept config profile` และ `kept config scope` จัดการค่าคอนฟิกผ่านคำสั่ง task-level
- `kept doctor --fix` ช่วยกู้คืน config ที่หายหรือเสียหายโดยสำรองไฟล์เดิมไว้ก่อนเสมอ
- `kept review` ตรวจสอบข้อกำหนดการตั้งชื่อตาม profile ในโหมด read-only ปลอดภัยต่อไฟล์ต้นทาง

## เอกสารประกอบ

| หัวข้อ | รายละเอียด |
| --- | --- |
| [คู่มือคำสั่ง CLI](get-start.md) | คู่มือการใช้งานคำสั่งย่อยและพารามิเตอร์ทั้งหมดของ `kept` |
| [Specification](SPEC.md) | ข้อกำหนดทางเทคนิค สถาปัตยกรรม และขอบเขตการทำงานของระบบ |
| [Registry Schema](schema/README.md) | สัญญาณข้อมูล JSON Schema (Draft-07) สำหรับ Keyword Registry |
| [Benchmarks](benchmarks/README.md) | ระเบียบวิธีและผลการวัดประสิทธิภาพ พร้อมชุดข้อมูลดิบและกราฟเปรียบเทียบ |
| [คู่มือการมีส่วนร่วม](CONTRIBUTING.md) | แนวทางการพัฒนา การเขียนโค้ด และการส่ง Pull Request |

## License

MIT
