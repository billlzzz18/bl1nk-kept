import re
import sys
from datetime import date
from pathlib import Path


SEMVER_PATTERN = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")
WORKSPACE_VERSION_PATTERN = re.compile(
    r"(?ms)(\[workspace\.package\][^\[]*?^version\s*=\s*)\"[^\"]+\""
)
WORKSPACE_VERSION_VALUE_PATTERN = re.compile(
    r"(?ms)\[workspace\.package\][^\[]*?^version\s*=\s*\"([^\"]+)\""
)
SPEC_VERSION_PATTERN = re.compile(r"\*\*Workspace package version:\*\* `[^`]+`")
UNRELEASED_PATTERN = re.compile(r"^## \[Unreleased\]\s*$", flags=re.MULTILINE)


def replace_once(text: str, pattern: re.Pattern[str], replacement: str, label: str) -> str:
    updated, count = pattern.subn(replacement, text, count=1)
    if count != 1:
        raise ValueError(f"cannot locate exactly one {label}")
    return updated


def current_workspace_version(root: Path) -> str:
    manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = WORKSPACE_VERSION_VALUE_PATTERN.search(manifest)
    if match is None:
        raise ValueError("cannot locate workspace package version")
    return match.group(1)


def next_patch_version(root: Path) -> str:
    version = current_workspace_version(root)
    core = version.split("+", 1)[0].split("-", 1)[0]
    parts = core.split(".")
    if len(parts) != 3 or not all(part.isdigit() for part in parts):
        raise ValueError(f"cannot auto-increment non numeric release version: {version}")
    parts[2] = str(int(parts[2]) + 1)
    return ".".join(parts)


def update_version(root: Path, version: str | None = None) -> str:
    if not version:
        version = next_patch_version(root)
    if not SEMVER_PATTERN.fullmatch(version):
        raise ValueError(f"version is not semantic version text: {version}")

    manifest_path = root / "Cargo.toml"
    spec_path = root / "SPEC.md"
    changelog_path = root / "CHANGELOG.md"

    manifest = replace_once(
        manifest_path.read_text(encoding="utf-8"),
        WORKSPACE_VERSION_PATTERN,
        rf'\1"{version}"',
        "workspace package version",
    )
    spec = replace_once(
        spec_path.read_text(encoding="utf-8"),
        SPEC_VERSION_PATTERN,
        f"**Workspace package version:** `{version}`",
        "SPEC workspace version",
    )
    changelog = changelog_path.read_text(encoding="utf-8")
    release_heading = f"## [{version}] - {date.today().isoformat()}"
    if release_heading not in changelog:
        changelog = replace_once(
            changelog,
            UNRELEASED_PATTERN,
            f"## [Unreleased]\n\n{release_heading}",
            "Unreleased changelog heading",
        )

    manifest_path.write_text(manifest, encoding="utf-8", newline="\n")
    spec_path.write_text(spec, encoding="utf-8", newline="\n")
    changelog_path.write_text(changelog, encoding="utf-8", newline="\n")
    return version


def main() -> int:
    if len(sys.argv) > 2:
        print("usage: python3 tools/bump_version.py [<semantic-version>]")
        return 2
    argument = sys.argv[1] if len(sys.argv) == 2 else ""
    root = Path(__file__).resolve().parents[1]
    resolved = update_version(root, argument)
    print(f"version-updated={resolved}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
