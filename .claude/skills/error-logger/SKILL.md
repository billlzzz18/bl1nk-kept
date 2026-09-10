---
name: error-logger
description: Use when agent makes a mistake, takes wrong approach, misunderstands a request, or user issues a correction. Enforces mandatory error logging to .learnings/ERRORS.md.
triggers:
  - mistake
  - wrong approach
  - misunderstood
  - correction
  - error
  - bug
  - fix
role: enforcer
scope: workflow
output-format: log
---

# Error Logger Enforcer

Bloop: Claude never logs its own mistakes. This skill fixes that.

## Tool Selection Rules

| Task | Use | When |
|------|-----|------|
| Log error to file | **Write/Edit** | Append to .learnings/ERRORS.md |
| Find existing errors | **FFF grep** | Search ERRORS.md for similar issues |
| Check error patterns | **Read** | Review recent errors for patterns |
| Update memory | **Write** | Add to persistent memory files |

## When to Log

**Every time** any of these happen:
- User corrects your approach
- You chose wrong approach
- You misunderstood a request
- You skipped verification steps
- You broke something
- You asked unnecessary questions
- You explained instead of acting
- You over-engineered a simple fix

## Log Format

Append to `.learnings/ERRORS.md`:

```markdown
## [ERR-YYYYMMDD-XXX] <short title>

**Logged at**: <ISO timestamp>
**Severity**: low/medium/high/critical
**Status**: active
**Scope**: <scope tags>

### Summary

<1-2 sentences: what happened, what should have happened>

### Error

<exact error text or description of wrong behavior>

### Context

- Request: <what user asked>
- Action: <what you did wrong>
- Root cause: <root cause>

### Suggested Fix

<how to prevent this in the future>

### Metadata
- Reproducible: yes/no
- Files: <list>
- Session: <session ID if available>
```

## Workflow

After any correction:
1. **STOP** — acknowledge the mistake
2. **LOG** — append entry to `.learnings/ERRORS.md`
3. **FIX** — correct the actual problem
4. **PREVENT** — update relevant skill/CLAUDE.md if pattern repeats

## Hard Rules

- **NEVER** skip logging after a correction
- **NEVER** log without a suggested fix
- **NEVER** batch log entries — log immediately when mistake happens
- **ALWAYS** include the exact user correction quote
- **ALWAYS** update severity if same error repeats

## ID Format

`ERR-YYYYMMDD-XXX` where:
- YYYYMMDD = date
- XXX = sequence number (001, 002, ...)

Check last entry in file, increment by 1.

## Output

After logging:
```
📝 Error logged: [ERR-YYYYMMDD-XXX] <title>
- Location: .learnings/ERRORS.md
- Severity: <level>
- Prevention: <what will change>
```
