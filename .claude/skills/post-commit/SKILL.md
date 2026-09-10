---
name: post-commit
description: Use after every git commit. Enforces post-commit checklist. Invoke automatically after commit or manually.
triggers:
  - after commit
  - post commit
  - commit done
  - just committed
role: enforcer
scope: workflow
output-format: checklist
---

# Post-Commit Enforcer

Runs mandatory checks after every commit. Prevents state divergence and forgotten updates.

## Tool Selection Rules

| Task | Use | When |
|------|-----|------|
| Check commit status | **Bash** | git log -1 --oneline |
| Verify TODO.md | **Read** | Check for matching items |
| Update TODO.md | **Edit** | Mark completed items [x] |
| Check CHANGELOG.md | **Read** | See if public API changed |
| Clean temp files | **Bash** | git status, rm temp files |

## Checklist (run in order)

```
1. VERIFY commit succeeded — git log -1 --oneline
2. CHECK TODO.md — any items matching committed changes?
3. UPDATE TODO.md — mark completed items [x]
4. CHECK CHANGELOG.md — does commit affect public API/behavior?
5. IF public change → update CHANGELOG.md
6. VERIFY no temp files — git status should be clean
7. REPORT: commit hash, TODO updates, CHANGELOG status
```

## Hard Rules

- **NEVER** skip TODO.md update after a code change commit
- **NEVER** leave untracked temp files from the session
- **NEVER** commit without running checks first (this is pre-commit, not post-commit)
- **ALWAYS** report what was updated

## Output Format

```
✅ Post-commit checks complete
- Commit: <hash> <message>
- TODO.md: <updated items or "no changes needed">
- CHANGELOG.md: <updated or "no public change">
- Temp files: <cleaned or "none found">
```
