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
| Consumer acceptance | The same test compiles the generated schema as a consumer validator, accepts a JSON instance serialized from a `KeywordRegistry`, rejects an instance without required `version`, and rejects `searchPolicy.fuzzyMinSimilarity` outside `0.0..=1.0`. |
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

## Scope

The generated JSON Schema guarantees **document shape, declared field names, Rust-derived primitive/container types, required-field rules and declared numeric bounds**. For example, an owner may add `searchPolicy` to a registry and choose the fuzzy minimum similarity, candidate limit, n-gram size and posting cap; Draft-07 rejects a similarity outside `0.0..=1.0` before search begins. The Rust validator guarantees semantic rules such as migration compatibility, Foundation normalization policy, regex test-vector validity and a nonzero fuzzy n-gram size.

A JSON Schema artifact is not a replacement for runtime validation: JSON Schema cannot express every cross-field, migration or corpus/provenance rule. Therefore `kept` continues to use Rust deserialization, deterministic migration and semantic validation after input is read. `kept registry search` validates the registry before it builds an index, so an invalid policy cannot silently alter a result. Any new public field must add a Rust model test, regenerate this artifact and keep both consumer and runtime tests passing.
