---
emoji: 🛡️
description: Automated Rust code quality and invariant guard for bl1nk-kept pull requests
on:
  pull_request:
    types: [opened, synchronize, reopened]
permissions:
  contents: read
  issues: read
  pull-requests: read
tools:
  github:
    mode: gh-proxy
    toolsets: [default]
steps:
  - name: Pre-fetch PR changed files and diff
    env:
      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    run: |
      mkdir -p /tmp/gh-aw/data
      gh pr diff "${{ github.event.pull_request.number }}" > /tmp/gh-aw/data/pr-diff.patch 2>/dev/null || true
      gh pr view "${{ github.event.pull_request.number }}" --json files,title,body,baseRefName,headRefName > /tmp/gh-aw/data/pr-meta.json 2>/dev/null || true
safe-outputs:
  add-comment:
    max: 1
    target: "triggering"
    hide-older-comments: true
---

# PR Quality Guard

## Task

Review pull request changes in `bl1nk-kept` to enforce codebase quality gates, safety invariants, and internal coding conventions.

Read pre-fetched files from `/tmp/gh-aw/data/pr-meta.json` and `/tmp/gh-aw/data/pr-diff.patch`.

### Core Invariants to check:

1. **Zero Panic Invariant:**
   - Inspect newly added or modified lines in production Rust files (`crate/*/src/`).
   - Flag any new `.unwrap()` or `.expect()` calls (except inside `#[cfg(test)]` modules or test files).
   - Require `?`, `match`, `if let`, or explicit error mapping instead.

2. **Single-Owner Invariant:**
   - Verify that changes in `crate/kept-cli` and `crate/kept-mcp` are strictly presentation, CLI commands, or transport layer.
   - Core domain logic, scanning, evaluation algorithms, and grammar parsing must remain in `kept-core` or `kept-grammars`.

3. **Comment & Language Standards:**
   - Verify that new internal rationale comments follow `// NOTE-xxx: [คำอธิบายภาษาไทย]`.
   - Ensure public documentation comments (`///`) and error messages are written in English.

4. **TDD Delivery Loop & TODO Sync:**
   - For behavioral changes or new features, verify that unit/integration tests are included in the diff.
   - Check if `TODO.md` status has been updated to reflect completed work slices.

### Output Decision:

- **Violations found:** Call `add_comment` with a clear, concise bulleted review detailing the file, line number, violated invariant, and suggested fix.
- **Clean / No violations:** Call `noop` with the explanation "All bl1nk-kept Rust quality invariants and coding standards verified successfully". Do not post an empty or unnecessary comment.

## Safe Outputs

- Use `add_comment` to submit quality review feedback.
- Call `noop` when all quality checks pass.
