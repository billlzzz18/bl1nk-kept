---
name: error-logger
description: Use after every mistake, wrong approach, or misunderstood request. Enforces mandatory error logging to .learnings/ERRORS.md. Invoke after any correction from user.
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

**บันทึกเมื่อ**: <ISO timestamp>
**ลำดับความสำคัญ**: ต่ำ/ปานกลาง/สูง/วิกฤต
**สถานะ**: กำลังแก้ไข
**ขอบเขต**: <scope tags>

### Summary

<1-2 sentences: what happened, what should have happened>

### Error

<exact error text or description of wrong behavior>

### Context

- คำส่ัง: <what user asked>
- สิ่งที่ทำ: <what you did wrong>
- สาเหตุ: <root cause>

### Suggested Fix

<how to prevent this in the future>

### Metadata
- ทำซ้ำได้: ใช่/ไม่ใช่
- ไฟล์ที่เกี่ยวข้อง: <list>
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
