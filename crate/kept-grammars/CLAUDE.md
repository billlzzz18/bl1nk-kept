# kept-grammars

Crate สำหรับ Data Schema, YAML/TOML Serialization และ Validation Rules ของการตั้งชื่อและ Keyword ใน `bl1nk-kept`

---

## Single Responsibility & Architectural Invariant

- **Data Models & Validation Only:** โมดูลนี้รับผิดชอบเฉพาะ Types (`UserConfig`, `NamingProfile`, `NamingSettings`, `NamingScope`, `TokenReplacement`), Language Grammar Specs (`LanguageConfig`, `builtin_languages()`), และ Static Validation (`validate_naming_settings`)
- **ห้ามใส่ Evaluation Logic:** การคำนวณ Path Matching, Cascading Scope Priority, Levenshtein Similarity, และ Token Replacement Execution ต้องอยู่ที่ `kept-core::policy` เท่านั้น

---

## Schema & Serialization Invariants

- **YAML Persistence Stability:** ฟิลด์ทั้งหมดใน `UserConfig` และ `NamingSettings` แมปตรงกับไฟล์คอนฟิกผู้ใช้ (`~/.config/kept/config.yaml`) การเปลี่ยนชื่อฟิลด์ต้องมี `#[serde(alias = "...")]` เพื่อรองรับ backward compatibility เสมอ
- **Mandatory Validation Gate:** หากเพิ่ม setting ใหม่ใน `types.rs` ต้องเพิ่ม logic ตรวจสอบใน `validate_naming_settings()` ของ `config.rs` และเพิ่มเทสต์ใน `tests/grammar_comprehensive.rs` เสมอ

---

## Targeted Verification

```bash
cargo test -p kept-grammars
cargo test -p kept-grammars --test grammar_comprehensive
```
