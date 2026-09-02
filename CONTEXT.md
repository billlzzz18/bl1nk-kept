# CONTEXT.md

Domain glossary for `bl1nk-kept`. Implementation details are intentionally excluded; this document defines ubiquitous domain language and terminology boundaries.

## Glossary

### Keyword Registry
A versioned, canonical collection of managed keywords, synonyms, and metadata rules. It defines field schemas, normalization standards, and classification policies for retrieval and domain categorization across tools.

### Universal IR (Intermediate Representation)
A format-agnostic, structured document representation. It acts as an offline bridge across diverse document types (such as Markdown, Notion blocks, DOCX, and PDF) without coupling document extraction to specific external platforms or storage formats.

### Naming Rule Scope
A declarative naming policy explicitly bound to an absolute filesystem path. It defines path-specific conventions (casing, token separators, token expansions, and similarity thresholds) and resolves conflicts using priority and path-depth hierarchy.
