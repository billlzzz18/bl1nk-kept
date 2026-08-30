---
name: run-kept
description: Build, run, test, and smoke-test the kept CLI with representative commands.
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
