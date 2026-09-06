# kept-cli — คู่มือ Agent

คำแนะนำสำหรับ Agent ในการพัฒนา crate `kept-cli`

---

## กฎและข้อควรระวัง
1. **CLI Layer ไม่เก็บ Business Logic:** Logic ทั้งหมดต้องเรียกผ่าน `kept-core` หรือ `kept-doc`
2. **Output สวยงามและ Parse ได้:** รองรับทั้ง Human-readable และ `--json` สำหรับ automation
3. **คำสั่งทดสอบ:** `cargo test -p kept-cli`
