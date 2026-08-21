[unix]
set shell := ["bash", "-cu"]

[windows]
set shell := ["powershell.exe", "-NoProfile", "-Command"]

python := if os() == "windows" { "python" } else { "python3" }

_default:
    @just --list

fmt:
    cargo fmt --all -- --check

test:
    cargo test --workspace

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

repo-contract:
    {{python}} -m unittest tests/test_repository_contract.py tests/test_repository_tools.py

cli-smoke:
    cargo build -q -p kept-cli
    {{python}} -m unittest tests/test_public_cli_smoke.py

schema:
    cargo run -q -p kept-core --example export_schema > schema/keyword-registry.schema.json

schema-check:
    {{python}} -c "import subprocess, sys; out = subprocess.check_output(['cargo', 'run', '-q', '-p', 'kept-core', '--example', 'export_schema'], text=True); target = open('schema/keyword-registry.schema.json', encoding='utf-8').read(); sys.exit(0 if out.strip() == target.strip() else 1)"

benchmark-chart:
    {{python}} benchmarks/scripts/render_search_duplicate_chart.py benchmarks/data/search_duplicate_release.jsonl benchmarks/charts/search_duplicate_release.png

links:
    {{python}} tools/check_markdown_links.py

version-check:
    {{python}} tools/check_version_contract.py

# Bump the workspace version. With no argument it increments the patch release.
version-bump new_version="":
    {{python}} tools/bump_version.py {{new_version}}

check: fmt test clippy repo-contract cli-smoke schema-check links version-check

package:
    {{python}} tools/package_source.py
