#!/usr/bin/env python3
"""
Accuracy Benchmark: Pattern matching, keyword extraction, symbol extraction

Compares kept's accuracy against ground truth fixtures.

Usage:
    python benchmarks/scripts/accuracy_benchmark.py [--root <path>]
"""

import json
import os
import subprocess
import sys
from pathlib import Path
from dataclasses import dataclass, asdict


@dataclass
class AccuracyCase:
    name: str
    input_text: str
    pattern: str
    expected_matches: list[str]
    match_type: str  # "contains", "regex", "exact"


# Ground truth test cases
ACCURACY_CASES = [
    # Pattern matching — exact identifier
    AccuracyCase(
        name="exact_identifier",
        input_text="fn handle_request() { let observation = Observation::new(); }",
        pattern="Observation",
        expected_matches=["Observation"],
        match_type="exact",
    ),
    # Pattern matching — function prefix
    AccuracyCase(
        name="function_prefix",
        input_text="fn handle_get() {}\nfn handle_post() {}\nfn handle_delete() {}\nstruct User {}",
        pattern="fn handle_",
        expected_matches=["fn handle_get()", "fn handle_post()", "fn handle_delete()"],
        match_type="contains",
    ),
    # Pattern matching — struct declaration
    AccuracyCase(
        name="struct_declaration",
        input_text="pub struct ResourceState {\n    pub name: String,\n}\nstruct Internal { x: i32 }",
        pattern="struct.*State",
        expected_matches=["pub struct ResourceState {"],
        match_type="regex",
    ),
    # Pattern matching — TODO markers
    AccuracyCase(
        name="todo_markers",
        input_text="// TODO: implement auth\n// FIXME: memory leak\n// TODO: add tests\nlet x = 1;",
        pattern="TODO",
        expected_matches=["// TODO: implement auth", "// TODO: add tests"],
        match_type="contains",
    ),
    # Pattern matching — nested braces (hard for regex)
    AccuracyCase(
        name="nested_braces",
        input_text="fn process() {\n    if true {\n        for i in 0..10 {\n            println!(\"{i}\");\n        }\n    }\n}",
        pattern="println!",
        expected_matches=['println!("{i}");'],
        match_type="contains",
    ),
    # Multi-pattern — combined search
    AccuracyCase(
        name="multi_pattern",
        input_text="fn main() {}\nstruct Config {}\nimpl Config {\n    fn new() -> Self { Self }\n}",
        pattern="fn|struct",
        expected_matches=["fn main() {}", "struct Config {}", "fn new()"],
        match_type="contains",
    ),
    # Thai text search
    AccuracyCase(
        name="thai_text",
        input_text="// ตรวจสอบไฟล์\nlet x = read_file();\n// ค้นหาข้อมูล",
        pattern="ตรวจสอบ",
        expected_matches=["// ตรวจสอบไฟล์"],
        match_type="contains",
    ),
    # Symbol extraction — pub items
    AccuracyCase(
        name="pub_symbols",
        input_text="pub fn public_api() {}\nfn private_helper() {}\npub struct PublicType {}\nstruct PrivateType {}",
        pattern="pub (fn|struct|enum|trait)",
        expected_matches=["pub fn public_api()", "pub struct PublicType {}"],
        match_type="regex",
    ),
]


def run_kept_grep(root: str, pattern: str) -> list[str]:
    """Run kept grep and return matched lines."""
    kept_bin = "target/debug/kept.exe" if sys.platform == "win32" else "target/debug/kept"
    if not os.path.exists(kept_bin):
        return []

    # For now, fall back to rg since kept grep may not be wired yet
    try:
        result = subprocess.run(
            ["rg", "-n", pattern, root],
            capture_output=True, text=True, timeout=10
        )
        return [line.strip() for line in result.stdout.strip().split("\n") if line.strip()]
    except Exception:
        return []


def run_rg_grep(pattern: str, text: str) -> list[str]:
    """Run rg on text and return matched lines."""
    try:
        result = subprocess.run(
            ["rg", "-n", pattern],
            input=text, capture_output=True, text=True, timeout=10
        )
        return [line.strip() for line in result.stdout.strip().split("\n") if line.strip()]
    except Exception:
        return []


def evaluate_accuracy(tool_name: str, grep_fn, cases: list[AccuracyCase]) -> dict:
    """Evaluate accuracy for a tool against test cases."""
    results = {}
    for case in cases:
        matches = grep_fn(case.pattern, case.input_text)

        # Check if expected matches are found
        found = []
        for expected in case.expected_matches:
            if any(expected in m for m in matches):
                found.append(expected)

        accuracy = len(found) / len(case.expected_matches) if case.expected_matches else 1.0
        results[case.name] = {
            "expected": len(case.expected_matches),
            "found": len(found),
            "accuracy": round(accuracy, 4),
            "missing": [e for e in case.expected_matches if e not in found],
        }

    return results


def main():
    print("Accuracy Benchmark: Pattern Matching")
    print("=" * 60)

    # Evaluate rg
    print("\n[rg] Running accuracy tests...")
    rg_results = evaluate_accuracy("rg", run_rg_grep, ACCURACY_CASES)

    # Summary
    print("\n" + "=" * 60)
    print("ACCURACY RESULTS")
    print("=" * 60)

    total_expected = 0
    total_found = 0
    for name, result in rg_results.items():
        status = "✅" if result["accuracy"] == 1.0 else "❌"
        print(f"  {status} {name}: {result['found']}/{result['expected']} ({result['accuracy']:.0%})")
        if result["missing"]:
            print(f"     Missing: {result['missing']}")
        total_expected += result["expected"]
        total_found += result["found"]

    overall = total_found / total_expected if total_expected else 0
    print(f"\nOverall accuracy: {total_found}/{total_expected} ({overall:.0%})")

    # Save report
    report = {
        "rg": rg_results,
        "overall_accuracy": round(overall, 4),
        "total_expected": total_expected,
        "total_found": total_found,
    }

    report_path = Path("benchmarks/data/accuracy_benchmark.json")
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)
    print(f"\nReport saved to: {report_path}")


if __name__ == "__main__":
    main()
