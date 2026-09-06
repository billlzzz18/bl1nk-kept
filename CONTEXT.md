# CONTEXT.md

Domain glossary for `bl1nk-kept`. Implementation details are intentionally excluded; this document defines ubiquitous domain language and terminology boundaries.

## Glossary

### Keyword Registry

A versioned, canonical collection of managed keywords, synonyms, and metadata rules. It defines field schemas, normalization standards, and classification policies for retrieval and domain categorization across tools.

### Universal IR (Intermediate Representation)

A format-agnostic, structured document representation. It acts as an offline bridge across diverse document types (such as Markdown, Notion blocks, DOCX, and PDF) without coupling document extraction to specific external platforms or storage formats.

### Naming Rule Scope

A declarative naming policy explicitly bound to an absolute filesystem path. It defines path-specific conventions (casing, token separators, token expansions, and similarity thresholds) and resolves conflicts using priority and path-depth hierarchy.

### Deep Research & Evidence Logging

A strict protocol for investigating external libraries, tools, and architectures. Every research phase MUST persist its full findings, architecture models, and verification evidence directly into `docs/research/<topic>_report.md` before proceeding to implementation. No transient research or unrecorded fan-outs.

### Tool & Context Throttling Rule (SQZ Guardrail)

To eliminate repetitive tool calls and wasteful token burn:
1. **Local Path First**: Always check existing research in `docs/research/` and memory before invoking external discovery tools (`WebFetch`, `WebSearch`, search tools).
2. **Strict Loop Limit**: Any repetitive sequence of identical or redundant search/read/fetch operations exceeding **3 attempts** must be halted immediately. Drop tool execution, fallback to local disk/code truth, and proceed with code implementation.
3. **SQZ MCP Enforcement**: Prefer `sqz_read_file` and `sqz_grep` for files >2KB or repeat reads to enforce byte-exact deduping and AST-aware compression.
