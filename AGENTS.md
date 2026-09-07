# bl1nk-kept — คู่มือการส่งต่องาน Agent

เอกสารแนวทางการทำงานร่วมกันของ AI Agent ภายใน Workspace `bl1nk-kept`

---

## 0. กฎเหล็กการเขียนโค้ด Rust (Rust Core Standards)

### 1. ความปลอดภัยและการจัดการข้อผิดพลาด (Safety & Error Handling)
- **ห้าม panic / หลีกเลี่ยง `unwrap()`:** ให้ใช้ `?`, `match`, หรือ `if let` เสมอ
- **ห้ามกลืน error ด้วย `let _ =`:** สำหรับการทำงานที่อาจ fail ต้องส่งต่อด้วย `?` หรือ log เสมอ (ห้ามละเลยเงียบ ๆ)
- **ระวัง Index Out of Bounds:** หลีกเลี่ยง `arr[i]` ตรง ๆ ให้ใช้ `.get(i)` เพื่อความปลอดภัย

### 2. ปรัชญาการพัฒนา (Coding Philosophy & Ergonomics)
- **เน้น Correctness ก่อน Performance:** โค้ดต้องถูกต้องและอ่านเข้าใจง่ายก่อนเสมอ ยกเว้นส่วนที่เป็น hot path และมี benchmark กำกับ
- **ใช้ชื่อเต็มสำหรับตัวแปร:** ไม่ใช้ตัวย่อกำกวม (เช่น `query` แทน `q`, `buffer` แทน `buf`)
- **ห้ามสร้างไฟล์ย่อยพร่ำเพรื่อ:** พัฒนาต่อในโมดูลเดิมที่มีอยู่ เว้นแต่จะเป็น logical component ใหม่จริง ๆ
- **Variable Shadowing ใน Async/Clones:** ใช้ shadowing เพื่อจำกัด scope ของ clone และลดอายุของ borrowed reference ให้อยู่เฉพาะใน closure เช่น:
  ```rust
  let client = client.clone();
  tokio::spawn(async move {
      client.execute().await;
  });
  ```

### 3. โครงสร้างโมดูล (Rust 2018+ Module Structure)
- **ห้ามสร้าง `mod.rs`:** ให้ใช้ Rust modern path convention เช่น `src/scanner.rs` คู่กับโฟลเดอร์ `src/scanner/fff.rs` เพื่อไม่ให้เปิดแท็บ `mod.rs` ซ้ำกันหลายแท็บ

### 4. วินัยของ Agent (Agent Discipline & Rule Hygiene)
- **ห้ามคิดเองเออเองเกินสั่ง:** ทำงานตามขอบเขต (scope) ที่ตกลงกันไว้อย่างเคร่งครัด
- **กฎคือ "กับดักที่ต้องเลี่ยง" (Traps to avoid):** ไม่ใช่แผนที่ architecture ที่เปลี่ยนบ่อย

---

## 1. สภาพแวดล้อมและเครื่องมือ (Environment & Tools)

เพื่อให้การทำงานร่วมกันเป็นไปอย่างราบรื่น Agent ต้องเข้าใจทรัพยากรที่มีอยู่ใน Workspace ดังนี้:

### 1. ระบบบริหารจัดการ Context (Context Management System)
- **`FFF` (Foresight, Filter, Fusion):** แกนหลักในการสำรวจและจัดการข้อมูล
- **`look`:** ใช้สำหรับสำรวจ `identity` และ `outline` (ต้นทุนต่ำ เหมาะกับการค้นหาเป้าหมาย)
- **`view`:** ใช้สำหรับดึงข้อมูลเนื้อหาจริง (Materialization) เมื่อจำเป็นต้องอ่านรายละเอียด
- **`Judge`:** ทำหน้าที่ประมวลผลและกลั่นกรอง Context ที่ส่งต่อไปยัง LLM (`PASS` / `REFERENCE` / `DELTA` / `COMPRESS`)
- **`SERENA`:** เครื่องมือหลักสำหรับ `view` operation (ดู `SERENA.md` สำหรับรายละเอียด)

### 2. Git Integration
- **`GitProvider`** และ **`GitCommands`**: เครื่องมือมาตรฐานสำหรับจัดการ Git repositories
- **`GitLogReader`**: ใช้สำหรับอ่านประวัติ commit และข้อมูลการแก้ไข

### 3. การทำงานกับไฟล์ (File System)
- **`FilesystemProvider`** และ **`FilesystemCommands`**: มาตรฐานการเข้าถึงไฟล์และไดเรกทอรี
- **Context URI Pattern**:
  - **Local Files**: `file:///path/to/file`
  - **Workspace Files**: `workspace:///src/lib.rs` (มีการ resolve absolute path ให้แล้ว)
  - **Git Files**: `git:///path/to/file` (ต้องระบุ branch/revision เสมอ)
  - **Standard Library**: `rust-std:///` (สำหรับข้อมูล stdio)
- **Rules**:
  - ห้ามส่งค่า raw path ไปยัง `view` โดยตรง ต้องผ่าน `ContextUri` เสมอ
  - ห้ามสร้าง path นอก workspace โดยตรง ต้องใช้ prefix ที่ถูกต้องเท่านั้น
  - ใช้ `look` เสมอเพื่อตรวจสอบความมีอยู่ของไฟล์ก่อน `view` เพื่อหลีกเลี่ยงค่าใช้จ่ายและข้อผิดพลาด

### 4. การทำงานกับเอกสาร (Documents)
- **`DocProvider`**, **`DocReader`**, **`DocWriter`**: เครื่องมือจัดการไฟล์เอกสาร
- **`Converter`**: เครื่องมือแปลงไฟล์ (รองรับ GFM, DOCX, PDF, NFM)
- **Context URI Pattern**:
  - **Local Documents**: `file:///path/to/document`
  - **Remote Documents**: `http://...`, `https://...`
- **Rules**:
  - ใช้ `DocReader` เพื่ออ่านเอกสาร
  - ใช้ `Converter` สำหรับการแปลงรูปแบบ
  - สามารถใช้ `view` กับ Remote Document URIs ได้โดยตรง

### 5. การทำงานกับข้อมูลตาราง (Tables)
- **`TableProvider`** และ **`TableReader`**: เครื่องมือจัดการข้อมูลตาราง
- **`QueryExecutor`**: สำหรับ Query ข้อมูลด้วย SQL
- **Context URI Pattern**:
  - **CSV**: `file:///path/to/file.csv` หรือ `workspace:///path/to/file.csv`
  - **JSON**: `file:///path/to/file.json` หรือ `workspace:///path/to/file.json`
  - **SQLite**: `file:///path/to/database.db` หรือ `workspace:///path/to/database.db`
- **Rules**:
  - **ห้ามส่ง Context แบบ full tables ไปยัง LLM โดยตรง**
  - ใช้ `view` เพื่ออ่านข้อมูลเบื้องต้น
  - ใช้ `QueryExecutor` เพื่อดึงข้อมูลที่ต้องการเฉพาะส่วนที่จำเป็น
  - สำหรับ Large Tables: ควรใช้ `QueryExecutor` ดึงข้อมูลที่ผ่านการกรอง/สรุปผลแล้ว แทนการส่งทั้งไฟล์

---

## 2. ลำดับขั้นตอนการเริ่มงาน (Start Here)

1. **อ่าน `.agents/MEMORY.md`:** ระบุ Requirement ID และตรวจสอบตารางสถานะ `MISSING`/`PARTIAL`
2. **อ่าน `TODO.md`:** เลือก checkbox ลำดับความสำคัญสูงสุดที่สอดคล้องกับ Requirement
3. **อ่าน `SPEC.md` และ `plan.md`:** ทำความเข้าใจ boundary และ contract ก่อนลงมือแก้ไข
4. **อ่าน `.learnings/ERRORS.md`:** หลีกเลี่ยงข้อผิดพลาดเดิมที่เคยถูกบันทึกไว้

---

## 3. วงจรการทำงาน (Work Loop & Acceptance Criteria)

1. **TDD เคร่งครัด:** เขียน failing test ที่ตรงเป้าหมายก่อนเขียน production code เสมอ
2. **Implement เล็กที่สุด:** ปรับแก้โค้ดเท่าที่จำเป็นเพื่อให้ test ผ่าน
3. **มาตรฐานข้อมูล (Observation-first):** ทุกการ acquire ข้อมูลต้องผ่านโครงสร้าง `Observation` และระบุด้วย `Target` URI เสมอ
4. **ตรวจสอบจริง:** รัน `cargo test --workspace` (หรือ `just check`) เพื่อยืนยันว่าไม่มี regression
5. **อัปเดตสถานะ:** ทำเครื่องหมายใน `TODO.md` และบันทึก public change ใน `CHANGELOG.md`

---

## 4. สถาปัตยกรรมและกฎสำคัญ (Core Architecture & Rules)

- **FFF เป็น Acquisition Core:** ใช้ `look` สำหรับตรวจ identity/outline (ต้นทุนต่ำ) และ `view` เมื่อต้องการ materialize context
- **Judge เป็นตัวตัดสิน Context:** ไม่ส่งข้อความซ้ำซ้อน พิจารณา treatment: `PASS`, `REFERENCE`, `DELTA`, `COMPRESS`
- **ห้ามตัดสินใจแทนผู้ใช้:** เมื่องานมีทางเลือกเชิงนโยบาย ให้เสนอ blank checkbox `[ ]` ให้ผู้ใช้เลือก
- **มาตรฐาน Comment & ภาษาใน Source Code:**
  - **Internal Rationale:** ใช้ `// NOTE-001:`, `// NOTE-002:` รันเลขตามประเด็น โดยเขียนคำอธิบายภาษาไทยสำหรับอ่านเอง/ในทีม (ห้ามใช้ `///` ปน)
  - **Public Rustdoc (`///`):** ใช้ภาษาอังกฤษล้วนสำหรับ Public APIs, Structs, CLI Help
  - **Error Messages ในโค้ด:** ใช้ภาษาอังกฤษล้วนตามมาตรฐานระบบ
  - **Project Docs & Guidelines:** ใช้ภาษาไทยสำหรับเอกสารจัดการงาน คู่มือ และคำอธิบายสำหรับผู้ใช้
