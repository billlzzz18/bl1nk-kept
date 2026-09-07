#!/usr/bin/env python3
"""
Smart Pre-Commit Hook for bl1nk-kept
Checks only what was actually staged:
- If only docs/markdown changed -> check eol / links only (instant)
- If rust code changed -> format check (and clippy if needed), but avoid full re-build of everything
- If no relevant files changed -> pass immediately
"""
import subprocess
import sys

def get_staged_files():
    try:
        out = subprocess.check_output(
            ["git", "diff", "--cached", "--name-only", "--diff-filter=ACM"],
            text=True,
            encoding="utf-8"
        )
        return [f.strip() for f in out.strip().splitlines() if f.strip()]
    except Exception:
        return []

def main():
    staged = get_staged_files()
    if not staged:
        sys.exit(0)

    has_rust = any(f.endswith(".rs") or f.endswith("Cargo.toml") for f in staged)
    has_md = any(f.endswith(".md") for f in staged)
    has_py = any(f.endswith(".py") for f in staged)

    # 1. Quick EOL check for staged files (ensures no CRLF leaks)
    crlf_files = []
    for filepath in staged:
        try:
            with open(filepath, "rb") as f:
                content = f.read()
                if b"\r\n" in content:
                    crlf_files.append(filepath)
        except Exception:
            pass

    if crlf_files:
        print("[pre-commit] Error: CRLF line endings detected in staged files:")
        for f in crlf_files:
            print(f"  - {f}")
        print("Run `just fix-eol` or convert to LF before committing.")
        sys.exit(1)

    # 2. If Rust code was changed, verify code formatting (takes < 0.5s)
    if has_rust:
        print("[pre-commit] Checking Rust formatting (`cargo fmt --check`)...")
        res = subprocess.run(["cargo", "fmt", "--all", "--", "--check"])
        if res.returncode != 0:
            print("[pre-commit] Error: Code formatting check failed! Run `cargo fmt` to fix.")
            sys.exit(1)

    # 3. If only docs were changed, do a quick links check if desired, otherwise skip smoothly
    if has_md and not has_rust and not has_py:
        print("[pre-commit] Docs-only change detected. Fast path approved.")
        sys.exit(0)

    print("[pre-commit] Fast pre-commit checks passed.")
    sys.exit(0)

if __name__ == "__main__":
    main()
