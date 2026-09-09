# kept-core — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-core`

---

## ขอบเขตโมดูล
- `observation.rs`: Target parser, Content identity, Evidence, Observation payload
- `scanner/`: Filesystem scan, duplicate detection, mutation (trash/delete/hardlink + rollback)
- `scanner/mutation.rs`: Duplicate mutation plan, simulate, execute, rollback with safety gates
- `foundation.rs`: Evidence classification, verification ledger, corpus manifest
- `search.rs` / `semantic.rs`: Retrieval engine (BM25, Thai Bigram, Vector rerank)
- `source_graph/`: Tree-sitter AST extraction, multi-language parser, graph index
- `policy.rs`: Naming policy engine (scope, cascading, aliases, shortcuts)
- `validator.rs`: Keyword validation, duplicate alias detection
- `schema.rs`: JSON Schema generation, keyword registry
- `context/`: ContextRegistry, Judge, admission decisions
- Guardrail ต้องตัดสินก่อน acquisition callback หรือ tool dispatch

## Error Handling

- Library layers (`kept-core`, `kept-grammar`): `thiserror` for error types
- Application layers (`kept-cli`, `kept-mcp`): `anyhow` for error propagation
- ห้าม `unwrap()` / `expect()` — ใช้ `?`, `match`, `if let`

## คำสั่งทดสอบ

```bash
cargo test -p kept-core
cargo test -p kept-core --test <test_name>
cargo test -p kept-core --test context_contract
```
