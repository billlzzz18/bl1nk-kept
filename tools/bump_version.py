import re
import sys
from datetime import date
from pathlib import Path


SEMVER_PATTERN = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")
WORKSPACE_VERSION_PATTERN = re.compile(
    r"(?ms)(\[workspace\.package\][^\[]*?^version\s*=\s*)\"[^\"]+\""
)
SPEC_VERSION_PATTERN = re.compile(r"\*\*Workspace package version:\*\* `[^`]+`")
UNRELEASED_PATTERN = re.compile(r"^## \[Unreleased\]\s*$", flags=re.MULTILINE)


def replace_once(text: str, pattern: re.Pattern[str], replacement: str, label: str) -> str:
    updated, count = pattern.subn(replacement, text, count=1)
    if count != 1:
        raise ValueError(f"cannot locate exactly one {label}")
    return updated


def update_version(root: Path, version: str) -> None:
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

    manifest_path.write_text(manifest, encoding="utf-8")
    spec_path.write_text(spec, encoding="utf-8")
    changelog_path.write_text(changelog, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: python3 tools/bump_version.py <semantic-version>")
        return 2
    root = Path(__file__).resolve().parents[1]
    update_version(root, sys.argv[1])
    print(f"version-updated={sys.argv[1]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
