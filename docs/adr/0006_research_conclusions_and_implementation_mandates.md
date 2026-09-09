# บันทึกการตัดสินใจ 0006: ผลวิจัยและคำสั่ง implement ทั้งหมด

**สถานะ:** ตัดสินใจแล้ว
**วันที่:** 2026-09-10

## บริIBUT

`docs/research/` มี 7 เอกสารวิจัย บางอันถูกใช้แล้ว บางอันยังไม่มี decision record ชัดเจน ต้องบันทึกสถานะและคำสั่ง implement ให้ครบ

## ผลวิจัยและสถานะ

| Research | สถานะ | คำสั่ง |
|---|---|---|
| FFF Architecture | ✅ ใช้แล้ว | `fff-search = "0.10.6"` + `scanner/fff.rs` adapter (ADR-005) |
| Czkawka Gap Analysis | ✅ ใช้แล้ว | ทุก gap ปิดแล้ว — duplicate, near-name, persistent scan, unreadable path, naming policy, file-type integrity, review queues |
| Ponytail Source Findings | ✅ ใช้แล้ว | benchmark patterns ใช้ใน ADR-001 (baseline, self-test, raw JSONL, offline rescore) |
| Ponytail Testing/YAGNI | ✅ ใช้แล้ว | deterministic contract, skip LLM judge, YAGNI ladder (ADR-001) |
| Thai Corpus & Tokenization | 🔧 implement | seed corpus จาก PyThaiNLP — TODO 1.3 |
| PDF Inspector | 🔧 implement | `pdf-inspector` adapter + OCR opt-in — TODO 3.3 (ADR-002) |
| FFF deep research JSON | ✅ ใช้แล้ว | raw data สำหรับ fff_architecture_report.md |

## คำสั่ง Implement

### 1. Thai Seed Corpus (TODO 1.3)

**ไม่ต้องตัดสินใจ — implement ทันที**

- นำเข้า corpus จาก PyThaiNLP: words, synonyms, stopwords, Wikipedia titles
- บังคับ license gate: ทุก corpus ต้องมี source URI, license, content SHA-256, retrieval timestamp
- เก็บใน `CorpusManifest` ตาม format ที่ `docs/research/THAI_CORPUS_AND_TOKENIZATION_SOURCES.md` กำหนด
- ห้าม network-fetch ตอน scan — import ล่วงหน้าเท่านั้น

### 2. PDF Inspector + OCR (TODO 3.3, ADR-002)

**ไม่ต้องตัดสินใจ — implement ทันที**

- เพิ่ม `pdf-inspector` เป็น optional dependency (feature: `pdf-inspector`)
- Adapter: `detect_pdf` แยก TextBased/Scanned/ImageBased/Mixed + confidence
- `process_pdf` คืน Markdown รายหน้า + needs_ocr + is_complex
- OCR opt-in: vision, model-cache, model-download — ไม่เพิ่ม default dependency
- ใช้ให้หมด: native text, scanned, image-based, mixed — ไม่มี skip

## ผลกระทบ

- ทุก research มี decision record ชัดเจน — ไม่มี open question เหลือ
- Thai corpus และ PDF inspector เป็น implement task ไม่ใช่ decision point
- ADR-002 ยังใช้ได้ — เพิ่ม detail เรื่อง full OCR scope

## เอกสารอ้างอิง

- [docs/research/](../research/) — ทุกเอกสารวิจัย
- [ADR-001](./0001_deterministic_evidence_benchmark_contract.md) — benchmark patterns
- [ADR-002](./0002_optional_pdf_inspection_adapter.md) — PDF adapter
- [ADR-005](./0005_sqz_integration_and_grammar_separation.md) — SQZ + FFF integration
- [TODO.md](../../TODO.md) — implementation tasks
