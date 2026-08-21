# bl1nk-kept specification

**Workspace package version:** `0.2.0`  
**Registry schema version:** `1.2.0`  
**CLI:** `kept`

## 1. Product boundary

`bl1nk-kept` is a Rust workspace for keyword registries, filesystem analytics, duplicate detection, and offline document conversion under one CLI: `kept`.

| Crate | Product responsibility |
|---|---|
| `kept-core` | Registry model/migration/validation/search, filesystem scan/index/filter/treemap, duplicate detection, and Foundation data model |
| `kept-doc` | Universal IR and offline document conversion primitives (library only, no binaries) |
| `kept-mcp` | `bl1nk-kept-mcp` stdio MCP server exposing document and table tools |
| `kept-cli` | Supported `kept` command surface |

The product inspects evidence, confidence, comparability, and gaps. It does not veto unrelated features and does not auto-promote fuzzy, synonym, transliteration, or hypothesis candidates into a dictionary.

## 2. Supported behavior

### 2.1 Keyword registry

Registry loads and saves JSON or YAML, imports CSV, validates, analyzes, and searches. Registry schema `1.2.0` provides a Foundation profile for UTF-8/NFC normalization, glossary/provenance records, regex rules, classification policy, corpus manifest, and optional `searchPolicy`. The owner may omit `searchPolicy` to use compatibility defaults or set fuzzy minimum similarity, candidate limit, n-gram size, and posting cap for that registry. The loader migrates `1.1.0 → 1.2.0` deterministically before validation or saving.

Search uses BM25 inverted postings, Thai bigrams, synonym compatibility, and n-gram fuzzy candidate retrieval. `searchPolicy.fuzzyMinSimilarity` is compared against normalized name similarity on a `0.0..=1.0` scale; the configured policy is applied when the in-memory search index is built. The current duplicate-name defaults are threshold `0.90`, n-gram size `3`, and posting cap `256`, selected from the retained repeated experiment.

The public registry document contract is Draft-07 JSON Schema at `schema/keyword-registry.schema.json`. Its generator, consumer validation, and runtime validation boundary are defined in `schema/README.md`.

### 2.2 Filesystem analytics and duplicate detection

`kept scan <root>` creates or refreshes a persistent `ScanIndex`. The same index is the source for `kept review`, `kept find`, and `kept duplicates`; these commands do not rescan the root. A scan records root/options binding, file records, total size, and structured `ScanIssue` diagnostics. Read/metadata failures are retained as issues while other accessible paths continue scanning. Default snapshots are user-state data, not files in the scanned root; `scan --output <path>` creates a portable snapshot and its consumers accept `--index <path>`.

Duplicate detection uses this evidence pipeline:

```text
size bucket → partial SHA-256 → full SHA-256 → group evidence
```

Results distinguish `same_name`, `near_name`, `same_content`, and `hard_link`. The scan, review, search, and duplicate inspection paths never mutate scanned files.

| Current command class | Behavior |
|---|---|
| `kept scan <root>` | Creates or refreshes the reusable persistent scan index and reports deterministic delta when a compatible prior index exists |
| `kept find <root> <facts>` | Reads the index and filters by type, name, path, size, or modified-time facts |
| `kept review <root>` | Reads the index and opens an interactive menu for space, find, duplicates, scan issues, and read-only naming findings from the user config |
| `kept setup` | Creates the OS-appropriate user `config.yaml` only when it is absent, then offers an interactive choice to open it for editing |
| `kept config` | Summarizes the user-owned config, profiles, scopes, and task-level next steps without rewriting YAML |
| `kept config fields` | Lists every supported naming key with type, allowed values, and the actual default value |
| `kept config defaults show/set/unset` | Reads or changes baseline naming settings through validated task-level mutations |
| `kept config profile list/show/add/set/unset/remove` | Manages reusable naming profiles; `set` and `unset` cover the published naming field catalog |
| `kept config scope list/show/add/set/remove` | Manages user-selected absolute scopes and prints deterministic resolve precedence by depth then priority |
| `kept config scope exception add/remove` | Manages absolute exclusions below an existing scope |
| `kept config scope setting set/unset` | Applies or clears a naming override for one scope without creating a new profile |
| `kept config edit` | Opens the existing YAML with `KEPT_EDITOR`, or `nano` when no editor is configured; this is an advanced escape hatch, not the primary configuration flow |
| `kept doctor` | Diagnoses config syntax, semantic scope errors, and the editor dependency with a specific recovery instruction |
| `kept doctor --fix` | Creates a missing config or backs up an invalid config before restoring a validated starter; it never overwrites an existing user config silently |
| `kept duplicates <root>` | Reads the index, verifies only content candidates, and can export an explicit duplicate plan |
| `kept group types/list/show/add/set/remove/move` | Inspects and manages registry groups, including explicit order, at the registry path supplied by the user |
| `kept group field list/add/remove` | Manages group field schemas using validator-supported types; it preserves base search fields `id` and `aliases` |
| `kept search <registry> <query>` | Searches a validated registry through the task-first command surface |
| `kept convert <input> <output>` | Writes an explicit converted JSON or Markdown artifact |
| `kept evidence run/rescore/correct/self-test` | Maintains offline evidence runs, preserved raw JSONL, append-only corrections, and no-mutation self-tests |
| `kept corpus validate/snapshot/replay` | Validates provenance-linked assertions, saves deterministic snapshots, and materializes only accepted assertions into a dictionary |

`config.yaml` is user-owned and follows the OS-appropriate configuration directory. Starter profiles are broad and scopes start empty. The owner can manage defaults, profiles, scopes, exceptions, per-scope overrides, aliases, filename-token shortcuts, variables, prefixes, similarity, transform rules, and constraints through the `config` command family. A naming scope must contain an absolute path selected by the user; `kept` does not infer a scope from a scan root. When a matching scope exists, `review` analyses the persisted index and reports violations, deterministic proposed targets, and target collisions. It does not rename files, rewrite scanned files, or create a rollback plan.

Legacy nested commands remain accepted for existing scripts but are hidden from root help. There is no duplicate-file delete, rename, copy, hard-link replacement, or apply command in the current CLI.

### 2.3 User configuration and diagnostics

On Linux, `config.yaml` is stored at `$XDG_CONFIG_HOME/kept/config.yaml` or `~/.config/kept/config.yaml`; macOS uses `~/Library/Application Support/kept/config.yaml`, and Windows uses `%APPDATA%/kept/config.yaml`. `kept setup` is the creation path. The `kept config` command family is the primary management path, while `kept config edit` is the advanced editing path. `kept doctor` is the diagnostic path. These commands operate on the same file and never invent configuration scopes.

If the config is missing, `doctor` identifies the exact path and directs the user to `kept setup`; `doctor --fix` creates the starter file. If YAML or semantic validation fails, `doctor` reports the parser or scope error and directs the user to `kept config edit`; `doctor --fix` moves the invalid file to a non-conflicting `.invalid*.bak` sibling before restoring a validated starter. It also verifies the executable used by `config edit` and directs the user to install `nano` or set `KEPT_EDITOR` when it is absent.

### 2.4 Offline document conversion

`kept convert` converts Markdown to Universal IR JSON or Markdown round-trip output. PDF inspection is a future adapter boundary; live Notion sync is not a supported command.

## 3. Benchmark contract

`benchmarks/` retains comparable raw results, experiment artifacts, renderers, and generated charts. Synthetic search/duplicate benchmarks detect regressions; they do not establish a production SLO. The duplicate-default experiment uses public CC0 Thai corpus, two strata, and repeated measurements; it does not stand in for real user filesystem or semantic-language distributions.

Every new benchmark result must preserve raw data, command/options, input or corpus revision, environment, repetition/warm-up policy, gate status, and limitations. Charts are regenerated from current raw data; benchmark values are never edited manually.

## 4. Evidence-system target contract

The future evidence system is informed by retained Ponytail research. It adopts evidence discipline, not YAGNI feature-veto policy.

| Mechanism | Product contract |
|---|---|
| Marker and debt ledger | Read source markers into a read-only ledger with current ceiling, trigger, and upgrade path; do not block unrelated development |
| Self-test | Good and bad fixtures must prove instrument behavior before benchmark selection or gain reporting |
| Preserved evidence | Retain raw JSONL, frozen manifests, and offline rescore inputs |
| Correction loop | Append correction/supersession records; preserve raw history |
| Comparable gain | Compare baseline and candidate only under the same run contract; otherwise report `incomparable` |
| Correctness/safety gates | Integrity, no-mutation, and negative cases precede latency or score claims |

The gold corpus target is 10,000 provenance-linked reviewed assertions, not generated words and not a filesystem-only sample. It must cover canonical and variant forms, synonym/homograph/homophone/semantic relations, transliteration, command/prohibition language, named entities, numerals, ambiguity, uncertain negatives, relation boundaries, and secondary syntax/file/path/URL/duplicate signals. An accepted assertion requires source/provenance, stable locator, context or fragment hash, normalized candidate, assertion kind, reviewer decision/reason, and split. The target split is build/validation/holdout = `7,000 / 1,500 / 1,500`.

Implementation status and acceptance checklist for all future work are maintained only in `TODO.md`.
