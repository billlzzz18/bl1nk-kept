# bl1nk-kept — คำแนะนำ Claude Code

คู่มือสำหรับ Claude Code ในการพัฒนาและทดสอบระบบ `bl1nk-kept`

---

## Workspace Structure

```
crate/
├── kept-core/      # Core logic: keyword validation, search, filesystem, observation model
├── kept-cli/       # CLI binary (`kept`) — command surface for users
├── kept-mcp/       # MCP server binary (`bl1nk-kept-mcp`) — long-running MCP service
├── kept-doc/       # Document sync/conversion library (Notion, Markdown, PDF, DOCX)
└── kept-grammar/   # Grammar types, keyword validation, naming profiles, config
```

---

## Build & Test Commands

```bash
# คอมไพล์ทั้ง workspace
cargo build --workspace

# รัน unit tests ทั้งหมด
cargo test --workspace

# รันเฉพาะ crate
cargo test -p kept-core
cargo test -p kept-grammar
cargo test -p kept-doc
cargo test -p kept-cli
cargo test -p kept-mcp

# static analysis
cargo clippy --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

---

## Rust Coding Standards

- TDD workflow: Red → Green → Refactor
- `rustfmt` format, `clippy` 0 warnings
- Restriction lints: `clippy::unwrap_used` และ `clippy::expect_used` = warn (workspace-level)
- Error handling: `thiserror` สำหรับ library layers, `anyhow` สำหรับ application layers
- หลีกเลี่ยง `unwrap()` / `expect()` — ใช้ `?`, `match`, `if let`
- ห้ามกลืน error ด้วย `let _ =` สำหรับ operations ที่อาจ fail
- ใช้ `.get(i)` แทน `arr[i]` เพื่อป้องกัน index out of bounds
- Variable shadowing ใน async/clone scopes เพื่อจำกัด lifetime:
  ```rust
  let client = client.clone();
  tokio::spawn(async move {
      client.execute().await;
  });
  ```
- ห้ามสร้าง `mod.rs` — ใช้ Rust modern path convention
- ห้ามสร้างไฟล์ย่อยพร่ำเพรื่อ — พัฒนาต่อในโมดูลเดิม เว้นแต่ logical component ใหม่จริง

---

## Comment Standards

- **Internal Rationale:** `// NOTE-001:` เรียงเลขตามประเด็น, ภาษาไทย
- **Public Rustdoc (`///`):** ภาษาอังกฤษล้วน สำหรับ Public APIs, Structs, CLI Help
- **Error Messages:** ภาษาอังกฤษล้วน
- **Project Docs & Guidelines:** ภาษาไทย

---

## External Tools (MCP)

| Tool | Binary Path | Purpose |
|------|------------|---------|
| FFF MCP | `C:\Users\Admin\AppData\Local\fff-mcp\bin\fff-mcp.exe` | Filesystem search, find, grep, multi-grep, rescan |
| Serena MCP | `C:\Users\Admin\.local\bin\serena.exe start-mcp-server` | LSP, AST symbols, diagnostics, targeted edits |
| SQZ MCP | `C:\Users\Admin\.cargo\bin\sqz-mcp.exe` | Token compression, context dedup, file reading |

---

## Domain Glossary

ดูที่ `CONTEXT.md` สำหรับคำศัพท์เฉพาะของ Domain

---

## Agent Handoff

ดูที่ `AGENTS.md` สำหรับคู่มือการส่งต่องานระหว่าง Agent
