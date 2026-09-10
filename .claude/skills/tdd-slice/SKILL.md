---
name: tdd-slice
description: Use when implementing a single behavior slice. Enforces TDD cycle and mandatory TODO.md update. Invoke for any code change.
triggers:
  - implement
  - add feature
  - fix bug
  - write code
  - TDD
  - red green refactor
role: enforcer
scope: implementation
output-format: code
---

# TDD Slice Enforcer

Locks in the correct TDD workflow: write test first, implement, verify, update TODO. Prevents shipping broken code.

## Tool Selection Rules

| Task | Use | When |
|------|-----|------|
| Understand existing code | **Serena get_symbols_overview** | See functions/structs in file |
| Find symbol usage | **Serena find_referencing_symbols** | Know where symbol is used |
| Check type/signature | **Serena find_declaration** | Need type info for symbol |
| Find implementation | **Serena find_implementations** | See where symbol is defined |
| Check diagnostics | **Serena get_diagnostics_for_file** | See errors/warnings before they appear |
| Read file efficiently | **SQZ read** | File > 2KB |
| Run tests | **Bash** | cargo test -p <crate> |
| Run clippy | **Bash** | cargo clippy --workspace |

## Workflow

```
1. READ TODO.md — identify target slice
2. WRITE failing test (Red)
3. RUN test — confirm failure with expected error
4. IMPLEMENT minimum code (Green)
5. RUN full test suite — cargo test -p <crate>
6. IF FAIL → fix, goto 5
7. RUN cargo clippy --workspace -- -D warnings
8. UPDATE TODO.md — mark slice [x]
9. REPORT: what changed, test results, clippy clean
```

## Hard Rules

- **NEVER** report "done" without running tests
- **NEVER** skip clippy check
- **NEVER** proceed to next slice without updating TODO.md
- **NEVER** implement without a failing test first (unless task explicitly skips tests)

## Error Recovery

If step 5 fails:
1. Read error output carefully
2. Fix the implementation (not the test)
3. Re-run from step 5
4. Max 3 fix attempts before asking user

If step 7 fails:
1. Fix clippy warnings
2. Re-run from step 5 (clippy changes may break tests)

## Output Format

After completing a slice:
```
✅ Slice [X.Y] complete
- Test: <test_name> — PASS
- Clippy: clean
- TODO.md: updated
- Files changed: <list>
```
