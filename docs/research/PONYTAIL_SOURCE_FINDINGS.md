# Ponytail source findings

**Research date:** 2026-08-19

เอกสารนี้เก็บข้อเท็จจริงจาก Ponytail ที่ต้องใช้สังเคราะห์ข้อเสนอสำหรับ bl1nk-kept. ข้อสรุปเชิงประยุกต์จะอยู่ในรายงานหลักแยกต่างหาก.

## Context7 findings

Context7 resolved Ponytail as `/dietrichgebert/ponytail` (470 code snippets; source reputation Medium; benchmark score 74.88). Query results identify the following source paths and commands.

| Area | Verified practice | Source path / command |
|---|---|---|
| Development checks | Rule copies are checked before `npm test`. | `node scripts/check-rule-copies.js`; `npm test` |
| Single-shot benchmark | Uses Promptfoo configuration; repeated evaluation can use `--repeat 10` or `--repeat 20`. | `npx promptfoo@latest eval -c benchmarks/promptfooconfig.yaml --env-file .env --repeat 10` |
| Agentic harness | Runs an offline instrument self-test before API calls; supports named tasks, arms, models, repetitions, workers and offline rescoring. | `python run.py --selftest`; `python run.py --rescore runs/<stamp>` |
| YAGNI ladder | Necessity → existing code → stdlib → platform → installed dependency → one line → minimal new code. | `.cursor/rules/ponytail.mdc`, `README.md` |
| Minimum verification | Non-trivial logic must leave one runnable check; trivial one-liners need no test. | `.cursor/rules/ponytail.mdc` |

Context7 source URLs:

- https://github.com/dietrichgebert/ponytail/blob/main/benchmarks/results/2026-06-17-cost-verification.md
- https://github.com/dietrichgebert/ponytail/blob/main/benchmarks/results/2026-06-16-correctness-gate-fix.md
- https://github.com/dietrichgebert/ponytail/blob/main/benchmarks/agentic/README.md
- https://github.com/dietrichgebert/ponytail/blob/main/.cursor/rules/ponytail.mdc
- https://github.com/dietrichgebert/ponytail/blob/main/skills/ponytail-gain/SKILL.md

## Repository source findings

### Benchmark design

Ponytail documents a correction from a single-shot prompt benchmark to an agentic benchmark. The agentic unit is a real headless Claude Code session editing an isolated copy of a pinned, real FastAPI + React template. The comparison arms are baseline, ponytail, caveman, `yagni`, and `yagni-oneliner`. Every task/arm cell has fresh workspace and agent context; the reported agentic run uses `n=4`.

The benchmark separates two tiers:

1. LOC tier: 12 feature tickets. It records source `git diff` lines and source file count. Tests are excluded from bloat metrics and `wrote_tests_rate` is recorded separately.
2. Safety tier: surgical implementation tasks executed against deterministic adversarial inputs for path traversal, rate limiting, SQL injection, tampered tokens, malformed CSV, cache behavior, and email newline injection.

Every instrument ships a `good` and a `bad` reference. `python run.py --selftest` must prove good passes and bad fails before any API call. Workspaces are preserved under `runs/<stamp>/`, allowing offline `--rescore` without paying an API again.

Ponytail adds two audit aids for non-deterministic evaluation: an over-engineering judge and a completeness judge. Both use a fixed model/temperature and published rubric; both require self-test calibration against known minimal/overbuilt or stub/complete reference pairs. The repository says these judges are to be read alongside LOC, not replace it.

The results write-up records methodology corrections and limitations. It states that a previous baseline was contaminated by a SessionStart plugin hook. The harness subsequently uses project/local setting sources and one plugin per arm for isolation. It also discloses a Windows timeout issue and partial run retention.

### Benchmark presentation

README presents a compact table of percent changes vs no-skill baseline across LOC, tokens, cost, time and safety, links to full methodology, and labels old single-shot numbers as superseded after criticism. A plain ASCII scoreboard in `skills/ponytail-gain/SKILL.md` presents benchmark medians for lines of code, cost and speed. The detailed report has task-level tables, safety table, aggregate table and limitations.

### YAGNI policy

The rule calls for reading the task and tracing the real flow before climbing the ladder. It says shortest working diff wins only after the change is understood. It rejects unnecessary abstractions, dependencies and boilerplate, prefers deletion over addition, and asks agents to mark deliberate simplifications with `ponytail:` comments naming the ceiling and upgrade path. It explicitly excludes trust-boundary validation, data-loss handling, security, accessibility, calibration and requested requirements from minimization.

## External search findings

External search found criticism focused on the original prompt benchmark. The directly relevant independent commentary says Ponytail used Promptfoo and compares baseline/no skill, caveman and ponytail; the primary repository now contains a response that changes the benchmark to agentic sessions and published limitations.

Relevant sources:

- https://blog.scottlogic.com/2026/06/16/ponytail-yagni-and-the-problem-with-prompt-benchmarks.html
- https://github.com/DietrichGebert/ponytail
- https://github.com/DietrichGebert/ponytail/blob/main/benchmarks/agentic/README.md
- https://github.com/DietrichGebert/ponytail/blob/main/benchmarks/results/2026-06-18-agentic.md
- https://github.com/DietrichGebert/ponytail/blob/main/.cursor/rules/ponytail.mdc
- https://martinfowler.com/bliki/Yagni.html

## Source URLs used in web extraction

[1]: https://github.com/DietrichGebert/ponytail
[2]: https://github.com/DietrichGebert/ponytail/blob/main/benchmarks/agentic/README.md
[3]: https://github.com/DietrichGebert/ponytail/blob/main/.cursor/rules/ponytail.mdc
[4]: https://github.com/DietrichGebert/ponytail/blob/main/benchmarks/results/2026-06-18-agentic.md


## Cross-validation from external sources

Scott Logic's June 2026 critique documents why Ponytail's original prompt benchmark was not sufficient: the baseline counted conversational alternatives and commentary as LOC, the tasks were simple, and one debounce task assumed a DOM. The article notes that the Ponytail author subsequently expanded/fixed the benchmark and revised claims. This supports treating Ponytail's later agentic benchmark as a stronger design than the earlier Promptfoo result, while still reviewing its one-model scope and synthetic task mix.

Martin Fowler's YAGNI explanation supplies a boundary that Ponytail's operational ladder does not fully replace. YAGNI rejects presumptive capability and abstraction that increase present complexity, but it does not reject refactoring, self-testing code or continuous delivery. Those practices make later change cheap enough for evolutionary design. A future-facing choice that adds no current complexity can be acceptable; an extensibility point that makes current code harder to understand is presumed unnecessary until a concrete need exists.

### Implications for adopting ideas safely

1. Do not import Ponytail's headline performance claims into bl1nk-kept. Use its test architecture, correction discipline and result layout, then measure bl1nk-kept's own workloads.
2. Keep deterministic correctness and safety gates separate from performance and complexity metrics. A faster search that loses recall, corrupts a registry or invalidates evidence is a failed candidate.
3. Treat the current Foundation migration, corpus manifest, integrity checks and experiment harness as enabling infrastructure, not YAGNI violations: each has a present user-visible purpose or protects repeated measurement. Do not add unused adapters, watch backends, ML rerankers or generalized policy layers without a real dataset/request.
4. Retain a visible debt ledger for deliberate ceilings, with a trigger and upgrade path, instead of inserting generalized abstractions before they are needed.

External sources:

[5]: https://blog.scottlogic.com/2026/06/16/ponytail-yagni-and-the-problem-with-prompt-benchmarks.html
[6]: https://martinfowler.com/bliki/Yagni.html
