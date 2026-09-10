---
name: context-budget
description: Use when context window is large or growing. Enforces context compression and pruning. Invoke when reading multiple files or large outputs.
triggers:
  - context
  - too many files
  - large output
  - compress
  - SQZ
  - context budget
role: enforcer
scope: workflow
output-format: action
---

# Context Budget Enforcer

Prevents context overflow by enforcing compression and pruning rules. Works with SQZ MCP.

## Tool Selection Rules

| Task | Use | When |
|------|-----|------|
| Read large file | **SQZ read** | File > 2KB |
| Compress output | **SQZ compress** | Tool output > 500 lines |
| Check cache | **SQZ cache list** | See what's cached |
| Quick file check | **Read** | File < 1KB, need exact content |
| Search file contents | **FFF grep** | Find specific patterns |

## Rules

### Before Reading Files
```
1. CHECK file size — if > 2KB, use SQZ read instead of raw Read
2. CHECK if content already in SQZ cache — use cached version
3. READ only what's needed — skip irrelevant sections
```

### After Large Tool Output
```
1. IF output > 500 lines → compress with SQZ before reasoning
2. IF multiple files read → batch compress together
3. EXTRACT only relevant facts — discard raw output after extraction
```

### Before Sending to User
```
1. KEEP responses under 200 lines
2. USE tables/lists over prose
3. REFERENCE files instead of dumping content
```

## SQZ Integration

```bash
# Read with compression
sqz read <file>

# Compress large output
echo "<output>" | sqz compress

# Check cache
sqz cache list
```

## Hard Rules

- **NEVER** read > 5 files without SQZ compression
- **NEVER** keep raw tool output in context after extracting facts
- **NEVER** send > 500 lines to user in one message
- **ALWAYS** prefer cached content over re-reading
- **ALWAYS** use SQZ for files > 2KB

## Warning Signs

If you notice:
- Context usage > 80% → compress everything immediately
- Re-reading same file → use SQZ cache
- Large grep/output → compress before reasoning
- Multiple file reads → batch and compress
