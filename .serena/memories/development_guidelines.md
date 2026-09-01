# Development & Testing Guidelines: bl1nk-kept

## 1. Test-Driven Development (TDD)

- Always write a red (failing) test before applying production changes.
- Focus on unit tests in the respective crate first, followed by integration and contract tests. <!-- rumdl-disable-line line-length -->
- Reconcile requirements in `.agents/MEMORY.md` and tick matching items in `TODO.md`.

## 2. Cross-Platform Compatibility

- **Line Endings**: Always keep LF line endings across source files (`just fix-eol`).
- **Conditional Compilation (`cfg(unix)`)**: Any Unix-specific imports (e.g. `std::os::unix::fs::MetadataExt`, `BTreeMap` used only in Unix code paths) must be scoped inside the `#[cfg(unix)]` function or block to prevent `unused_imports` warnings on Windows under `clippy -- -D warnings`. <!-- rumdl-disable-line line-length -->
- **Paths**: Use standard `Path` and `PathBuf` abstractions rather than hardcoded path separators. <!-- rumdl-disable-line line-length -->

## 3. Dependency & Schema Conventions

- **`schemars`**: Must stay pinned to `=0.8.22` (0.8 branch) because `schemars 1.x` changes output to Draft 2020-12, breaking the public Draft-07 JSON Schema contract. <!-- rumdl-disable-line line-length -->
- **`dialoguer`**: In version `0.12.0+`, `Select::items` accepts the array directly without references (use `.items(choices)` rather than `.items(&choices)`). <!-- rumdl-disable-line line-length -->
- **Pure Library Crates**: `kept-core` and `kept-doc` must remain pure libraries without binaries. <!-- rumdl-disable-line line-length -->

## 4. Git & Commit Best Practices

- Never commit binary/database files (`*.db`, `*.sqlite`, `.headroom/`, `codeql-db/`).
- Never commit git worktrees as submodules/gitlinks (`.claude/worktrees/`).
- Always run `just check` before committing.
