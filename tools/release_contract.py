import argparse
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


_TAG_RE = re.compile(r"^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


class ReleaseContractError(ValueError):
    pass


@dataclass(frozen=True, order=True)
class SemVer:
    major: int
    minor: int
    patch: int

    @classmethod
    def from_tag(cls, tag: str) -> "SemVer":
        match = _TAG_RE.fullmatch(tag)
        if match is None:
            raise ReleaseContractError(f"release tag must match vX.Y.Z: {tag}")
        return cls(*(int(part) for part in match.groups()))

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"


def workspace_version(root: Path) -> str:
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    return manifest["workspace"]["package"]["version"]


def release_versions(tags: list[str]) -> list[SemVer]:
    versions: list[SemVer] = []
    for tag in tags:
        tag = tag.strip()
        if not tag:
            continue
        if _TAG_RE.fullmatch(tag):
            versions.append(SemVer.from_tag(tag))
    return versions


def validate_release_tag(root: Path, tag: str, existing_tags: list[str]) -> str:
    candidate = SemVer.from_tag(tag)
    manifest_version = workspace_version(root)
    if str(candidate) != manifest_version:
        raise ReleaseContractError(
            f"tag {tag} does not match workspace version {manifest_version}"
        )

    previous_versions = release_versions(existing_tags)
    if previous_versions and candidate <= max(previous_versions):
        raise ReleaseContractError(
            f"release version {candidate} is not greater than existing maximum {max(previous_versions)}"
        )
    return str(candidate)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="validate a monotonic vX.Y.Z release tag against the workspace manifest"
    )
    parser.add_argument("--tag", required=True)
    parser.add_argument("--existing-tags-file", type=Path, required=True)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[1]
    existing_tags = args.existing_tags_file.read_text(encoding="utf-8").splitlines()
    try:
        version = validate_release_tag(root, args.tag, existing_tags)
    except ReleaseContractError as error:
        print(f"release-contract=failed error={error}")
        return 1
    print(f"release-contract=passed tag={args.tag} version={version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
