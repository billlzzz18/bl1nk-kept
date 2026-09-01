#!/usr/bin/env python3
"""Normalize text line endings (CRLF -> LF) for repository source files.

Useful after git worktree operations or cross-platform checkouts on Windows
to ensure `cargo fmt -- --check` and git diffs remain clean.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

DEFAULT_EXTENSIONS = {
    ".rs",
    ".toml",
    ".md",
    ".json",
    ".yml",
    ".yaml",
    ".py",
    ".sh",
    ".cmd",
    ".ps1",
}

IGNORED_DIRS = {
    ".git",
    "target",
    "data",  # binary/checksum-verified upstream artifacts
    ".claude",
    ".headroom",
    "codeql-db",
    "__pycache__",
    ".venv",
}


def normalize_directory(root: Path, extensions: set[str], check_only: bool) -> int:
    modified_count = 0
    for root_dir, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d not in IGNORED_DIRS]
        for f in files:
            path = Path(root_dir) / f
            if path.suffix.lower() in extensions or path.name in {"Justfile", ".gitignore", ".gitattributes"}:
                try:
                    data = path.read_bytes()
                    if b"\r\n" in data and b"\x00" not in data[:1024]:
                        modified_count += 1
                        if check_only:
                            print(f"CRLF line endings: {path}")
                        else:
                            path.write_bytes(data.replace(b"\r\n", b"\n"))
                            print(f"Normalized: {path}")
                except Exception as exc:
                    print(f"Skipping {path}: {exc}", file=sys.stderr)
    return modified_count


def main() -> int:
    parser = argparse.ArgumentParser(description="Normalize line endings to LF")
    parser.add_argument(
        "root",
        nargs="?",
        default=".",
        help="Root directory to normalize (default: repo root)",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check for CRLF without modifying files",
    )
    args = parser.parse_args()

    repo_root = Path(args.root).resolve()
    count = normalize_directory(repo_root, DEFAULT_EXTENSIONS, args.check)

    if args.check:
        if count > 0:
            print(f"Found {count} file(s) with CRLF line endings.")
            return 1
        print("All source files have LF line endings.")
        return 0

    print(f"Done. Normalized {count} file(s) to LF.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
