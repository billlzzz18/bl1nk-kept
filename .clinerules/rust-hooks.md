## Brief overview

- Rust-specific hook guidelines for automated formatting, linting, and compilation checks after file edits. Extends common hook conventions with Rust tooling.

## PostToolUse Hooks

- Configure hooks in `~/.claude/settings.json` under PostToolUse.
- **cargo fmt**: Auto-format `.rs` files immediately after any edit to Rust source.
- **cargo clippy**: Run lint checks after editing Rust files to catch issues early.
- **cargo check**: Verify compilation after changes — prefer over `cargo build` for speed.

## Hook ordering

- Run in sequence: `cargo fmt` → `cargo clippy` → `cargo check` (format first so lints/checks see formatted code).

## Scope

- Only trigger on `.rs` file edits; skip for non-Rust files to avoid wasted cycles.
