# Git Safety & Destructive Command Ban Rules

## 1. Absolute Ban on `git clean -fd` (Zero-Tolerance)

All agents operating in this workspace (Antigravity IDE Main Agent, Claude Code, Subagents, Codex, and automated scripts) are **STRICTLY PROHIBITED** from running `git clean` commands under any circumstance.

### Banned Commands:
- ❌ `git clean -fd`
- ❌ `git clean -f`
- ❌ `git clean -xdf` / `git clean -Xdf`
- ❌ `git clean -n` (unless explicitly requested for dry-run viewing)
- ❌ `git reset --hard` (without explicit, prompt-verified user consent)
- ❌ `git push --force` / `git push -f`
- ❌ `rm -rf *` / `rmdir /s /q .`

## 2. Rationale

1. **Untracked File Safety:** Untracked files may contain critical in-flight work, local configs, uncommitted test files, datasets, or scratch scripts that are permanently lost if `git clean` executes.
2. **Deterministic & Surgical Engineering:** Changes must be made surgically to target files only. Never wipe the workspace clean to resolve dirty states.

## 3. Approved Safe Alternatives

- **Inspect Changes:** `git status --short` or `git status`
- **Discard specific file changes:** `git checkout -- <file>` or `git restore <file>` (targeted specific files only)
- **Delete specific obsolete files:** Delete only the named file using dedicated tools (`rm <file>` or `del <file>`).
