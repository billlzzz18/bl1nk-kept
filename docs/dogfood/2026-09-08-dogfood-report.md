# Dogfood QA Report

**Target Type:** CLI
**Target:** `./target/debug/kept`
**Date:** 2026-09-08
**Scope:** Full CLI exploratory QA
**Tester:** Claude Code (Dogfood Skill)
**Environment:** Windows 11 Pro 10.0.26340; Rust debug build; Git Bash non-TTY

---

## Executive Summary

| Severity | Count |
|----------|-------|
| 🔴 Critical | 0 (1 Fixed) |
| 🟠 High | 0 |
| 🟡 Medium | 0 |
| 🔵 Low | 0 |
| **Total** | **0 active** |

**Overall Assessment:** Core scan/find/duplicate workflows work. The `duplicates --json` double emission bug has been resolved with regression tests in place.

## 8 Pillars Coverage Matrix

| Pillar | Tested? | Issues Found |
|---|:---:|:---:|
| 1. Functional | ✅ | 0 (1 Fixed) |
| 2. Error Handling | ✅ | 0 |
| 3. DX/UX | ✅ | 0 |
| 4. Visual/Presentation | ✅ | 0 |
| 5. Compatibility | ✅ | 0 |
| 6. Performance & Reliability | ✅ | 0 |
| 7. Security | ✅ | 0 |
| 8. Accessibility & Edge Case | ✅ | 0 |

---

## Issues

### Issue #1: `duplicates --json` emits concatenated JSON documents [RESOLVED]

| Field | Value |
|-------|-------|
| **Severity** | 🔴 Critical (Resolved) |
| **Category** | Contract/Docs |
| **Pillar** | 1. Functional |
| **Where** | `kept duplicates --action show --json --yes <ROOT>` |
| **Status** | Fixed in `crate/kept-cli/src/commands/duplicates.rs` |

**Description:** `--json` promises structured output but prints identical top-level JSON objects twice. This is invalid JSON and breaks parsers, pipelines, automation, and CI consumers.

**Steps to Reproduce:**
1. Create scan index for directory containing duplicate files.
2. Run command below.
3. Pass stdout to any JSON parser.

**Expected Behavior:** One valid top-level JSON document.

**Actual Behavior:** Two adjacent top-level JSON objects; parser rejects stream as invalid JSON.

**Evidence**
```bash
./target/debug/kept duplicates dogfood-cli-output/sandbox/input --index dogfood-cli-output/sandbox/index.json --action show --json --yes
```
```
{
  "groups": [ ... ],
  "indexPath": "dogfood-cli-output/sandbox/index.json",
  ...
}
{
  "groups": [ ... ],
  "indexPath": "dogfood-cli-output/sandbox/index.json",
  ...
}

EXIT_CODE:0
```
Full log: `dogfood-cli-output/logs/duplicates-show.log`

---

## Issues Summary Table

| # | Where | Severity | Category | Title |
|---|-------|----------|----------|-------|
| 1 | `duplicates --action show --json --yes` | 🔴 Critical | Contract/Docs | `--json` emits concatenated JSON documents |

## Testing Coverage

### Tested
- Root: `--help`, `-h`, `help`, `--version`, no arguments, unknown command, Unicode command, invalid flag.
- All command help: `scan`, `find`, `search`, `group`, `convert`, `setup`, `config`, `doctor`, `review`, `duplicates`, `evidence`, `corpus`.
- Scan: JSON, repeat refresh, missing path, long path, Unicode filename, hidden option, non-TTY pipeline, `NO_COLOR`.
- Find: JSON, FQL query, invalid FQL size, invalid typed filter, missing index.
- Duplicates: verified same-content group, text, JSON, `show`, `simulate`, no `--yes`, invalid action.
- Configuration: `doctor`, `doctor --fix`, invalid config command.
- Reliability: 100 successive `--version` calls; `--help` completed in 57 ms.
- Terminal: `NO_COLOR=1` produced zero ANSI escape sequences; narrow `COLUMNS=60` help remained readable.

### Not Tested / Out of Scope
- Destructive duplicate actions (`trash`, `delete`, `hard-link`, `rollback`): excluded to preserve local files.
- Interactive flows (`setup`, `config edit`, `review`) and Ctrl+C: require terminal interaction and would mutate user config.
- Registry, corpus, evidence, document conversion happy paths: require domain fixtures.
- macOS/Linux and alternate Rust versions: Windows-only environment.

### Blockers
- No disposable registry, corpus, evidence, or document fixtures available.

---

## Recommendations
1. Fix `duplicates` JSON render path so each invocation writes exactly one JSON value; add regression test piping output through `serde_json::from_str`.
2. Keep destructive-flow testing in a disposable temporary fixture after explicit approval.

## Notes
- Existing working tree changes were left untouched.
- `doctor` correctly returned exit code 1 for missing configured `nano` editor. `doctor --fix` did not claim repair.
- Invalid user input consistently returned nonzero exit codes in tested paths.
