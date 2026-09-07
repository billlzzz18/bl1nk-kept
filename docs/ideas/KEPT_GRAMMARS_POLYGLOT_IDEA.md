# ไอเดียการวิจัย: สถาปัตยกรรม Tree-sitter Polyglot Grammars สำหรับ `bl1nk-kept`

**สถานะ:** Backlog / Architecture Evaluation  
**วันที่บันทึก:** 2026-09-08  
**แหล่งข้อมูลอ้างอิง:** [Zed Grammars Crate Architecture](https://github.com/zed-industries/zed/tree/main/crates/grammars)

---

## 1. ที่มาและความกังวล (Motivation & Risk of Forgetting)

ในปัจจุบัน `bl1nk-kept` โฟกัสการวิเคราะห์ AST และ Symbols สำหรับโค้ด **Rust** เป็นภาษาแรกผ่าน `tree-sitter-rust` ใน `kept-core`

**ความเสี่ยงที่ต้องบันทึกไว้ (Risk):**
- หากในอนาคต `bl1nk-kept` ต้องรองรับหลายภาษา (เช่น TypeScript, Python, Go, Markdown, C/C++) การเพิ่ม tree-sitter bindings ทีละตัวตรงๆ ใน `kept-core` จะทำให้ Compile Time และ Dependency Graph ของ Core Engine หนักเกินไป
- โมเดลของ Zed Editor (`crates/grammars`) แก้ปัญหานี้ด้วยการแยก Crate เฉพาะสำหรับการรวบรวม Grammars ทั้งหมด

---

## 2. การวิเคราะห์เปรียบเทียบ (Zed vs kept)

| มิติ | Zed `crates/grammars` | `bl1nk-kept` (แนวทางที่แนะนำ) |
|---|---|---|
| **เป้าหมาย** | Syntax highlighting + Code editing ทุกภาษา | สกัด Outline, Definition, Scope (`symbol://`) สำหรับ AI Agent |
| **จำนวนภาษา** | 50+ ภาษา | Bounded set (Rust, Python, TS/JS, Markdown) |
| **จุดติดตั้ง** | รวมศูนย์ไว้ใน crate `grammars` | ระยะแรก: อยู่ใน `kept-core` (เฉพาะ Rust)<br>ระยะขยายตัว: แยกเป็น `crate/kept-grammars` เมื่อรองรับ $\ge 3$ ภาษา |

---

## 3. สัญญาณเตือนเมื่อถึงเวลาต้องทำ `kept-grammars` (Trigger Conditions)

ไม่ต้องกังวลว่าจะลืม เพราะเรากำหนด Trigger Conditions ไว้ชัดเจน เมื่อถึงจุดเหล่านี้ให้แตก Crate ทันที:
1. **เมื่อเริ่ม Phase ขยายภาษา (Polyglot Support):** เมื่อผู้ใช้ต้องการให้ `look` และ `outline` สกัด symbol ของ Python / TypeScript / Go
2. **เมื่อ Compile Time ของ `kept-core` เกิน 30 วินาที:** เนื่องจากการคอมไพล์ C parser ของ tree-sitter หลายตัว
3. **เมื่อต้องการทำ Dynamic / WASM Grammar Loading:** โหลด grammar แบบ on-demand โดยไม่ต้อง build ฝังลงใน Binary หลัก

---

## 4. แผนงานในอนาคต (Future Action Plan)
เมื่อถึงเฟส Polyglot:
- [ ] สร้าง `crate/kept-grammars` ใน Workspace `Cargo.toml`
- [ ] ย้าย `tree-sitter-rust` และเพิ่ม parser ของภาษาเป้าหมายเข้าสู่ `kept-grammars`
- [ ] Export Trait แบบรวมศูนย์ `GrammarProvider` ให้ `kept-core` เรียกใช้
