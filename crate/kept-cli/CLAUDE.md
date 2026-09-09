# kept-cli — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-cli`

---

## Contract

- CLI ต้อง delegate domain logic ไป `kept-core`; ห้ามแยก Judge หรือ registry logic ใน command handler
- `--json` ต้องคืน JSON document เดียวและ parse ได้
- mutation command ต้องรักษา explicit confirmation contract

## ขอบเขตโมดูล

- `commands/duplicates.rs`: Duplicate scan, simulate, trash/delete/hardlink, rollback
- `commands/fs.rs`: Filesystem scan (`kept scan`), duplicate scan dispatch
- `commands/config.rs`: Config load/save for mutation policies
- `commands/evidence.rs`: Evidence run manifest, rescore, correction history
- `corpus.rs`: Corpus validate, snapshot save, replay
- `helpers.rs`: Confirmation prompt, interactive terminal check

## Interactive vs Non-Interactive

- Interactive: แสดง menu ผ่าน `dialoguer::Select`, confirmation ผ่าน `confirm_action()`
- Non-interactive: ต้อง `--action <name> --yes` — ห้าม lack `--yes` ใน non-terminal stdin

## คำสั่งทดสอบ

```bash
cargo test -p kept-cli
cargo run -p kept-cli -- --help
```
