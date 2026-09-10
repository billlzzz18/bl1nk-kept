---
name: repository-operating-recovery
description: Use when a repository has mixed reports/snapshots/specs, missing source-of-truth navigation, absent task automation/CI/versioning/schema artifacts, unclear mutation commands, or an incomplete source ZIP. Recovers, organizes, and hardens for reliable agent or developer handoff.
triggers:
  - messy repository
  - no clear entrypoint
  - missing CI
  - broken automation
  - repository handoff
  - source ZIP
role: Repository recovery specialist — establish facts, classify files, apply operating layer with TDD
scope: Entire repository structure, automation, and handoff packaging
output-format: |
  [RECOVERY REPORT]
  Facts established: <list>
  Classification: <table>
  Contracts added: <list>
  Verification: <pass/fail per contract>
---

# Repository Operating Recovery

Turn a difficult-to-continue repository into an operating one with clear entrypoint, active spec/backlog, tested automation, and verified source handoff.

Do not use as generic refactor. Preserve supported product behavior and recover facts before changing structure.

# Repository Operating Recovery

Use this skill to turn a repository that is difficult to continue into an operating repository with a clear entrypoint, one active specification/backlog, tested automation, and a verified source handoff.

Do not use it as a generic refactor. Preserve supported product behavior and recover facts before changing structure.

## Inputs and decisions

Read repository instructions, user constraints, root README, backlog, changelog, current specs, research, benchmark assets, schema sources, CLI entrypoints, and existing automation before changing files.

Do not ask the user to reconfirm facts already stated. Make ordinary organization decisions from those requirements. Ask only when a requested destructive mutation, a credential, or an irreconcilable product decision is genuinely unresolved.

Use `scripts/repository_inventory.py <repo-root>` as a read-only first pass. Load `references/operating-layout.md` before proposing or applying moves.

## Mandatory workflow

### 1. Establish facts before design

1. Run the inventory script and inspect the complete tree.
2. Identify every real write/mutation command by reading CLI handlers. Classify each command as **read-only**, **writes an explicit output**, **mutates source**, or **unavailable**. Do not invent commands or safety policies for behavior that does not exist.
3. Locate schema source, existing generated schema, research, raw benchmark data, chart renderers, screenshots, reports, active specifications, TODOs, agent memory, task runners, lint configuration, version flow, and CI.
4. Record evidence as present, planned, generated, stale, duplicated, or unknown. Never call a target dataset, planned feature, or synthetic benchmark completed when no artifact proves it.

### 2. Classify before moving or deleting

Make a file-level migration table before changing paths.

| Class | Action |
|---|---|
| Active specification | Keep exactly one active `SPEC.md` at root. Merge still-valid rules from superseded plans. |
| Active work | Keep exactly one `TODO.md` at root. Remove completed tickets/reports from active navigation after extracting durable facts. |
| Public schema/API | Keep under `schema/` or `api/`; regenerate from code and add a drift gate when possible. |
| Research | Retain under `research/` with an index. Do not discard source-backed research. |
| Benchmark evidence | Keep raw data, renderer, chart, run metadata and limitations together in `benchmarks/`. |
| Agent memory | Keep `.learnings/` in checkout and source handoff. Never let product code read it. |
| Rebuildable output | Exclude build/cache/package artifacts from source handoff. |
| Presentation/snapshot | Keep only if an active visual, UX, or regression contract needs it; otherwise exclude from the source package. |

Do not create empty directory taxonomies. Use a directory only when a real retained asset requires it.

### 3. Apply the operating layer with TDD

Write failing repository contract tests before production changes for each new invariant. Typical invariants are: required root entrypoints exist, public schema is parseable, active docs do not mix closed reports/snapshots, tool scripts work, package retains learning/research and excludes rebuildable output.

Apply the smallest change that satisfies each contract. Preserve links and plain-text path references; add a local Markdown link checker. Use Thai `NOTE-001:` comments only for non-obvious code reasons.

Use `templates/AGENTS.md.template` and `templates/Justfile.template` only as starting points. Replace every placeholder and omit recipes that the actual repository cannot support.

### 4. Add automation that matches real contracts

Provide one task runner surface for format, tests, strict lint, repository contracts, schema generation/drift, benchmark chart regeneration, links, version checking and source packaging.

Add project configuration only when it is enforceable. In Rust repositories, make `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` runnable locally and in CI. Set MSRV only after checking source features; do not set an artificial MSRV that turns existing code into false lint failures.

Add a version updater only if it updates all actual version sources atomically and has a test. Add CI that runs the same local contracts, not a weaker parallel workflow.

### 5. Preserve benchmark meaning

Retain raw rows, commands/options, corpus or fixture revision, correctness gate status, run policy and limitations. Regenerate charts from raw data. Label mismatched datasets/configurations as `incomparable`; do not calculate gain claims from incompatible runs.

Treat seed corpora, synthetic benchmarks, reviewed gold datasets and agent hypotheses as distinct assets. Do not substitute one for another.

### 6. Verify and package

Run focused red/green tests for each new script or contract. Then run the full task runner. Verify:

1. Format, workspace tests and strict lint pass.
2. Repository, tool, schema-drift, link and version contracts pass.
3. The benchmark renderer runs from committed raw data.
4. The source package includes source, active specification, research, benchmark assets, schema, automation, and `.learnings/`.
5. The source package excludes at least VCS metadata, build outputs, Python caches, previous packages, and non-source presentation artifacts.
6. Inspect the archive listing; do not claim a clean package from the zip command alone.

## Handoff standard

Update `AGENTS.md` with a direct read order and the next TDD start point. Keep the final handoff short: state the source-of-truth layout, actual write commands, verification results, retained evidence, excluded package artifacts, and the exact next task. Attach the source archive.

Do not replace a research-derived master plan with a narrow subplan. Keep every approved workstream visible and mark unimplemented work as unimplemented.

## Resources

- `scripts/repository_inventory.py`: read-only JSON inventory.
- `references/operating-layout.md`: retention and placement policy.
- `templates/AGENTS.md.template`: agent handoff entrypoint.
- `templates/Justfile.template`: task runner contract for a Rust repository with generated schemas and benchmark charts.
