import re
import sys
from pathlib import Path

# NOTE-001: รูปแบบมาตรฐานของคอมเมนต์เหตุผลภายใน: // NOTE-001: หรือ // NOTE-PREFIX-001: ตามด้วยภาษาไทย
NOTE_PATTERN = re.compile(r"//\s*NOTE\b")
VALID_NOTE_PATTERN = re.compile(r"//\s*NOTE-(?:[A-Z0-9]+-)?[0-9]{3}:")
THAI_CHAR_PATTERN = re.compile(r"[฀-๿]")


def find_note_comment_errors(root: Path) -> list[tuple[Path, int, str]]:
    errors: list[tuple[Path, int, str]] = []
    crates_dir = root / "crate"
    if not crates_dir.exists():
        return errors

    for rs_file in sorted(crates_dir.glob("*/src/**/*.rs")):
        text = rs_file.read_text(encoding="utf-8")
        for line_num, line in enumerate(text.splitlines(), start=1):
            if not NOTE_PATTERN.search(line):
                continue

            stripped = line.strip()
            # ตรวจสอบรูปแบบ NOTE-xxx:
            if not VALID_NOTE_PATTERN.search(stripped):
                errors.append(
                    (
                        rs_file,
                        line_num,
                        f"NOTE comment does not match standard '// NOTE-xxx:': {stripped}",
                    )
                )
                continue

            # ตรวจสอบว่ามีภาษาไทยกำกับ
            if not THAI_CHAR_PATTERN.search(stripped):
                errors.append(
                    (
                        rs_file,
                        line_num,
                        f"NOTE comment rationale must be in Thai: {stripped}",
                    )
                )

    return errors


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    errors = find_note_comment_errors(root)
    if not errors:
        print("note-comments=passed")
        return 0
    for file_path, line_num, message in errors:
        print(f"{file_path.relative_to(root)}:{line_num}: {message}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
