#!/usr/bin/env python3
"""Produce a read-only repository operating-layer inventory as JSON."""

import argparse
import json
from pathlib import Path


ROOT_FILES = [
    "README.md",
    "README.th.md",
    "SPEC.md",
    "TODO.md",
    "CHANGELOG.md",
    "AGENTS.md",
    "Justfile",
    "Cargo.toml",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "clippy.toml",
    "nextest.toml",
    "deny.toml",
]
DIRECTORIES = [
    "schema",
    "research",
    "benchmarks",
    "data",
    "tests",
    "tools",
    ".learnings",
    ".github/workflows",
    "docs",
    "scripts",
    "presentation",
    "target",
    "dist",
]


def markdown_files(root: Path) -> list[str]:
    ignored = {".git", "target", "dist", "__pycache__"}
    return sorted(
        str(path.relative_to(root))
        for path in root.rglob("*.md")
        if not any(part in ignored for part in path.relative_to(root).parts)
    )


def inventory(root: Path) -> dict[str, object]:
    root = root.resolve()
    return {
        "root": str(root),
        "rootFiles": {name: (root / name).is_file() for name in ROOT_FILES},
        "directories": {name: (root / name).is_dir() for name in DIRECTORIES},
        "markdownFiles": markdown_files(root),
        "legacyCandidates": [
            path
            for path in markdown_files(root)
            if path.startswith("docs/") or "report" in path.lower() or "snapshot" in path.lower()
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path, help="repository root to inspect")
    args = parser.parse_args()
    print(json.dumps(inventory(args.root), ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
