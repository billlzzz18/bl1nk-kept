# kept-core — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-core`

---

## ขอบเขตโมดูล
- `observation/`: Target parser, Content identity, Evidence, Observation payload
- `scanner/`: Filesystem traversal และ FFF wrapper
- `foundation.rs`: Evidence classification และ verification ledger
- `search.rs` / `semantic.rs`: Retrieval engine (BM25, Bigram, Vector)

## คำสั่งทดสอบ
```bash
cargo test -p kept-core
cargo test -p kept-core --test <test_name>
```
