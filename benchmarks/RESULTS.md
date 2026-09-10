# Benchmark Results — 2026-09-10

## 1. MCP Tool Performance (rg baseline)

| Operation | Min (ms) | Avg (ms) | P95 (ms) |
|-----------|----------|----------|----------|
| File search (`rg --files`) | 19.5 | 21.2 | 23.5 |
| Grep `fn handle_` | 29.5 | 29.9 | 30.6 |
| Grep `struct.*State` | 26.0 | 27.7 | 30.3 |
| Grep `pub fn` | 26.9 | 30.2 | 34.6 |
| Grep `TODO` | 26.1 | 28.4 | 32.2 |
| Grep `unwrap()` | 28.6 | 29.5 | 30.8 |
| Multi-grep (3 patterns) | 25.7 | 27.7 | 29.0 |

**Key finding:** rg grep averages ~28ms across patterns. File search is fastest at ~21ms.

## 2. Accuracy (Pattern Matching)

| Test Case | Result |
|-----------|--------|
| Exact identifier (`Observation`) | ✅ 100% |
| Function prefix (`fn handle_`) | ✅ 100% |
| Struct declaration (`struct.*State`) | ✅ 100% |
| TODO markers | ✅ 100% |
| Nested braces (`println!`) | ✅ 100% |
| Multi-pattern (`fn|struct`) | ✅ 100% |
| Thai text (`ตรวจสอบ`) | ✅ 100% |
| Pub symbols (`pub (fn|struct)`) | ✅ 100% |

**Overall accuracy: 14/14 (100%)**

## 3. Read Speed

| Method | Total (ms) | Throughput (MB/s) |
|--------|------------|-------------------|
| 🏆 Direct read | 2.5 | 20.0 |
| cat/type | 19.5 | 2.5 |
| rg -c | 116.6 | 0.4 |

**Key finding:** Direct file read is 8x faster than cat, 50x faster than rg for single-file reads.

## 4. MCP Server

| Test | Result |
|------|--------|
| Server `--check` mode | ✅ PASS |
| Stdio communication | ⚠️ Requires MCP inspector |

**Note:** Full MCP protocol testing requires `@anthropic-ai/mcp-inspector` (not installed).

## Files Generated

- `benchmarks/data/mcp_tool_benchmark.json` — Raw performance data
- `benchmarks/data/accuracy_benchmark.json` — Accuracy test results
- `benchmarks/data/read_speed_benchmark.json` — Read speed data
- `benchmarks/data/mcp_server_test.json` — Server test results
