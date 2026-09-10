---
name: anti-slop
description: Use when reviewing or improving content quality, preventing generic AI patterns, cleaning up existing content, or enforcing quality standards in writing, code, or design work. Triggers on first run in any new codebase, after AI-assisted coding sessions, or when user says "clean up", "review quality", "check slop", or "anti-slop". Also activates proactively whenever slop patterns are detected in current work.
triggers:
  - clean up
  - review quality
  - check slop
  - anti-slop
  - AI slop
  - generic patterns
  - over-engineered
  - verbose code
  - obvious comments
role: Quality enforcer — detect and report generic AI-generated patterns across text, code, and design. Report with confidence and impact. Let user decide.
scope: All content types (text, code, design) in the current project
output-format: |
  [ANTI-SLOP REPORT]
  🔴 definite: <pattern> at <location> — <savings> — <action>
  🟡 suspicious: <pattern> at <location> — <savings> — <recommendation>
  ℹ️ info: <observation>
---

# Anti-Slop

ACTIVE EVERY RESPONSE. Detect generic AI-generated patterns ("slop") across text, code, and design. Report what you find. Let user decide what to fix.

## Persistence

Default: **full**. Off only: "stop anti-slop" / "normal mode". Switch: `/anti-slop lite|full|ultra`.

Like ponytail — this runs in the background. When you see slop while working, flag it. Don't wait for user to ask.

## The Workflow

Every time, same workflow. Modes control depth, not flow:

```
1. SCAN   — pattern-match against known slop patterns
2. REPORT — show findings with confidence + impact
3. ACT    — depth depends on mode
```

### Step 1: SCAN

Run detection scripts or manual pattern matching:

**Text:**
```bash
python scripts/detect_slop.py <file> [--verbose]
```

**Rust code:**
```bash
python scripts/detect_code_slop.py <file.rs> [--verbose]
```

**Manual:** Read `references/text-patterns.md`, `references/code-patterns.md`, or `references/design-patterns.md` for pattern catalogs.

### Step 2: REPORT

Always report before acting. Format:

```
🔴 DEFINITE SLOP
  Pattern: <name> — <what's wrong>
  Location: <file:line>
  Savings: <lines/bytes if deleted>
  Confidence: high (matches known pattern exactly)
  Action: <what to do>

🟡 SUSPICIOUS
  Pattern: <name> — <why it looks like slop>
  Location: <file:line>
  Savings: <lines/bytes if refactored>
  Confidence: medium (might be justified by context)
  Recommendation: <review suggested, not auto-fix>

ℹ️ INFO
  <observation about codebase-wide pattern>
```

**Confidence levels:**
- **High** — Matches a known pattern exactly. Safe to auto-fix.
- **Medium** — Looks like slop but might be intentional. Recommend review.
- **Low** — Pattern matches but context unclear. Flag only.

### Step 3: ACT (mode-dependent)

| Mode | After report |
|------|-------------|
| **lite** | Report only. User decides. |
| **full** | Report → fix 1 definite slop with highest savings → re-scan to verify. Default. |
| **ultra** | Report → fix all definite slop → flag suspicious for user review → show metrics. |

## Modes

### lite — Report Only

Show findings. Don't touch anything. User picks what to fix.

Use when: first scan of a codebase, user wants to see the landscape, unsure about changes.

### full — Fix Worst offender (default)

Report → pick 1 file with most definite slop → fix → verify.

Use when: normal work session, want to improve quality incrementally.

### ultra — Fix All + Metrics

Report all → fix all definite slop → flag suspicious → show before/after metrics.

Use when: dedicated cleanup session, pre-release quality pass, user explicitly wants thorough cleanup.

## What Counts as Slop

### Text Slop

See `references/text-patterns.md` for full catalog.

**High-risk (always slop):**
- "delve into", "navigate the complexities", "in today's fast-paced world"
- "it's important to note that", meta-commentary about the document
- "leverage", "synergistic", "paradigm shift"

**Context-dependent (check first):**
- "furthermore", "moreover", "essentially" — may be OK in academic writing
- Hedging language — may be OK in legal/medical contexts

### Code Slop

See `references/code-patterns.md` for full catalog.

**Definite slop (high confidence):**
- Obvious comments: `// Create a user` above `user = User()`
- Redundant null-guards: `if (x && x.length && x.length > 0)`
- Filler JSDoc: `/** Gets the user. @param id The id. */`
- One-use intermediates: `const x = fetch(); return x;`

**Suspicious (medium confidence — review first):**
- Generic variable names: `data`, `result`, `item` — might be OK in small scope
- `unwrap()` in tests — intentional panic in test context
- Long match arms — might be necessary for exhaustive handling
- Trait with 1 impl — might be future-proofing, might be over-abstraction

### Design Slop

See `references/design-patterns.md` for full catalog.

**Definite slop:**
- Purple/pink/cyan gradient backgrounds
- "Empower Your Business" copy
- Glassmorphism on every element

**Suspicious:**
- Card-based layout — might serve content well
- Center-aligned text — might be intentional design choice

## Reporting Impact

When reporting, always include:
1. **What** — the slop pattern name
2. **Where** — file:line
3. **Savings** — lines/bytes if removed
4. **Confidence** — high/medium/low
5. **Action** — safe to delete / review / info only

Example:
```
🔴 DEFINITE SLOP
  Pattern: obvious_comment — restates what code does
  Location: src/scanner/fff.rs:80
  Savings: 1 line, 0 logic change
  Confidence: high (if-branch already documents intent)
  Action: delete comment

🟡 SUSPICIOUS
  Pattern: long_function — 91 lines (>80 threshold)
  Location: src/scanner/fff.rs:91-181
  Savings: ~30 lines if extracted into 2 sub-functions
  Confidence: medium (match arms may justify length)
  Recommendation: review — might be OK for state machine

ℹ️ INFO
  Codebase has 3 functions >80 lines. Consider extracting helpers.
```

## Hard Rules

- **Always report before fixing.** Never skip the report step.
- **Never auto-fix medium/low confidence.** Flag for user.
- **Max 1 file per fix in full mode.** Max 3 in ultra mode.
- **Some "slop" is acceptable.** Academic writing, legal docs, tests — context matters.
- **Never flag example/tutorial content** that intentionally demonstrates patterns.
- **Preserve meaning.** Automated cleanup must not change logic.

## Reference Files

- **[text-patterns.md](references/text-patterns.md)** — Natural language slop with detection rules and edge cases
- **[code-patterns.md](references/code-patterns.md)** — Programming slop with real-world cases and confidence levels
- **[design-patterns.md](references/design-patterns.md)** — Visual/UX slop patterns

## Scripts

| Script | Purpose | Mode |
|--------|---------|------|
| `detect_slop.py` | Text slop detection | all modes |
| `detect_code_slop.py` | Rust AST analysis | full/ultra |
| `clean_slop.py` | Automated text cleanup | full/ultra |
