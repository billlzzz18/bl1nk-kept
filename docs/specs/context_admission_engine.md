# Specification: Context Admission & Judge Engine

## Problem Statement

When AI coding agents or autonomous tools read workspace files and documents repetitively across multi-step execution loops, each retrieval dumps complete, duplicate file payloads into the context window. This exhausts LLM context token budgets rapidly, escalates inference costs, and creates repetitive tool-call loops (>3 times) that degrade agent reasoning.

Additionally, ambiguous terms in agent instructions (e.g., "ทดสอบ", "ปัญหา", "ลบ") are silently misinterpreted — agents dispatch actions based on their own preferred interpretation rather than seeking clarification, violating the AGENTS.md Semantic Disambiguation requirement.

## Solution

A local-first, session-scoped Context Admission and Judge Engine in `kept-core` that monitors observed targets (`Observation`), tracks seen revisions in an in-memory/session `ContextRegistry`, and emits admission decisions (`Judge` engine):
- `Pass`: Materialize complete observation payload on first encounter or when content hash changes.
- `Reference`: Return compact 13-token reference pointer (`context://<target>@<hash>`) when identical content was already seen.
- `Delta`: Return structural diff when only partial content changed.
- `Compress`: Compress structured or document tokens when requested.
- `Warn`: Emit early warning when repeated reads on the same unmodified target reach 3 times.
- `Block` / `Drop`: Halt or suppress requests that exceed loop safety thresholds.
- `Resolve`: Return structured choices to the caller when an ambiguous term is detected — the caller must clarify before dispatching any action (ADR 0004 Tier 1).

## User Stories

1. As an AI coding agent, I want observation URIs (`file://`, `symbol://`, `search://`, `context://`) to have a deterministic string format so that I can reference and resolve targets unambiguously.
2. As an AI agent, I want the system to track content hashes (BLAKE3/SHA-256) separately from file modification timestamps so that touching a file without altering its bytes does not trigger redundant token dumps.
3. As an AI agent, I want `ContextRegistry` to record seen observations per session so that subsequent requests to the same target return compact references instead of multi-kilobyte file dumps.
4. As an LLM context consumer, I want admission decisions to be serialized deterministically as JSON so that my client/MCP interface can handle `Pass`, `Reference`, `Delta`, `Compress`, `Warn`, and `Resolve` uniformly.
5. As a developer, I want `Judge` to emit a `Warn` decision when an agent requests the same unchanged target 3 or more times so that infinite tool-calling loops terminate before wasting token quotas.
6. As an MCP client, I want `bl1nk-kept-mcp` to expose filesystem discovery tools (`filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, `filesystem_status`) backed by persistent FFF instances so that file acquisition is fast and respects `.gitignore`.
7. As a CLI user, I want `kept scan` and `kept find` to utilize the FFF-backed file inventory and preserve filter constraints (`FilterSet`) so that scan summaries and search results remain 100% backward-compatible.
8. As a maintainer, I want all context admission and filesystem contracts to run in `cargo test --workspace` and pass `just check` with zero warnings so that the codebase remains robust and regression-free.
9. As an agent operator, I want `Judge.evaluate_intent(term, action)` to return `Resolve` with ranked choices when a term is ambiguous (e.g., "ทดสอบ", "ปัญหา", "ลบ", "ทั้งหมด") so that the agent cannot dispatch actions based on its own preferred interpretation (ADR 0004 Tier 1).
10. As an agent operator, I want `Judge.evaluate_scope(scope, tool)` to return `Block` when the scope is an implicit or overly wide path (e.g., `/`, `~`, `.`) so that filesystem tools always operate on explicit canonical scopes (CLI-004).

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
  - `Resolve { term: String, choices: Vec<String>, hint: String }` — returned by `evaluate_intent()` for ambiguous terms
- Policy threshold: `view_count >= 3` triggers `AdmissionDecision::Warn`.

### 5. Semantic & Scope Disambiguation (P0.3 — ADR 0004)

**Two-tier architecture** (see `docs/adr/0004_semantic_disambiguation_two_tier_architecture.md`):

- **Tier 1 — Literal Rule** (Rust in `judge.rs`): `evaluate_intent(term, action)` checks against a static `AMBIGUOUS_TERMS` table and returns `Resolve` with ranked choices. Zero external dependencies, fully deterministic, 100% testable in unit tests.
- **Tier 2 — Intent Congruence** (host-agent boundary): Checking whether an intent is congruent with the current session goal requires LLM reasoning. This is delegated to the host-agent Skill/Hook layer. `kept-core` does **not** implement LLM calls.

**Disambiguation taxonomy** (4 categories from AGENTS.md):
| Category | Example terms | Choices surface |
|---|---|---|
| Action ambiguity | "ทดสอบ" | dogfood / cargo test / manual verification |
| Problem taxonomy | "ปัญหา" | hallucination / syntax error / task mismatch / hang |
| Mutation risk | "ลบ" | delete file / remove config / discard plan / uninstall |
| Scope creep | "ทั้งหมด" | current file / module / workspace / all crates |

**Scope Resolution**: `evaluate_scope(scope, tool)` blocks implicit/wider scopes (`/`, `~`, `$HOME`, `.`, empty string) and requires explicit canonical absolute paths (CLI-004).

**override_rate metric**: `override_rate()` returns the ratio of overridden evaluations to total evaluations. `record_override()` allows callers to signal explicit overrides. High rates indicate rules firing too aggressively.

## Testing Decisions

- Only external behavior and contract invariants are tested (Black-box & TDD contract testing).
- Contract test suites:
  - `tests/observation_contract.rs`: Verifies Target URI parsing, ContentIdentity hashing, and Observation serialization.
  - `tests/context_contract.rs`: Verifies Judge decisions (`Pass` on first read, `Reference` on second read, `Warn` on fourth read, `Resolve` for ambiguous terms, `Block` for implicit wider scopes, `override_rate` starts at 0.0).
  - `tests/fff_scanner_contract.rs`: Verifies FFF ignore semantics, binary detection, and git status reporting.
  - `tests/filesystem_mcp_contract.rs`: Verifies MCP server tool registration, `FffManager` lifecycle, and query execution.

## Out of Scope

- Remote cloud vector indexing (handled locally via sqlite/in-memory).
- LLM prompt generation or direct model inference within `kept-core`.
- Automated destructive file mutations without explicit user consent.
- Tier 2 intent congruence checks inside `kept-core` (host-agent responsibility per ADR 0004).

## Further Notes

All contracts conform to ADR 0001 (deterministic evidence benchmarks), ADR 0004 (semantic disambiguation two-tier architecture), and the repository pre-commit quality gate (`just check`).

