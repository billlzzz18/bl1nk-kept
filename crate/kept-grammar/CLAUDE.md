# kept-grammar — คำแนะนำ Claude Code

แนวทางการทำงานร่วมกับ `kept-grammar`

---

## ขอบเขตโมดูล

- `types.rs`: UserConfig, NamingProfile, NamingSettings, PolicyError, TokenReplacement, SimilarityRule, NamingScope
- `config.rs`: load/save config, validate_naming_settings, default paths, starter config
- ไม่มี business logic — analyze/resolve อยู่ใน `kept-core::policy`

---

## คำสั่งทดสอบ

```bash
cargo test -p kept-grammar
cargo test -p kept-grammar --test grammar_comprehensive
cargo clippy -p kept-grammar -- -D warnings
cargo fmt -p kept-grammar -- --check
```

---

## ข้อควรระวัง

1. **ห้ามเพิ่ม Binary** — pure library เท่านั้น
2. **Types backward-compatible** — field name ใน UserConfig ผูกกับ YAML serialization
3. **Validation ต้องครอบคลุม** — new setting = new validation rule
4. **Error types** ใช้ `thiserror` เท่านั้น (library layer)
5. **Comment standards** — Internal: `// NOTE-xxx:` ภาษาไทย, Public rustdoc: อังกฤษล้วน
