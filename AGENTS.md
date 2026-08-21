# bl1nk-kept agent handoff

This file is for the next agent session. It is not a product guide or a status report.

## Start here

1. Read `.agents/REQUIREMENTS.md` first. Select the Requirement ID being served, inspect every related `MISSING`/`PARTIAL` row, and record the acceptance evidence planned for this session. This is agent-only traceability, not product documentation.
2. Read `TODO.md`. Select the highest-priority unchecked item that satisfies the selected requirement; it is the active product-work source of truth.
3. Read the relevant section of `SPEC.md` before changing a public behavior or boundary.
4. Read `.learnings/ERRORS.md` before implementation. Keep it in the checkout and source handoff; it is agent memory, not product input.
5. Read the related file in `research/`, `schema/`, or `benchmarks/` only when the selected TODO item touches that area.

## Work loop

1. Identify the Requirement ID, exact TODO checkbox, related `MISSING`/`PARTIAL` ledger rows, and behavior that must not regress.
2. Before a public-contract change, obtain the Director's confirmation unless the existing ledger already records an approved contract.
3. Write and run a focused failing test before production code.
4. Implement the smallest change; rerun the focused test.
5. Demonstrate completed user-facing behavior with a real command and observable output. Tests verify code; do not create a persistent artifact only to demonstrate a completed change.
6. Run `just check`, then reconcile every ledger row touched in the session. A row cannot become `COMPLETE` without code path, focused test, command-level evidence, and documentation where public behavior changed.
7. Update the selected TODO checklist and requirement ledger when work state changes. Do not create product task-ticket or completion-report documents.
8. Before packaging or final handoff, verify the archive listing and run `just cli-smoke` from an extracted source archive after any public CLI change.

## Repository map

| Path | Meaning |
|---|---|
| `SPEC.md` | Product specification and public boundaries |
| `TODO.md` | Only work checklist: completed, active, blocked, and planned work |
| `research/` | Retained source-backed research |
| `schema/` | Generated public registry contract and its generation notes |
| `benchmarks/` | Raw comparison data, renderers, and generated benchmark outputs |
| `.agents/skills/` | Project-local skills; read the relevant skill before using it |
| `.agents/REQUIREMENTS.md` | Agent-only directive coverage, evidence and stop conditions; read first |
| `.learnings/` | Agent memory; retain, never expose as product data |

Use `just --list` to discover project commands. The next agent must start from the requirement ledger and matching TODO item, not from old reports, snapshots, presentations, or prior chat summaries. A passing test for one new feature never substitutes for reconciliation of earlier Director requirements.
