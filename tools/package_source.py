import tarfile
import tomllib
import zipfile
from pathlib import Path


EXCLUDED_DIRECTORIES = {".git", "target", "__pycache__", "presentation", "dist"}
EXCLUDED_SUFFIXES = {".pyc"}


def should_include(root: Path, path: Path) -> bool:
    relative = path.relative_to(root)
    return not any(part in EXCLUDED_DIRECTORIES for part in relative.parts) and path.suffix not in EXCLUDED_SUFFIXES


def included_files(root: Path) -> list[Path]:
    return [
        path
        for path in sorted(root.rglob("*"))
        if path.is_file() and should_include(root, path)
    ]


def create_source_archive(root: Path, archive: Path) -> None:
    archive.parent.mkdir(parents=True, exist_ok=True)
    prefix = root.name
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as package:
        for path in included_files(root):
            package.write(path, Path(prefix) / path.relative_to(root))


def create_source_tarball(root: Path, archive: Path) -> None:
    archive.parent.mkdir(parents=True, exist_ok=True)
    prefix = root.name
    with tarfile.open(archive, "w:gz") as package:
        for path in included_files(root):
            package.add(path, arcname=Path(prefix) / path.relative_to(root), recursive=False)


def package_name(root: Path) -> str:
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    version = manifest["workspace"]["package"]["version"]
    return f"bl1nk-kept-{version}-source.zip"


def tarball_name(root: Path) -> str:
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    version = manifest["workspace"]["package"]["version"]
    return f"bl1nk-kept-{version}-source.tar.gz"


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    output_dir = root / "dist"
    archive = output_dir / package_name(root)
    tarball = output_dir / tarball_name(root)
    create_source_archive(root, archive)
    create_source_tarball(root, tarball)
    print(f"source-package={archive}")
    print(f"source-tarball={tarball}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
