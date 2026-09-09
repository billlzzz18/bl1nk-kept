# kept-mcp — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-mcp`

---

## Scope

- `kept-mcp` เป็น stdio MCP transport; domain logic อยู่ `kept-core`
- filesystem tools ต้องผ่าน Context Admission/Guardrail ก่อนส่ง payload

## MCP Tools

| Tool | Purpose |
|------|---------|
| `scan` | Scan directory, build index |
| `find` | Find structural symbols from AST |
| `search` | Keyword/semantic search |
| `rescan` | Incremental filesystem refresh |

## การเพิ่ม Tool ใหม่

1. เพิ่ม handler ใน `handler.rs`
2. Register ใน `server.rs` tool list
3. Delegate domain logic ไป `kept-core`
4. เพิ่ม contract test ใน `tests/`

## คำสั่งทดสอบ

```bash
cargo test -p kept-mcp
```
