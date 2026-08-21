import re
import sys
import tomllib
from pathlib import Path


def workspace_version(root: Path) -> str:
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    return manifest["workspace"]["package"]["version"]


def find_version_contract_errors(root: Path) -> list[str]:
    version = workspace_version(root)
    changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    spec = (root / "SPEC.md").read_text(encoding="utf-8")
    errors: list[str] = []
    if not re.search(rf"^## \[{re.escape(version)}\]", changelog, flags=re.MULTILINE):
        errors.append(f"CHANGELOG.md does not contain release heading {version}")
    if f"**Workspace package version:** `{version}`" not in spec:
        errors.append(f"SPEC.md does not declare workspace version {version}")
    return errors


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    errors = find_version_contract_errors(root)
    if not errors:
        print(f"version-contract=passed version={workspace_version(root)}")
        return 0
    for error in errors:
        print(error)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
