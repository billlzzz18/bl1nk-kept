# kept-grammar

Grammar types, keyword validation rules, naming profiles, และ config resolution สำหรับ `bl1nk-kept`

---

## องค์ประกอบหลัก (Core Components)

1. **Types (`types.rs`)**
   - `UserConfig`, `NamingProfile`, `NamingSettings`, `PolicyError`
   - `TokenReplacement`, `TokenReposition`, `SimilarityRule`, `SimilaritySettings`
   - `NamingScope`, `ScopeOverrides`, `ResolvedNamingRule`, `NamingFinding`

2. **Config I/O (`config.rs`)**
   - `load_user_config`, `save_user_config` — YAML round-trip
   - `default_user_config_path` — platform-specific config path (Windows/macOS/Linux)
   - `create_user_config_if_missing` — สร้าง starter config ครั้งแรกโดยไม่ overwrite
   - `starter_config_yaml` — default config template

3. **Validation (`config.rs`)**
   - `validate_naming_settings` — ตรวจ grammar rules (case, separator, unicode, similarity, length, regex, extensions)

---

## ขอบเขต

- **มี:** Types, config I/O, validation rules, starter config
- **ไม่มี:** Business logic (analyze_naming, resolve_naming_rule) — อยู่ใน `kept-core::policy`
- **ไม่มี:** Binary — เป็น pure library เท่านั้น

---

## คำสั่งทดสอบ

```bash
cargo test -p kept-grammar
cargo test -p kept-grammar --test grammar_comprehensive
```

---

## Dependencies

- `serde`, `serde_json`, `serde_yaml` — serialization
- `regex` — stem regex validation
- `thiserror` — error types
- `toml` — config format support
