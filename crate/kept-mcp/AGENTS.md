# kept-mcp — คู่มือ Agent

คำแนะนำสำหรับ Agent ในการพัฒนา crate `kept-mcp`

---

## กฎและข้อควรระวัง
1. **Strict Protocol Conformance:** ปฏิบัติตามมาตรฐาน MCP อย่างเคร่งครัด
2. **Token Efficiency:** ส่งข้อมูลผ่าน Judge Treatment เสมอเพื่อลด token context
3. **คำสั่งทดสอบ:** `cargo test -p kept-mcp`
