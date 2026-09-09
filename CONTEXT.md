# CONTEXT.md

Domain glossary for `bl1nk-kept`. Implementation details are intentionally excluded; this document defines ubiquitous domain language and terminology boundaries.

---

## Core Domain Terms

### Keyword Registry

A versioned, canonical collection of managed keywords, synonyms, and metadata rules. Defines field schemas, normalization standards, and classification policies for retrieval and domain categorization.

### Universal IR (Intermediate Representation)

A format-agnostic, structured document representation. Acts as an offline bridge across diverse document types (Markdown, Notion blocks, DOCX, PDF) without coupling document extraction to specific external platforms or storage formats.

### Naming Rule Scope

A declarative naming policy explicitly bound to an absolute filesystem path. Defines path-specific conventions (casing, token separators, token expansions, similarity thresholds) and resolves conflicts using priority and path-depth hierarchy.

### Observation

The fundamental data unit for acquired context. Every piece of information entering the system is wrapped as an `Observation` with a `Target` URI (`file://`, `symbol://`, `document://`, `search://`, `context://`).

### Target URI

A structured identifier for the source of an observation. Schemes:
- `file:///path/to/file` — local filesystem
- `symbol://crate/module/symbol` — code symbol (function, struct, etc.)
- `document:///path/to/doc` — document source
- `search://query` — search result origin
- `context://target@revision` — previously admitted context (for dedup)

### Judge / Admission

The decision engine that evaluates whether an observation should be admitted into the active context stream. Treatments:
- **PASS** — observation is new and valuable
- **REFERENCE** — observation already in context, return pointer
- **DELTA** — observation is a superset/update of existing context
- **COMPRESS** — observation can be summarized to save tokens
- **DROP** — observation is noise or redundant
- **BLOCK** — observation contradicts a correction in the ledger

### Correction Ledger

An immutable SQLite-backed store of factual corrections. When the user corrects the agent, the correction is recorded here. Future observations that contradict active corrections are blocked before reaching the agent.

### FFF (Foresight, Filter, Fusion)

The filesystem acquisition engine. Provides `look` (cheap metadata/outline scan) and `view` (full content materialization) operations. Manages file indexing, Git status, and ignore rules.

### kept-core

Core library crate. Houses keyword validation, filesystem scanning (via FFF), observation model, Judge engine, context registry, and token counting.

### kept-cli

CLI binary crate. Exposes user-facing commands (`kept scan`, `kept find`, `kept review`, etc.).

### kept-mcp

MCP server binary crate. Exposes tools over stdio for AI agent integration (filesystem search, document conversion, etc.).

### kept-doc

Document conversion library. Handles Notion, Markdown, PDF, and DOCX conversion to/from Universal IR.

### kept-grammar

Grammar types library. Houses keyword validation rules, naming profiles, and config resolution logic.

---

## Planned Terms (Not Yet Implemented)

These terms describe systems that are designed but not yet fully operational. Use them in design discussions, but do not reference them as working systems.

- **Vault Package Manager** — Git-first package manager for knowledge packages (planned)
- **Notion Safe Sync** — Idempotent, conflict-aware sync between local vault and Notion (planned)
- **Hybrid Semantic Search** — Combined lexical + dense vector retrieval with reranking (planned)
- **Gold Corpus** — Verified assertion dataset for benchmarking search quality (planned)
