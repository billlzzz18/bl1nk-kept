# bl1nk-kept — คู่มือการส่งต่องาน Agent

เอกสารแนวทางการทำงานร่วมกันของ AI Agent ภายใน Workspace `bl1nk-kept`

---

## 1. ลำดับขั้นตอนการเริ่มงาน (Start Here)
1. **อ่าน `.agents/MEMORY.md`:** ระบุ Requirement ID และตรวจสอบตารางสถานะ `MISSING`/`PARTIAL`
2. **อ่าน `TODO.md`:** เลือก checkbox ลำดับความสำคัญสูงสุดที่สอดคล้องกับ Requirement
3. **อ่าน `SPEC.md` และ `plan.md`:** ทำความเข้าใจ boundary และ contract ก่อนลงมือแก้ไข
4. **อ่าน `.learnings/ERRORS.md`:** หลีกเลี่ยงข้อผิดพลาดเดิมที่เคยถูกบันทึกไว้

---

## 2. วงจรการทำงาน (Work Loop & Acceptance Criteria)
1. **TDD เคร่งครัด:** เขียน failing test ที่ตรงเป้าหมายก่อนเขียน production code เสมอ
2. **Implement เล็กที่สุด:** ปรับแก้โค้ดเท่าที่จำเป็นเพื่อให้ test ผ่าน
3. **มาตรฐานข้อมูล (Observation-first):** ทุกการ acquire ข้อมูลต้องผ่านโครงสร้าง `Observation` และระบุด้วย `Target` URI เสมอ
4. **ตรวจสอบจริง:** รัน `cargo test --workspace` (หรือ `just check`) เพื่อยืนยันว่าไม่มี regression
5. **อัปเดตสถานะ:** ทำเครื่องหมายใน `TODO.md` และบันทึก public change ใน `CHANGELOG.md`

---

## 3. สถาปัตยกรรมและกฎสำคัญ
- **FFF เป็น Acquisition Core:** ใช้ `look` สำหรับตรวจ identity/outline (ต้นทุนต่ำ) และ `view` เมื่อต้องการ materialize context
- **Judge เป็นตัวตัดสิน Context:** ไม่ส่งข้อความซ้ำซ้อน พิจารณา treatment: `PASS`, `REFERENCE`, `DELTA`, `COMPRESS`
- **ห้ามตัดสินใจแทนผู้ใช้:** เมื่องานมีทางเลือก เชิงนโยบาย ให้เสนอ blank checkbox `[ ]` ให้ผู้ใช้เลือก
