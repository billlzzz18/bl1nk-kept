# ไอเดียการวิจัย: การประเมิน `zvec` (Embedded Vector Database) สำหรับสถาปัตยกรรม Retrieval ของ `bl1nk-kept`

**สถานะ:** Idea / Research Backlog (ยังไม่ใช่ข้อผูกมัดในแผนหลัก)  
**วันที่บันทึก:** 2026-09-06  
**แหล่งข้อมูลอ้างอิง:** `alibaba/zvec`, `zvec-rust` (Tongyi Lab)

---

## 1. บริบทและที่มา (Context & Problem Statement)

ปัจจุบัน `bl1nk-kept` มีแผนงาน Retrieval Layer ใน **Phase 5 และ Phase 10** ซึ่งต้องรองรับการค้นหาแบบ Hybrid (Lexical + Semantic) ภายในเครื่อง (Local-first):
- **สถาปัตยกรรมปัจจุบัน:** ใช้ BM25 / FTS / Thai bigram ร่วมกับ memory array หรือ HTTP vector provider ภายนอก
- **ปัญหาที่คาดว่าจะเกิดเมื่อคลังข้อมูลขยายตัว (Scale Challenge):**
  - หากคลังเอกสารใน Vault มีขนาดใหญ่ (หมื่นถึงแสนชิ้น) การเก็บ Vector Index ใน RAM ทั้งหมดจะกินหน่วยความจำมหาศาล
  - การเปิด Daemon ภายนอก (เช่น Qdrant, Milvus, Chroma) ขัดกับหลักการ **Local-first, In-process และ Standalone Binary** ของ `kept`

---

## 2. ข้อมูลสรุปเชิงเทคนิคของ `zvec`

`zvec` เป็น **In-process Vector Database** พัฒนาด้วย C++ จาก Alibaba Tongyi Lab (Proxima Engine) โดยมี Rust Binding (`zvec-rust`):

1. **Embedded / In-process Engine:** ทำงานใน process เดียวกับแอปพลิเคชันแบบเดียวกับ SQLite (ไม่ต้องมี daemon/server แยก)
2. **DiskANN Index Support:** มี on-disk vector index ที่เก็บเวกเตอร์ส่วนใหญ่ไว้บนดิสก์และดึงเฉพาะ index cache เข้า RAM ช่วยประหยัดหน่วยความจำได้มหาศาล
3. **Write-Ahead Logging (WAL) & Concurrency:** รองรับ Concurrent Multi-process Readers และการันตีข้อมูลไม่สูญหายเมื่อ process crash
4. **Native MultiQuery (Hybrid Search):** รองรับ Dense Vector + Sparse Vector + Full-Text Search (FTS) + Scalar Filters ภายใน query เดียว
5. **Quantization:** รองรับ INT8 และ INT4 compression พร้อม Random Rotation

---

## 3. การเปรียบเทียบเชิงสถาปัตยกรรม: `bl1nk-kept` vs `zvec`

| มิติการเปรียบเทียบ | `bl1nk-kept` สถาปัตยกรรมปัจจุบัน | `zvec` Embedded DB | ข้อพิจารณาในการปรับปรุง |
|---|---|---|---|
| **Filesystem & Acquisition** | **FFF Core (`fff-search`) + Tree-sitter** ดึงโครงสร้าง, look/view, watch, hash, diff | ไม่มี filesystem engine (เป็น vector store ล้วน) | **คง FFF Core ไว้:** FFF เหมาะสมที่สุดสำหรับการสำรวจและจับการเปลี่ยนแปลงของไฟล์ |
| **Context Admission (Judge)** | **Observation Model & Judge Engine** คัดกรอง Token และจัดการ `PASS`, `REFERENCE`, `DELTA`, `COMPRESS` | มีเฉพาะ Group-by Search Deduplication | **คง Judge Engine ไว้:** `kept` ตัดสินใจเรื่อง LLM Context Budget ซึ่ง `zvec` ทำแทนไม่ได้ |
| **Vector Storage & Scale** | In-memory array / External endpoint | **DiskANN, HNSW, WAL, On-disk Index** | **`zvec` เหนือกว่า:** สามารถเป็น backend จัดเก็บเวกเตอร์ขนาดใหญ่โดยไม่เปลือง RAM |
| **Hybrid Query Pipeline** | แยกโมดูล BM25, Regex, Vector | รวม FTS + Dense + Sparse + Scalar ใน C-API เดียว | **`zvec` มีความพร้อมสูง:** ลดความซ้ำซ้อนในการเขียน Hybrid Query Merger ขึ้นเอง |
| **Toolchain & Build** | Pure Rust Workspace (คอมไพล์ผ่าน `cargo` 100%) | C++17 Core + CMake + FFI C-API (`libzvec_c_api`) | **Trade-off:** เพิ่มความซับซ้อนในการ build บน cross-platform |

---

## 4. แนวทางการนำไปประยุกต์ใช้ในอนาคต (Potential Integration Roadmap)

หากในอนาคตตัดสินใจนำ `zvec` เข้ามาเสริม ให้ดำเนินการตามลำดับดังนี้:

1. **Phase 1-4 (ปัจจุบัน):** โฟกัส FFF Core + Observation Contract + Tree-sitter + Context Registry (SQLite) ให้เสร็จสมบูรณ์ก่อน
2. **Phase 5/10 (Retrieval Engine Layer):**
   - ทำ `zvec` เป็น **Optional Storage Backend** ภายใต้ Cargo Feature Flag เช่น `cargo build --features zvec-backend`
   - ใช้ `zvec` เก็บ Embedding ของ `EvidenceRecord` และ `ContextEntry` เพื่อทำ **Semantic Reference Lookup** (ตรวจสอบว่าประเด็นนี้ AI เคยเห็นหรือยังผ่านความหมาย ไม่ใช่แค่ exact hash)
   - ใช้ DiskANN เพื่อรองรับ Local RAG คลังเอกสารขนาดใหญ่ในระดับองค์กร
