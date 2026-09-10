#!/usr/bin/env python3
"""
MCP Tool Benchmark: kept vs FFF vs ripgrep (rg)

Compares performance of kept MCP tools against FFF and ripgrep
for equivalent operations.

Usage:
    python benchmarks/scripts/mcp_tool_benchmark.py [--root <path>] [--repeat <N>]
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional


@dataclass
class BenchmarkResult:
    tool: str
    operation: str
    pattern: Optional[str]
    latency_ms: float
    matches: int
    throughput_mbps: Optional[float] = None
    error: Optional[str] = None


def run_command(cmd: list[str], timeout: int = 30) -> tuple[str, str, int]:
    """Run a command and return (stdout, stderr, returncode)."""
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=timeout
        )
        return result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired:
        return "", "TIMEOUT", -1
    except FileNotFoundError as e:
        return "", str(e), -2


def bench_rg_file_search(root: str, repeat: int = 5) -> list[BenchmarkResult]:
    """Benchmark ripgrep file listing."""
    results = []
    for _ in range(repeat):
        start = time.perf_counter()
        stdout, stderr, rc = run_command(["rg", "--files", root])
        elapsed = (time.perf_counter() - start) * 1000
        matches = len(stdout.strip().split("\n")) if stdout.strip() else 0
        results.append(BenchmarkResult(
            tool="rg", operation="file_search", pattern=None,
            latency_ms=elapsed, matches=matches,
            error=stderr if rc != 0 else None
        ))
    return results


def bench_rg_grep(root: str, pattern: str, repeat: int = 5) -> list[BenchmarkResult]:
    """Benchmark ripgrep content search."""
    results = []
    for _ in range(repeat):
        start = time.perf_counter()
        stdout, stderr, rc = run_command(["rg", "-c", pattern, root])
        elapsed = (time.perf_counter() - start) * 1000
        # rg -c output: "path:count" per line (multi-file) or just "count" (single file)
        matches = 0
        if stdout.strip():
            for line in stdout.strip().split("\n"):
                line = line.strip()
                if not line:
                    continue
                if ":" in line and not line.startswith(":"):
                    parts = line.rsplit(":", 1)
                    if len(parts) == 2 and parts[1].isdigit():
                        matches += int(parts[1])
                elif line.isdigit():
                    matches += int(line)
        results.append(BenchmarkResult(
            tool="rg", operation="grep", pattern=pattern,
            latency_ms=elapsed, matches=matches,
            error=stderr if rc != 0 and rc != 1 else None
        ))
    return results


def bench_rg_multi_grep(root: str, patterns: list[str], repeat: int = 5) -> list[BenchmarkResult]:
    """Benchmark ripgrep multi-pattern search."""
    results = []
    for _ in range(repeat):
        args = ["rg", "-c"] + [arg for p in patterns for arg in ["-e", p]] + [root]
        start = time.perf_counter()
        stdout, stderr, rc = run_command(args)
        elapsed = (time.perf_counter() - start) * 1000
        # rg -c output: "path:count" per line (multi-file) or just "count" (single file)
        matches = 0
        if stdout.strip():
            for line in stdout.strip().split("\n"):
                line = line.strip()
                if not line:
                    continue
                if ":" in line and not line.startswith(":"):
                    parts = line.rsplit(":", 1)
                    if len(parts) == 2 and parts[1].isdigit():
                        matches += int(parts[1])
                elif line.isdigit():
                    matches += int(line)
        results.append(BenchmarkResult(
            tool="rg", operation="multi_grep",
            pattern="|".join(patterns),
            latency_ms=elapsed, matches=matches,
            error=stderr if rc != 0 and rc != 1 else None
        ))
    return results


def bench_kept_cli(operation: str, args: list[str], repeat: int = 5) -> list[BenchmarkResult]:
    """Benchmark kept CLI operations."""
    results = []
    kept_bin = "target/debug/kept.exe" if sys.platform == "win32" else "target/debug/kept"
    if not os.path.exists(kept_bin):
        return [BenchmarkResult(
            tool="kept", operation=operation, pattern=None,
            latency_ms=0, matches=0, error=f"kept binary not found at {kept_bin}"
        )]

    for _ in range(repeat):
        cmd = [kept_bin, operation] + args
        start = time.perf_counter()
        stdout, stderr, rc = run_command(cmd)
        elapsed = (time.perf_counter() - start) * 1000
        # Try to parse JSON output for match count
        matches = 0
        try:
            data = json.loads(stdout)
            if "groups" in data:
                matches = len(data["groups"])
            elif "matches" in data:
                matches = len(data["matches"])
        except (json.JSONDecodeError, TypeError):
            pass
        results.append(BenchmarkResult(
            tool="kept", operation=operation,
            pattern=args[1] if len(args) > 1 else None,
            latency_ms=elapsed, matches=matches,
            error=stderr if rc != 0 else None
        ))
    return results


def compute_stats(results: list[BenchmarkResult]) -> dict:
    """Compute min/max/avg/p50/p95 for a set of results."""
    latencies = [r.latency_ms for r in results if r.error is None]
    if not latencies:
        return {"min": 0, "max": 0, "avg": 0, "p50": 0, "p95": 0, "n": 0}
    latencies.sort()
    n = len(latencies)
    return {
        "min": round(latencies[0], 2),
        "max": round(latencies[-1], 2),
        "avg": round(sum(latencies) / n, 2),
        "p50": round(latencies[n // 2], 2),
        "p95": round(latencies[int(n * 0.95)], 2),
        "n": n,
    }


def main():
    parser = argparse.ArgumentParser(description="MCP Tool Benchmark")
    parser.add_argument("--root", default=".", help="Root directory to benchmark against")
    parser.add_argument("--repeat", type=int, default=5, help="Number of repetitions")
    args = parser.parse_args()

    root = os.path.abspath(args.root)
    repeat = args.repeat

    print(f"Benchmark root: {root}")
    print(f"Repetitions: {repeat}")
    print("=" * 60)

    all_results = {}

    # 1. File search benchmark
    print("\n[1/4] File search (rg --files)")
    rg_results = bench_rg_file_search(root, repeat)
    all_results["rg_file_search"] = rg_results

    # 2. Grep benchmarks with different patterns
    patterns = [
        "fn handle_",
        "struct.*State",
        "pub fn",
        "TODO",
        "unwrap()",
    ]
    print("\n[2/4] Content search (grep)")
    for pattern in patterns:
        print(f"  Pattern: {pattern}")
        rg_results = bench_rg_grep(root, pattern, repeat)
        all_results[f"rg_grep_{pattern}"] = rg_results

    # 3. Multi-pattern grep
    print("\n[3/4] Multi-pattern search")
    multi_patterns = ["fn handle_", "struct.*State", "pub fn"]
    rg_results = bench_rg_multi_grep(root, multi_patterns, repeat)
    all_results["rg_multi_grep"] = rg_results

    # 4. Kept CLI benchmarks (if available)
    print("\n[4/4] Kept CLI benchmarks")
    kept_results = bench_kept_cli("scan", [root], repeat)
    all_results["kept_scan"] = kept_results

    # Generate report
    print("\n" + "=" * 60)
    print("BENCHMARK RESULTS")
    print("=" * 60)

    report = {}
    for key, results in all_results.items():
        stats = compute_stats(results)
        report[key] = stats
        errors = [r.error for r in results if r.error]
        error_str = f" [errors: {errors[0]}]" if errors else ""
        print(f"{key}: min={stats['min']}ms avg={stats['avg']}ms p95={stats['p95']}ms n={stats['n']}{error_str}")

    # Save report
    report_path = Path(root) / "benchmarks" / "data" / "mcp_tool_benchmark.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)
    print(f"\nReport saved to: {report_path}")


if __name__ == "__main__":
    main()
