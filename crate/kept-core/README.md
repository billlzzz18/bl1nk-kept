# kept-core

Core domain และ Engine หลักของ `kept` รับผิดชอบด้านความรู้, สถานะทรัพยากร และการคัดกรองบริบทเข้า AI (Context Admission)

---

## องค์ประกอบหลัก (Core Components)

1. **Observation & Evidence Model (`observation/`)**
   - มาตรฐานข้อมูลกลาง (`Observation`) ประกอบด้วย Source, Target URI, Revision, Content Identity, Structure และ Evidence
   - Target locator มาตรฐาน: `file://`, `symbol://`, `search://`, `document://`, `context://`

2. **Filesystem & Acquisition (`scanner/`, `source_graph/`)**
   - FFF Adapter: รองรับ `look` (metadata/outline เบาแรง) และ `view` (content materialization)
   - Resource State & Watcher: ติดตามการเปลี่ยนแปลงและจัดการ revision token

3. **Search & Indexing Engine (`search/`, `semantic.rs`)**
   - FTS, BM25, Thai bigram tokenizer, Vector search และ Keyword Registry

4. **Judge Engine & Context Registry**
   - เปรียบเทียบประวัติสิ่งที่ Agent เคยเห็น
   - เลือก Treatment: `PASS`, `REFERENCE`, `DELTA`, `COMPRESS`, `WARN`, `BLOCK`
