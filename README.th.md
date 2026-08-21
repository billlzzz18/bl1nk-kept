# bl1nk-kept

[English](README.md) · [Specification](SPEC.md) · [Roadmap](TODO.md) · [Research](research/README.md) · [Benchmarks](benchmarks/README.md) · [Schema](schema/README.md)

**bl1nk-kept** คือ Rust workspace สำหรับตรวจสอบ keyword, filesystem, duplicate และเอกสารแบบ offline ผ่าน CLI เดียวชื่อ `kept`.

| Crate | หน้าที่ |
|---|---|
| `kept-core` | model ของ registry, search, filesystem analytics, duplicate detection และ data foundations |
| `kept-doc` | แปลงเอกสารแบบ offline ผ่าน Universal IR |
| `kept-cli` | command-line interface ชื่อ `kept` |

## ทำไมต้อง bl1nk-kept

**เส้นทางหลักฐานเดียวสำหรับชื่อ ไฟล์ และเอกสาร** `kept` เริ่มจาก keyword registry และ filesystem index แล้วแยกสัญญาณออกจากกัน: ความคล้ายทางภาษาไม่ใช่ความเท่ากันของเนื้อหา และ duplicate ทุกกลุ่มมีหลักฐานจาก size, partial hash, full hash และ group evidence

**ค้นหาภาษาไทยโดยไม่ซ่อนความไม่แน่ใจ** BM25, Thai bigram, synonym compatibility และ n-gram fuzzy retrieval ทำงานร่วมกัน แต่ near match ยังคงเป็น candidate ไม่ถูกยกเป็น canonical data แบบเงียบ ๆ

**งานเอกสารแบบ offline ที่ตรวจย้อนกลับได้** Universal IR ให้เป้าหมายแบบมีชนิดกับการแปลง Markdown แทนการมองเอกสารเป็น plain text ส่วน PDF adapter ที่วางไว้แยก native extraction, page diagnostics และ OCR opt-in ออกจากกัน

**default ต้องอธิบายได้** provenance ของ public corpus, repeated experiment, raw benchmark artifact และ public schema ที่ generate ทำให้ choice ของ implementation กลับไปตรวจและแก้ได้

## เริ่มต้น

```bash
kept setup
kept doctor
kept scan ./workspace
kept review ./workspace
kept find ./workspace --type pdf --min-size 50mb
kept duplicates ./workspace
```

`config.yaml` เป็นไฟล์ของผู้ใช้: `kept config` สรุป profiles และ scopes; ใช้ `kept config defaults`, `config profile` และ `config scope` เพื่อจัดการค่าแบบ task-level; `config edit` เป็นทางเลือกสำหรับ YAML ขั้นสูง. `kept doctor --fix` กู้ config ที่หายหรือเสียโดยเก็บ backup ก่อน. ใช้ `kept group` เพื่อจัดการ registry groups และ field schema. `review` ใช้ config นี้เพื่อรายงาน naming findings แบบ read-only; ไม่มีคำสั่ง rename หรือ apply ในรุ่นนี้.

## จุดเริ่มต้นตามงาน

| ต้องการดูอะไร | เริ่มที่ไหน |
|---|---|
| ขอบเขตผลิตภัณฑ์และสถาปัตยกรรม | [SPEC.md](SPEC.md) |
| งานค้างและงานที่ปิดแล้ว | [TODO.md](TODO.md) |
| ขั้นตอนสำหรับผู้ร่วมพัฒนา | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Research ที่ใช้ตัดสินใจ implementation | [research/](research/README.md) |
| ผลเปรียบเทียบที่ทำซ้ำได้ | [benchmarks/](benchmarks/README.md) |
| contract เอกสาร registry สำหรับ public use | [schema/](schema/README.md) |
| คู่มือคำสั่ง CLI ทั้งหมด | [get-start.md](get-start.md) |
| การตัดสินใจจาก research | [ADR](docs/adr/) |
| เรื่องที่ยังไม่ตัดสินใจก่อน implementation | [plan.md](plan.md) |

## License

MIT
