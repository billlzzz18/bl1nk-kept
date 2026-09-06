# Specification: Context Admission & Judge Engine

## Problem Statement

When AI coding agents or autonomous tools read workspace files and documents repetitively across multi-step execution loops, each retrieval dumps complete, duplicate file payloads into the context window. This exhausts LLM context token budgets rapidly, escalates inference costs, and creates repetitive tool-call loops (>3 times) that degrade agent reasoning.

## Solution

A local-first, session-scoped Context Admission and Judge Engine in `kept-core` that monitors observed targets (`Observation`), tracks seen revisions in an in-memory/session `ContextRegistry`, and emits admission decisions (`Judge` engine):
- `Pass`: Materialize complete observation payload on first encounter or when content hash changes.
- `Reference`: Return compact 13-token reference pointer (`context://<target>@<hash>`) when identical content was already seen.
- `Delta`: Return structural diff when only partial content changed.
- `Compress`: Compress structured or document tokens when requested.
- `Warn`: Emit early warning when repeated reads on the same unmodified target reach 3 times.
- `Block` / `Drop`: Hault or suppress requests that exceed loop safety thresholds.

## User Stories

1. As an AI coding agent, I want observation URIs (`file://`, `symbol://`, `search://`, `context://`) to have a deterministic string format so that I can reference and resolve targets unambiguously.
2. As an AI agent, I want the system to track content hashes (BLAKE3/SHA-256) separately from file modification timestamps so that touching a file without altering its bytes does not trigger redundant token dumps.
3. As an AI agent, I want `ContextRegistry` to record seen observations per session so that subsequent requests to the same target return compact references instead of multi-kilobyte file dumps.
4. As an LLM context consumer, I want admission decisions to be serialized deterministically as JSON so that my client/MCP interface can handle `Pass`, `Reference`, `Delta`, `Compress`, and `Warn` uniformly.
5. As a developer, I want `Judge` to emit a `Warn` decision when an agent requests the same unchanged target 3 or more times so that infinite tool-calling loops terminate before wasting token quotas.
6. As an MCP client, I want `bl1nk-kept-mcp` to expose filesystem discovery tools (`filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, `filesystem_status`) backed by persistent FFF instances so that file acquisition is fast and respects `.gitignore`.
7. As a CLI user, I want `kept scan` and `kept find` to utilize the FFF-backed file inventory and preserve filter constraints (`FilterSet`) so that scan summaries and search results remain 100% backward-compatible.
8. As a maintainer, I want all context admission and filesystem contracts to run in `cargo test --workspace` and pass `just check` with zero warnings so that the codebase remains robust and regression-free.

## Implementation Decisions

### 1. Architecture & Crates
- `kept-core`: Hosts `Target`, `Revision`, `ContentIdentity`, `Observation`, `ContextRegistry`, and `Judge` admission logic without transport coupling.
- `kept-mcp`: Hosts `FffManager` and registers MCP filesystem tools (`filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, `filesystem_status`).
- `kept-cli`: Interfaces with FFF-backed scanner and snapshots.

### 2. Observation & Target Model
- `Target` enum: `File(String)`, `Symbol(String)`, `Search(String)`, `Context(String)`.
- `Revision`: Tracks timestamp and optional token identifier.
- `ContentIdentity`: Stores content length and content hash digest.
- `Observation`: Combines `id`, `source`, `target`, `revision`, `content`, `metadata`, and `provenance`.

### 3. Context Registry & State Tracking
- `SeenEntry`: Tracks target, revision timestamp, content hash, first/last seen unix timestamps, and view count.
- `ContextRegistry`: Thread-safe `RwLock<HashMap<String, SeenEntry>>` with `register(observation)` and `get(target)` methods.

### 4. Judge Admission Decisions
- `AdmissionDecision`:
  - `Pass(Box<Observation>)`
  - `Reference { target: String, hash: String, token_cost: usize }`
  - `Delta { target: String, diff: String, token_cost: usize }`
  - `Compress { target: String, compressed: String, ratio: f64 }`
  - `Drop { target: String, reason: String }`
  - `Warn { target: String, warning: String }`
  - `Block { target: String, reason: String }`
- Policy threshold: `view_count >= 3` triggers `AdmissionDecision::Warn`.

## Testing Decisions

- Only external behavior and contract invariants are tested (Black-box & TDD contract testing).
- Contract test suites:
  - `tests/observation_contract.rs`: Verifies Target URI parsing, ContentIdentity hashing, and Observation serialization.
  - `tests/context_contract.rs`: Verifies Judge decisions (`Pass` on first read, `Reference` on second read, `Warn` on fourth read).
  - `tests/fff_scanner_contract.rs`: Verifies FFF ignore semantics, binary detection, and git status reporting.
  - `tests/filesystem_mcp_contract.rs`: Verifies MCP server tool registration, `FffManager` lifecycle, and query execution.

## Out of Scope

- Remote cloud vector indexing (handled locally via sqlite/in-memory).
- LLM prompt generation or direct model inference within `kept-core`.
- Automated destructive file mutations without explicit user consent.

## Further Notes

All contracts conform to ADR 0001 (deterministic evidence benchmarks) and the repository pre-commit quality gate (`just check`).
