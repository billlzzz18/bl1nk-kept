# kept-grammar — คู่มือ Agent

คำแนะนำสำหรับ Agent เมื่อแก้ไขหรือขยายความสามารถของ crate `kept-grammar`

---

## กฎและข้อควรระวัง

1. **Pure Library ห้ามมี Binary:** kept-grammar เป็น library เท่านั้น ห้ามเพิ่ม `[[bin]]` หรือ `main.rs`
2. **Types ต้อง backward-compatible:** `UserConfig`, `NamingSettings` ถูก serialize เป็น YAML — ห้ามเปลี่ยน field name โดยไม่มี migration
3. **Validation ต้องครอบคลุม:** ทุก new setting ที่เพิ่มต้องมี validation ใน `validate_naming_settings`
4. **Orphan Rule:** `impl UserConfig` ไม่สามารถ define ใน kept-grammar ได้ถ้า types มาจากที่อื่น — ยังอยู่ใน `kept-core::policy`
5. **TDD:** เขียน test เคส edge cases เสมอก่อน implement

---

## คำสั่งทดสอบ

```bash
cargo test -p kept-grammar
cargo test -p kept-grammar --test grammar_comprehensive
cargo clippy -p kept-grammar -- -D warnings
```

---

## File Structure

```
kept-grammar/
├── Cargo.toml
├── README.md          — เอกสาร crate
├── AGENTS.md          — คู่มือนี้
├── CLAUDE.md          — แนวทาง Claude Code
└── src/
    ├── lib.rs         — module declarations + re-exports
    ├── types.rs       — UserConfig, NamingSettings, PolicyError, etc.
    └── config.rs      — load/save, validation, starter config, default paths
```
