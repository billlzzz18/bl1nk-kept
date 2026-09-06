# kept-doc — คู่มือ Agent

คำแนะนำสำหรับ Agent ในการพัฒนา crate `kept-doc`

---

## กฎและข้อควรระวัง
1. **Pure Library Constraint:** `kept-doc` ต้องเป็น library เท่านั้น ห้ามสร้าง CLI binary หรือเปิด network I/O
2. **Deterministic Output:** การแปลงเอกสารเข้า Universal IR ต้องได้ผลลัพธ์แน่นอนและคงทนต่อ whitespace
3. **คำสั่งทดสอบ:** `cargo test -p kept-doc`
