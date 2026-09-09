# bl1nk-kept — คู่มือการส่งต่องาน Agent

## 0. Rust Core Standards

### 1. Safety & Error Handling

- **ห้าม panic / หลีกเลี่ยง `unwrap()`:** ให้ใช้ `?`, `match`, หรือ `if let` เสมอ
- **ห้ามกลืน error ด้วย `let _ =`:** สำหรับการทำงานที่อาจ fail ต้องส่งต่อด้วย `?` หรือ log เสมอ (ห้ามละเลยเงียบ ๆ)
- **ระวัง Index Out of Bounds:** หลีกเลี่ยง `arr[i]`ตรง ๆ ให้ใช้ `.get(i)` เพื่อความปลอดภัย

### 2. Coding Philosophy & Ergonomics

- **เน้น Correctness ก่อน Performance:** โค้ดต้องถูกต้องและอ่านเข้าใจง่ายก่อนเสมอ ยกเว้นส่วนที่เป็น hot path และมี benchmark กำกับ
- **ใช้ชื่อเต็มสำหรับตัวแปร:** ไม่ใช้ตัวย่อกำกวม (เช่น `query` แทน `q`, `buffer` แทน `buf`)
- **ห้ามสร้างไฟล์ย่อยพร่ำเพรื่อ:** พัฒนาต่อในโมดูลเดิมที่มีอยู่ เว้นแต่จะเป็น logical component ใหม่จริง ๆ
- **Variable Shadowing ใน Async/Clones:** ใช้ shadowing เพื่อจำกัด scope ของ clone:

  ```rust
  let client = client.clone();
  tokio::spawn(async move {
      client.execute().await;
  });
  ```

### 3. Module Structure

- **ห้ามสร้าง `mod.rs`:** ให้ใช้ Rust modern path convention เช่น `src/scanner.rs` คู่กับโฟลเดอร์ `src/scanner/fff.rs`

### 4. Agent Discipline

- **ห้ามคิดเองเออเองเกินสั่ง:** ทำงานตามขอบเขต (scope) ที่ตกลงกันไว้อย่างเคร่งครัด
- **ห้ามเดาคำกำกวมเป็น Action โปรดของตัวเอง:**
  - **"ทดสอบ":** อาจหมายถึง `cargo test`, Dogfooding, CLI Execution, หรือการตรวจเทียบผลลัพธ์ — ให้สังเกตบริบทเสมอ
  - **"ข้อผิดพลาด / ปัญหา":** อาจหมายถึง Agent คิดไปเอง, งานไม่ตรงบรีฟ, หรือทำงานค้าง — ไม่ใช่แค่ Syntax error
- **ค้นหาแล้วต้องบันทึก:** การอ่าน/ค้นหาข้อมูลเข้ามาแล้วไม่นำเสนอ ไม่สรุป insight หรือไม่บันทึกเป็น artifact ถือเป็น Unproductive Acquisition
- **ห้ามอ่านซ้ำถ้าของอยู่ในหัวแล้ว:** หากไฟล์เพิ่งถูกอ่านไปในเซสชันเดียวกันและยังไม่มีการแก้ไข ให้ใช้บริบทที่มีอยู่ทันที

---

## 1. Workspace Overview

| Crate | Purpose |
|-------|---------|
| `kept-core` | Core logic: keyword validation, search, filesystem, observation model, Judge engine |
| `kept-cli` | CLI binary (`kept`) — command surface สำหรับผู้ใช้ |
| `kept-mcp` | MCP server binary (`bl1nk-kept-mcp`) — long-running MCP service |
| `kept-doc` | Document sync/conversion library (Notion, Markdown, PDF, DOCX) |
| `kept-grammar` | Grammar types, keyword validation, naming profiles, config resolution |

---

## 2. Start Here

1. **อ่าน `.agents/MEMORY.md`:** ระบุ Requirement ID และตรวจสอบตารางสถานะ `MISSING`/`PARTIAL`
2. **อ่าน `TODO.md`:** เลือก checkbox ลำดับความสำคัญสูงสุดที่สอดคล้องกับ Requirement
3. **อ่าน `SPEC.md` และ `plan.md`:** ทำความเข้าใจ boundary และ contract ก่อนลงมือแก้ไข
4. **อ่าน `.learnings/ERRORS.md`:** หลีกเลี่ยงข้อผิดพลาดเดิมที่เคยถูกบันทึกไว้

---

## 3. Work Loop & Acceptance Criteria

1. **TDD เคร่งครัด:** เขียน failing test ที่ตรงเป้าหมายก่อนเขียน production code เสมอ
2. **Implement เล็กที่สุด:** ปรับแก้โค้ดเท่าที่จำเป็นเพื่อให้ test ผ่าน
3. **ตรวจสอบจริง:** รัน `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`
4. **อัปเดตสถานะ:** ทำเครื่องหมายใน `TODO.md` และบันทึก public change ใน `CHANGELOG.md`

---

## 4. Comment & Language Standards

- **Internal Rationale:** ใช้ `// NOTE-001:`, `// NOTE-002:` เรียงเลขตามประเด็น ภาษาไทย
- **Public Rustdoc (`///`):** ภาษาอังกฤษล้วน สำหรับ Public APIs, Structs, CLI Help
- **Error Messages:** ภาษาอังกฤษล้วน
- **Project Docs & Guidelines:** ภาษาไทย

---

## 5. External Tools

| Tool | Binary | Purpose |
|------|--------|---------|
| FFF MCP | `C:\Users\Admin\AppData\Local\fff-mcp\bin\fff-mcp.exe` | Filesystem search, find, grep, multi-grep, rescan |
| Serena MCP | `C:\Users\Admin\.local\bin\serena.exe start-mcp-server` | LSP, AST symbols, diagnostics, targeted edits |
| SQZ MCP | `C:\Users\Admin\.cargo\bin\sqz-mcp.exe` | Token compression, context dedup, file reading |
