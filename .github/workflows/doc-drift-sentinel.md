---
emoji: 📡
description: Detects specification, schema, and documentation drift across bl1nk-kept pull requests
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
  - name: Pre-fetch PR changed files and metadata
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

# Doc Drift Sentinel

## Task

Audit the pull request to detect contract, version, schema, and documentation drift between Rust source code and repository documentation.

Read pre-fetched files from `/tmp/gh-aw/data/pr-meta.json` and `/tmp/gh-aw/data/pr-diff.patch`.

### Drift Checks to enforce:

1. **Workspace Version Synchronization:**
   - If `Cargo.toml` workspace version is changed, verify that `SPEC.md`, `CLAUDE.md`, and `CHANGELOG.md` declare the exact same version string.
   - If version in `SPEC.md` or `CLAUDE.md` differs from `Cargo.toml`, flag it as a version contract violation.

2. **Keyword Registry & Schema Drift:**
   - If structs or types in `crate/kept-core/src/schema.rs` or registry models are modified, verify that `schema/keyword-registry.schema.json` has been updated or regenerated via `just schema`.

3. **Workspace Member & Crate List Alignment:**
   - If `members` in root `Cargo.toml` are modified (e.g. crate rename or addition), verify that the crate table in `CLAUDE.md` and `README.md` reflects the current crate names.

4. **CLI Command & Spec Alignment:**
   - If subcommands in `crate/kept-cli/src/main.rs` or `commands.rs` are added, renamed, or modified, verify that `get-start.md`, `README.md`, and `SPEC.md` document the updated command surface.

5. **Release Notes Reconciliation:**
   - If user-facing features or bug fixes are introduced, check whether an entry is recorded under the current version section in `CHANGELOG.md`.

### Output Decision:

- **Drift detected:** Call `add_comment` with a concise report pointing out the exact mismatched files, missing documentation updates, or schema discrepancies with actionable instructions.
- **No drift:** Call `noop` with the explanation "Documentation, specifications, and schema contracts are in sync". Do not post a comment when everything is aligned.

## Safe Outputs

- Use `add_comment` to notify PR authors about documentation or schema drift.
- Call `noop` when all documentation contracts are satisfied.
