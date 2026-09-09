# kept-doc — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-doc`

---

## ขอบเขตโมดูล
- Parsers: แปลงไฟล์ต้นทาง (Markdown, DOCX, PDF, HTML)
- IR: Model โครงสร้างเอกสาร
- Slicing: ดึง heading, section, outline
- `kept-doc` เป็น library only; ห้ามเพิ่ม binary หรือ transport coupling
- รักษา source provenance และ deterministic IR serialization

## Converters

- `converter/notion.rs`: Notion API → IR, `NotionClient` API layer
- `converter/markdown.rs`: Markdown ↔ IR round-trip
- `converter/docx.rs`: DOCX → IR
- `converter/pdf.rs`: PDF inspection (via pdf-inspector adapter)

## Filters

- `Filter` trait: pipeline สำหรับ transform IR
- `ThaiSanitizationFilter`: ทำความสะอาด Thai text
- `MarkdownAlertFilter`: Markdown alert syntax normalization

## คำสั่งทดสอบ

```bash
cargo test -p kept-doc
```
