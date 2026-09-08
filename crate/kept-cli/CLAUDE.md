# kept-cli — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-cli`

---

## Contract

- CLI ต้อง delegate domain logic ไป `kept-core`; ห้ามแยก Judge หรือ registry logic ใน command handler
- `--json` ต้องคืน JSON document เดียวและ parse ได้
- mutation command ต้องรักษา explicit confirmation contract

## คำสั่งทดสอบ

```bash
cargo test -p kept-cli
cargo run -p kept-cli -- --help
```
