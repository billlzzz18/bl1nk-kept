#!/usr/bin/env python3
"""
Extract modern UI terms and bilingual pairs from Obsidian translations
into structured corpus evidence for bl1nk-kept.
"""

import json
import re
import sys
from pathlib import Path

def parse_ini_translations(path: Path) -> dict[str, dict[str, str]]:
    content = path.read_text(encoding="utf-8")
    entries = {}
    current_key = None
    current_entry = {}

    for line in content.splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            if current_key and current_entry:
                entries[current_key] = current_entry
            current_key = line[1:-1].strip()
            current_entry = {}
        elif "=" in line:
            k, _, v = line.partition("=")
            current_entry[k.strip()] = v.strip()

    if current_key and current_entry:
        entries[current_key] = current_entry

    return entries

def main():
    th_file = Path("data/corpus/raw/obsidian_th.txt")
    if not th_file.exists():
        print(f"Error: {th_file} not found", file=sys.stderr)
        sys.exit(1)

    th_entries = parse_ini_translations(th_file)
    output_records = []

    for key, data in th_entries.items():
        original = data.get("original", "").strip()
        translation = data.get("translation", "").strip()

        if not original or not translation:
            continue

        # Skip entries where translation is identical or a placeholder
        if original == translation and len(original) < 3:
            continue

        record = {
            "id": f"obsidian:{key}",
            "original_en": original,
            "translation_th": translation,
            "provenance": {
                "source": "https://github.com/obsidianmd/obsidian-translations",
                "file": "translations/th.txt",
                "key": key
            },
            "category": "ui_domain_term"
        }
        output_records.append(record)

    out_jsonl = Path("data/corpus/obsidian_terms.jsonl")
    out_jsonl.parent.mkdir(parents=True, exist_ok=True)
    with open(out_jsonl, "w", encoding="utf-8") as f:
        for r in output_records:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")

    print(f"Extracted {len(output_records)} terms to {out_jsonl}")

if __name__ == "__main__":
    main()
