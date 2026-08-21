import re
import sys
from pathlib import Path


LINK_PATTERN = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")


def find_broken_local_links(root: Path) -> list[tuple[Path, str]]:
    broken: list[tuple[Path, str]] = []
    for markdown_path in sorted(root.rglob("*.md")):
        if any(part in {"target", ".learnings"} for part in markdown_path.parts):
            continue
        text = markdown_path.read_text(encoding="utf-8")
        for match in LINK_PATTERN.finditer(text):
            target = match.group(1).strip().strip("<>")
            if not target or target.startswith(("#", "http://", "https://", "mailto:")):
                continue
            local_path = target.split("#", maxsplit=1)[0]
            if local_path and not (markdown_path.parent / local_path).exists():
                broken.append((markdown_path, target))
    return broken


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    broken = find_broken_local_links(root)
    if not broken:
        print("markdown-links=passed")
        return 0
    for source, target in broken:
        print(f"broken local Markdown link: {source.relative_to(root)} -> {target}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
