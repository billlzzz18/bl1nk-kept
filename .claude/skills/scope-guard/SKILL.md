---
name: scope-guard
description: Use when making code changes. Enforces scope discipline and prevents over-engineering. Invoke for any edit/write operation.
triggers:
  - edit
  - modify
  - change
  - refactor
  - scope
  - over-engineer
role: enforcer
scope: implementation
output-format: validation
---

# Scope Guard Enforcer

Prevents over-engineering and excessive changes. Locks scope to what was asked.

## Pre-Edit Checklist

Before any file modification:
```
1. COUNT files to change — if > 5, ASK user to confirm scope
2. LIST exact files — do NOT add files beyond the list
3. CHECK if new files needed — prefer editing existing files
4. CHECK if worktree/agent needed — NEVER for < 5 files
```

## Hard Rules

### File Count Limits
- **< 5 files**: Direct edits only. No worktrees, no agents.
- **5-15 files**: Direct edits preferred. Worktree only if user explicitly asks.
- **> 15 files**: Worktree allowed. Must get user approval first.

### Scope Boundaries
- **NEVER** create new files unless explicitly asked
- **NEVER** change files outside the specified list
- **NEVER** add abstractions for a single use case
- **NEVER** suggest alternative tools unless asked
- **ALWAYS** edit in place instead of creating temp files
- **ALWAYS** clean up any worktree before session end

### Change Size
- **Prefer** minimal diffs — shortest possible change
- **Prefer** editing existing code over adding new code
- **Prefer** stdlib over new dependencies
- **Prefer** one function over a new module

## Violation Detection

Flag these patterns:
- Creating `temp_*` or `tmp_*` files
- Spawning subagents for simple edits
- Adding new dependencies for < 10 lines of code
- Creating new modules for single functions
- Suggesting "alternatives" when not asked

## Output Format

Before editing:
```
Scope: <N> files
Files: <list>
New files: <none or explain why>
Worktree: <no or explain why>
```

After editing:
```
Scope: <N> files changed
Diff: <+/- lines>
New files: <0 or explain>
Dependencies: <unchanged or list additions>
```
