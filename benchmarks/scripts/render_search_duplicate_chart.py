#!/usr/bin/env python3
"""Render the checked-in Rust release benchmark JSONL as a compact evidence snapshot."""

import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt


def load_rows(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("{"):
            rows.append(json.loads(line))
    if not rows:
        raise ValueError(f"No JSON benchmark rows found in {path}")
    return rows


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("Usage: generate_benchmark_chart.py INPUT.jsonl OUTPUT.png")

    rows = load_rows(Path(sys.argv[1]))
    output = Path(sys.argv[2])
    output.parent.mkdir(parents=True, exist_ok=True)
    rows.sort(key=lambda row: row["objects"])
    scales = [row["objects"] for row in rows]

    plt.style.use("seaborn-v0_8-whitegrid")
    fig, axes = plt.subplots(1, 2, figsize=(13.5, 5.5), constrained_layout=True)
    fig.suptitle("bl1nk-kept Rust Release Benchmark", fontsize=18, fontweight="bold")

    latency_series = {
        "Exact keyword": "exact_search_average_us",
        "Full-text": "full_text_search_average_us",
        "Fuzzy candidate": "fuzzy_search_average_us",
    }
    colors = ["#246BFD", "#1B998B", "#D97706"]
    for (label, field), color in zip(latency_series.items(), colors):
        values = [row[field] for row in rows]
        axes[0].plot(scales, values, marker="o", linewidth=2.5, label=label, color=color)
        for scale, value in zip(scales, values):
            axes[0].annotate(f"{value:,.0f} µs", (scale, value), xytext=(0, 9),
                             textcoords="offset points", ha="center", fontsize=9, color=color)
    axes[0].set_xscale("log")
    axes[0].set_yscale("log")
    axes[0].set_title("Average Search Latency")
    axes[0].set_xlabel("Registry objects (log scale)")
    axes[0].set_ylabel("Microseconds (log scale)")
    axes[0].legend(frameon=True)

    total_pairs = [row["duplicate_total_possible_pairs"] for row in rows]
    compared_pairs = [row["duplicate_compared_pairs"] for row in rows]
    width = 0.32
    positions = list(range(len(scales)))
    axes[1].bar([value - width / 2 for value in positions], total_pairs, width,
                label="All possible pairs", color="#CBD5E1")
    axes[1].bar([value + width / 2 for value in positions], compared_pairs, width,
                label="Levenshtein comparisons", color="#246BFD")
    axes[1].set_yscale("log")
    axes[1].set_xticks(positions, [f"{value:,}" for value in scales])
    axes[1].set_title("Duplicate Candidate Reduction")
    axes[1].set_xlabel("Filesystem objects")
    axes[1].set_ylabel("Pair count (log scale)")
    axes[1].legend(frameon=True)
    for position, total, compared in zip(positions, total_pairs, compared_pairs):
        axes[1].annotate(f"{total:,}", (position - width / 2, total), xytext=(0, 6),
                         textcoords="offset points", ha="center", fontsize=8)
        axes[1].annotate(f"{compared}", (position + width / 2, compared), xytext=(0, 6),
                         textcoords="offset points", ha="center", fontsize=8, color="#246BFD")

    fig.savefig(output, dpi=180, bbox_inches="tight")


if __name__ == "__main__":
    main()
