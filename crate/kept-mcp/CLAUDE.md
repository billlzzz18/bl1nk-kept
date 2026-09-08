# kept-mcp — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-mcp`

---

## Scope

- `kept-mcp` เป็น stdio MCP transport; domain logic อยู่ `kept-core`
- filesystem tools ต้องผ่าน Context Admission/Guardrail ก่อนส่ง payload

## คำสั่งทดสอบ

```bash
cargo test -p kept-mcp
```
