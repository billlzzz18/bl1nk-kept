---
name: run-kept
description: Use when building, running, testing, or smoke-testing the kept CLI. Covers cargo build, CLI commands, workspace tests, and driver-based smoke flow.
triggers:
  - run kept
  - build kept
  - test kept
  - smoke test
  - kept CLI
role: CLI tester — build, run, and verify kept CLI functionality
scope: kept-cli crate and workspace tests
output-format: |
  [KEPT SMOKE]
  Build: <pass/fail>
  --help: <output summary>
  scan/find/convert: <pass/fail per command>
  Tests: <pass/fail>
---

# Run kept

Paths relative to repository root. `kept` is a Rust CLI; use driver for build and real command smoke flow.

## Agent path

Run:

```powershell
python .claude/skills/run-kept/driver.py
```

Driver builds `kept`, runs `--help`, creates a temporary fixture, runs `scan`, `find`, and `convert`, checks exit codes and output, then removes fixture.

## Direct commands

```powershell
cargo build -q -p kept-cli --bin kept
cargo run -q -p kept-cli -- --help
```

## Tests

```powershell
cargo test -p kept-cli
cargo test --workspace
```

## Gotchas

- `kept find` defaults root to `.` when omitted.
- `cargo fmt --all -- --check` checks workspace formatting.
- `kept doctor` may select `notepad` on Windows when `nano` is absent.
