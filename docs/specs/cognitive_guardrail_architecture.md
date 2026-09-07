# Cognitive Guardrail Architecture

## Purpose

`bl1nk-kept` prevents agents from repeating confirmed misunderstandings, acquiring context without an outcome, and acting on ambiguous intent or scope. This specification defines the guardrail contract. It extends the existing Context Admission and Judge Engine specification; it does not replace scan, retrieval, document, or vault contracts.

## Problem Taxonomy

| Category | Failure | Required guardrail result |
|---|---|---|
| Lexical Trap | A word or phrase has multiple plausible technical meanings. | Resolve intent before selecting an action. |
| Scope Confusion | An action uses an inferred, stale, or wider scope than the caller selected. | Require canonical explicit scope; use `look` before `view` when discovery is needed. |
| Behavioral Waste | An agent rereads unchanged material in its active context or acquires material without creating a durable outcome. | Return a reference, block the repeat, or require an outcome record. |
| State & Memory Drift | A confirmed correction is absent, ignored, or contradicted by a later agent. | Block contradictory action and return correction evidence. |

## Invariants

1. A confirmed correction is immutable. Later evidence may supersede it but must create a new record linked to the prior record; it must never overwrite history.
2. If a correction establishes `A ≠ a`, no admission or action may claim, infer, or execute `A = a` while that correction is active.
3. SQLite is enforcement authority. An append-only Vault/Markdown projection is an audit artifact and can be rebuilt from SQLite.
4. A guardrail decision occurs before context acquisition, tool dispatch, or mutation planning.
5. Ambiguous intent or scope must resolve to an explicit contract before execution. The system must not select a convenient interpretation.
6. `look` returns bounded outline or metadata. `view` returns content only after a resolved target and scope exist.
7. Every acquisition requiring durable work must create or update an outcome record: an observation, decision, evidence record, correction, report, plan, or explicit discard reason.
8. Destructive actions retain existing explicit-consent requirements. This specification adds no autonomous mutation path.

## Components

### Guardrail Router

The router receives an `ActionRequest` before acquisition or dispatch. It normalizes the requested intent, target, scope, and declared outcome. It invokes checks in dependency order:

1. Correction Ledger check.
2. Intent disambiguation check.
3. Scope resolution check.
4. Context Registry and acquisition-accountability check.
5. Existing Judge admission treatment.

The router returns one deterministic result:

- `Allow`: request has resolved intent, canonical scope, no conflicting correction, and an admissible acquisition plan.
- `Resolve`: caller must provide one explicit intent or scope choice.
- `Reference`: previously admitted unchanged context is available by pointer.
- `RequireOutcome`: caller must name durable outcome or explicit discard reason before acquisition.
- `Block`: request conflicts with an active correction, violates scope, or exceeds a configured repeated-acquisition safety threshold.

`Resolve`, `Reference`, `RequireOutcome`, and `Block` are terminal for the original request. A caller must submit a new request with the required evidence; the router never silently changes intent or scope.

### Correction Ledger

The ledger records confirmed user or verified-system corrections. Each record contains:

```text
correction_id: UUID
subject: canonical target or semantic key
assertion: canonical statement
rejected_assertion: canonical statement
status: active | superseded
confidence: confirmed
source: user | verified_system
created_at: UTC timestamp
supersedes: optional correction_id
evidence_target: context://, file://, symbol://, or document:// URI
outcome_target: durable report, plan, evidence, or vault URI
```

Only `active` records enforce blocks. A superseding record must reference prior `correction_id`, preserve both assertions, include replacement evidence, and atomically update active status. Deletion and in-place edits are forbidden.

Exact matching compares canonical subject plus canonical assertion. Semantic matching may identify candidate conflicts but must return `Resolve` with the candidate evidence; it must not auto-create or auto-supersede corrections.

### Context Registry and Outcome Ledger

The existing session `ContextRegistry` remains authority for seen target revisions and view counts. The guardrail adds outcome linkage for acquisitions:

```text
acquisition_id: UUID
target: canonical Target URI
revision_hash: content identity
session_id: session identifier
requested_at: UTC timestamp
outcome_kind: observation | decision | evidence | correction | report | plan | discard
outcome_target: URI
```

An unchanged target already present in session returns `Reference` instead of materializing content. A caller that requests fresh acquisition without a declared outcome receives `RequireOutcome`. A caller can record `discard` only with a reason; discard records remain auditable.

### Intent and Scope Resolver

Intent resolution is a closed set of action contracts, not free-form agent preference. Candidate actions must state required input, allowed scope, read or mutation behavior, and expected durable outcome. When multiple contracts match, router returns `Resolve` with options and does not call a tool.

Scope resolution canonicalizes caller-supplied paths and target URIs. It rejects implicit reuse of previous scan roots, home roots, filesystem roots, and scopes broader than the declared action contract. Discovery requests use `look`; content acquisition uses `view` after a concrete target exists.

## Persistence and Projection

SQLite stores corrections, supersession edges, acquisition records, outcomes, and guardrail decisions. All enforcement queries read this store inside the decision path.

Vault/Markdown export is append-only and contains human-readable correction and decision history with record IDs, timestamps, evidence URIs, outcome URIs, and supersession links. Export failure must not weaken SQLite enforcement; it creates a retryable projection issue. Projection never writes back into enforcement tables.

## Decision Flow

```text
ActionRequest
  ├─ active Correction Ledger conflict? ── yes ── Block with correction evidence
  ├─ intent maps to one contract? ───────── no ─── Resolve intent options
  ├─ scope canonical and contract-valid? ── no ─── Resolve scope requirement
  ├─ unchanged target in ContextRegistry? ─ yes ── Reference existing context
  ├─ durable outcome declared? ──────────── no ─── RequireOutcome
  └─ Judge admission ──────────────────────────── Allow / Pass / Delta / Compress
```

## Error Contract

Every non-allow result includes: decision kind, canonical target or candidate targets, rule identifier, evidence URI when available, and one required next action. Error text must distinguish agent behavior from system failure. A correction conflict is never silently downgraded to a warning.

## Verification Contract

Implementation uses TDD and proves these behaviors:

1. An exact active correction blocks conflicting action before acquisition callback executes.
2. A superseding correction preserves prior record and changes enforcement only after replacement evidence is stored.
3. Ambiguous intent returns `Resolve` without tool dispatch.
4. Implicit or stale scope returns `Resolve` without filesystem access.
5. `view` without resolved target fails; `look` may return bounded discovery data.
6. An unchanged context target returns `Reference` and does not materialize duplicate content.
7. Acquisition without outcome returns `RequireOutcome`; acquisition with durable outcome records linkage.
8. SQLite enforcement continues when Vault/Markdown projection fails.
9. Vault/Markdown projection is reproducible from SQLite ledger records.
10. All serialization and decision ordering are deterministic.

## Boundaries

- No remote LLM inference in `kept-core`.
- No implicit semantic correction enforcement from unconfirmed similarity.
- No destructive mutation from router decisions.
- No replacement of existing FFF, Observation, ContextRegistry, Judge, or consent contracts; this layer composes them.
