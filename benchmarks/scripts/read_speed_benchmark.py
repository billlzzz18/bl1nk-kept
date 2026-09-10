#!/usr/bin/env python3
"""
Read Speed Benchmark: File reading latency

Compares reading speed for individual files across tools.

Usage:
    python benchmarks/scripts/read_speed_benchmark.py [--root <path>] [--repeat 5]
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from dataclasses import dataclass


@dataclass
class FileTarget:
    path: str
    size_bytes: int


def find_test_files(root: str, count: int = 10) -> list[FileTarget]:
    """Find a mix of small, medium, large files for benchmarking."""
    files = []
    for dirpath, dirnames, filenames in os.walk(root):
        # Skip hidden dirs, target, node_modules
        dirnames[:] = [d for d in dirnames if not d.startswith(".") and d not in ("target", "node_modules")]
        for f in filenames:
            if f.endswith((".rs", ".toml", ".md")):
                full = os.path.join(dirpath, f)
                try:
                    size = os.path.getsize(full)
                    if 100 < size < 100_000:  # 100B to 100KB
                        files.append(FileTarget(path=full, size_bytes=size))
                except OSError:
                    pass
    # Sort by size for stratified sampling
    files.sort(key=lambda x: x.size_bytes)
    if len(files) > count:
        # Pick evenly across size range
        step = len(files) // count
        files = [files[i * step] for i in range(count)]
    return files[:count]


def read_file_direct(path: str) -> bytes:
    """Direct file read (baseline)."""
    with open(path, "rb") as f:
        return f.read()


def read_file_rg(path: str) -> str:
    """Read file via rg (simulates MCP grep with context)."""
    result = subprocess.run(
        ["rg", "-c", ".", path],
        capture_output=True, text=True, timeout=10
    )
    return result.stdout


def read_file_cat(path: str) -> str:
    """Read file via cat (simulates MCP read)."""
    if sys.platform == "win32":
        result = subprocess.run(["type", path], capture_output=True, text=True, timeout=10)
    else:
        result = subprocess.run(["cat", path], capture_output=True, text=True, timeout=10)
    return result.stdout


def benchmark_tool(name: str, read_fn, files: list[FileTarget], repeat: int) -> dict:
    """Benchmark a read function across files."""
    results = []
    for target in files:
        times = []
        for _ in range(repeat):
            start = time.perf_counter()
            try:
                read_fn(target.path)
            except Exception:
                pass
            elapsed = time.perf_counter() - start
            times.append(elapsed)

        results.append({
            "path": os.path.basename(target.path),
            "size_bytes": target.size_bytes,
            "times_ms": [round(t * 1000, 3) for t in times],
            "avg_ms": round(sum(times) / len(times) * 1000, 3),
            "min_ms": round(min(times) * 1000, 3),
            "max_ms": round(max(times) * 1000, 3),
        })

    # Aggregate across all files
    all_avgs = [r["avg_ms"] for r in results]
    total_size = sum(t.size_bytes for t in files)
    total_time = sum(all_avgs)

    return {
        "files": results,
        "aggregate": {
            "total_files": len(files),
            "total_size_bytes": total_size,
            "total_avg_ms": round(total_time, 3),
            "throughput_mbps": round(total_size / (total_time / 1000) / (1024 * 1024), 2) if total_time > 0 else 0,
        },
    }


def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="Repository root")
    parser.add_argument("--repeat", type=int, default=5, help="Repetitions per file")
    parser.add_argument("--count", type=int, default=10, help="Number of test files")
    args = parser.parse_args()

    print("Read Speed Benchmark")
    print("=" * 60)

    files = find_test_files(args.root, args.count)
    if not files:
        print("No test files found!")
        return

    print(f"\nTest files: {len(files)}")
    for f in files:
        print(f"  {os.path.basename(f.path)} ({f.size_bytes:,} bytes)")
    print(f"Repeat: {args.repeat}x per file")

    tools = {
        "direct_read": read_file_direct,
        "cat/type": read_file_cat,
        "rg_count": read_file_rg,
    }

    all_results = {}
    for name, fn in tools.items():
        print(f"\n[{name}] Benchmarking...")
        result = benchmark_tool(name, fn, files, args.repeat)
        all_results[name] = result
        agg = result["aggregate"]
        print(f"  Total: {agg['total_avg_ms']:.1f}ms | Throughput: {agg['throughput_mbps']:.1f} MB/s")

    # Summary table
    print("\n" + "=" * 60)
    print("READ SPEED SUMMARY")
    print("=" * 60)
    print(f"{'Tool':<20} {'Total (ms)':<15} {'MB/s':<12} {'Winner':<8}")
    print("-" * 60)

    best_throughput = 0
    best_tool = ""
    for name, result in all_results.items():
        agg = result["aggregate"]
        if agg["throughput_mbps"] > best_throughput:
            best_throughput = agg["throughput_mbps"]
            best_tool = name

    for name, result in all_results.items():
        agg = result["aggregate"]
        marker = "🏆" if name == best_tool else "  "
        print(f"{marker} {name:<18} {agg['total_avg_ms']:<15.1f} {agg['throughput_mbps']:<12.1f}")

    # Save report
    report_path = Path("benchmarks/data/read_speed_benchmark.json")
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, "w") as f:
        json.dump(all_results, f, indent=2)
    print(f"\nReport saved to: {report_path}")


if __name__ == "__main__":
    main()
