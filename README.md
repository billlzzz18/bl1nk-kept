# Public registry JSON Schema

`keyword-registry.schema.json` is the machine-readable Draft-07 contract for the public keyword registry document. It is **generated from the Rust type model**, not manually maintained.

## Trust chain

| Layer | Enforced proof |
|---|---|
| Authoritative model | `crate/kept-core/src/schema.rs` defines `KeywordRegistry` and nested types with `Serialize`, `Deserialize` and `JsonSchema`. |
| Deterministic export | `crate/kept-core/examples/export_schema.rs` writes `schemars::schema_for!(KeywordRegistry)` as JSON. |
| Committed artifact | `just schema` regenerates this file from the current Rust model. |
| Drift prevention | `just schema-check` regenerates to a temporary file and requires byte-identical output before passing. The repository's GitHub workflow template invokes the same comparison when its owner enables it. |
| JSON Schema validity | `schema_export_tests::public_schema_is_meta_valid_and_validates_the_rust_registry_contract` validates the generated artifact against Draft-07 meta-schema through an independent `jsonschema` consumer. |
| Consumer acceptance | The same test compiles the generated schema as a consumer validator, accepts a migrated canonical registry, rejects missing `foundation`, a legacy `version`, unknown root fields, and zero/out-of-range search-policy values. |
| Runtime acceptance | `load_registry` deserializes JSON/YAML into `KeywordRegistry`, migrates supported legacy versions, then `Validator::validate_registry` enforces domain rules. Regression tests cover migration and invalid Foundation normalization/regex cases. |

Run all layers with:

```bash
just check
```

## Owner configuration

A registry owner may add this fragment, choose values that fit the corpus, and validate before searching:

```json
{
  "searchPolicy": {
    "fuzzyMinSimilarity": 0.82,
    "fuzzyCandidateLimit": 1024,
    "fuzzyNgramSize": 2,
    "maxFuzzyNgramPostings": 4096
  }
}
```

`fuzzyMinSimilarity` is compared with normalized `0.0..=1.0` similarity. Removing `searchPolicy` returns the registry to the compatibility defaults. Each `kept registry search` run validates the policy and rebuilds its in-memory candidate index from the current registry; rerun `kept fs index` when filesystem scan options or filesystem contents must be refreshed.

## ขอบเขตการตรวจ

Schema สาธารณะนี้เป็น **canonical saved-document contract** ของ registry รุ่นปัจจุบัน ไม่ใช่ schema สำหรับรับเอกสาร legacy โดยตรง. เอกสาร legacy ต้อง deserialize และ migrate ใน Rust ก่อน แล้วจึงตรวจ/บันทึกเป็น canonical registry.

Generated JSON Schema รับประกัน document shape, field names, primitive/container types, required fields, closed object boundaries, enum/constant และ numeric bounds ที่ประกาศไว้. Canonical registry ต้องมี version ที่ระบบรองรับ, Foundation profile ที่ไม่เป็น null, normalization policy ที่ตรึงค่า และ search policy ที่ไม่มี budget เป็นศูนย์.

Rust validator ยังรับผิดชอบ semantic rules ที่ Draft-07 ตรวจไม่ได้ครบ: migration compatibility, field rules ที่กำหนดจาก `baseFieldsSchema` ของแต่ละ group, duplicate IDs/aliases, relation integrity, Foundation provenance linkage, regex test vectors และเงื่อนไขข้าม field เช่น threshold ordering.

`entries` เป็น object ที่มี field contract เปลี่ยนตาม `baseFieldsSchema` ภายในแต่ละ group. Global schema จึงตรวจว่า entry เป็น object ได้ แต่ Rust validator เป็น authoritative validator ของ field dynamic เหล่านั้น. งานต่อไปควรเพิ่ม `kept registry schema <registry>` เพื่อ compile per-registry schema สำหรับ editor/CI จาก field declarations จริง.

JSON Schema ไม่แทน runtime validation. `kept` ต้อง deserialize, migrate และ semantic-validate หลังอ่าน input ทุกครั้ง. `kept registry search` ต้อง validate ก่อนสร้าง search index เพื่อไม่ให้ policy ที่ผิดเปลี่ยนผลค้นหาเงียบ ๆ. ทุก public field ใหม่ต้องเพิ่ม Rust model test, consumer negative test, regenerate artifact และทำให้ consumer/runtime parity tests ผ่าน.
