# kept-doc — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-doc`

---

## ขอบเขตโมดูล
- Parsers: แปลงไฟล์ต้นทาง (Markdown, DOCX, PDF, HTML)
- IR: Model โครงสร้างเอกสาร
- Slicing: ดึง heading, section, outline
- `kept-doc` เป็น library only; ห้ามเพิ่ม binary หรือ transport coupling
- รักษา source provenance และ deterministic IR serialization

## คำสั่งทดสอบ

```bash
cargo test -p kept-doc
```
