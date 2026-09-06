# kept-doc

Universal Document IR และ Offline Converter Engine สำหรับจัดการเอกสารความรู้ (Pure Library)

---

## องค์ประกอบหลัก (Core Components)

1. **Universal Intermediate Representation (IR)**
   - โครงสร้างเอกสารเชิงนามธรรม (Blocks, Headings, Paragraphs, Lists, Code, Tables)
   - ไม่ผูกติดกับรูปแบบไฟล์ต้นทาง

2. **Document Converters & Parsers**
   - **Markdown / NFM:** รองรับ CommonMark และ Notion Flavored Markdown
   - **Office & PDF:** สกัดเนื้อหาจาก DOCX, PDF, HTML เข้าสู่ IR

3. **Outline & Slicing**
   - ดึงโครงร่างเอกสารเพื่อสร้าง `document://` target และแยก slice ส่งให้ Judge
