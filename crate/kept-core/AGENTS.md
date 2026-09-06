# kept-core — คู่มือ Agent

คำแนะนำสำหรับ Agent เมื่อแก้ไขหรือขยายความสามารถของ crate `kept-core`

---

## กฎและข้อควรระวัง
1. **รักษาความเสถียรของ Observation Model:** Struct `Observation`, `Target`, `Revision` เป็นรากฐานของทั้งระบบ ห้ามแก้ breaking changes โดยไม่มี ADR รองรับ
2. **แยกแยะ look vs view:**
   - `look`: ต้องเบา ไม่ดึง content ทั้งไฟล์ คืนเฉพาะ metadata, revision, outline
   - `view`: ใช้เมื่อต้องการเนื้อหาจริงตาม range หรือ symbol
3. **TDD:** เขียน test เคส edge cases (เช่น hash mismatch, cyclic reference, revision drift) เสมอ
4. **คำสั่งทดสอบ:** `cargo test -p kept-core`
