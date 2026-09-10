# Operating layout reference

Use this layout only when the repository actually contains the relevant asset. Do not create empty directories to imitate a template.

| Asset | Preferred location | Retention rule |
|---|---|---|
| Active product specification | `SPEC.md` at repository root | Exactly one active specification. Fold superseded plans into it or remove them after migrating their still-valid rules. |
| Current work | `TODO.md` at repository root | Exactly one active backlog. Completed task tickets and progress reports are not an active work queue. |
| Agent handoff | `AGENTS.md` at repository root | State read order, source-of-truth map, TDD workflow, task runner commands and non-regression rules. |
| Public schema/API contracts | `schema/` or `api/` | Generate from source when possible; add a drift check. |
| Source-backed research | `research/` with `README.md` index | Retain sources and findings. Distinguish verified findings from project adaptation. |
| Benchmark/evaluation evidence | `benchmarks/data/`, `benchmarks/charts/`, `benchmarks/scripts/` | Preserve raw rows plus reproducible renderer. Regenerate charts; never hand-edit values. |
| Application fixtures/corpus | `data/` | Preserve provenance/license/checksum; do not label a seed corpus as reviewed gold data. |
| Repository tools | `tools/` | Give each script one named contract and test it. |
| Agent memory | `.learnings/` | Retain in checkout and source handoff. Never make it product input. |
| Build/cache/package output | `target/`, `__pycache__/`, `dist/` | Exclude from source handoff. |
| Completed slides/visual snapshots | Outside the source package unless they are an active UI/regression fixture | Keep only when a design/flow/output contract requires them. |

## Migration rules

1. Inventory first. Read all candidate specification, schema, research, benchmark and release files before moving or deleting them.
2. Classify each file as source of truth, retained evidence, active work, generated output, legacy duplication or unknown.
3. Preserve source-backed research and raw benchmark data. Remove reports/tickets/snapshots from active navigation only after migrating their surviving facts into the correct source-of-truth file.
4. Update every local Markdown link and plain-text path reference. Add a link checker before declaring the migration complete.
5. Do not invent a `docs/` taxonomy. Put one active spec at root; create specialized directories only when real artifacts require them.
6. Do not use a generic “clean package” rule to remove `.learnings/`; source handoff and product runtime have different boundaries.
