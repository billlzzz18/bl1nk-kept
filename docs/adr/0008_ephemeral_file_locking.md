# ADR-0008: Ephemeral File Locking for Multi-Agent Coordination

## Status

Proposed

## Context

Multiple agents (subagents, MCP clients) may edit the same files concurrently. Without coordination, agents can overwrite each other's changes silently.

Current approach: serial execution (one agent at a time via SDD). This works but doesn't scale to parallel agent workflows.

Git worktrees provide full isolation but cost ~178MB per worktree for this repo — impractical for Rust projects with large working trees.

## Decision

Implement ephemeral, in-memory file-level locking via MCP tools.

### Architecture

```
┌─────────────┐     acquire_lock      ┌──────────────┐
│  Agent A    │ ──────────────────→   │  LockStore   │
│  (editor)   │ ←──────────────────   │  (in-memory) │
│             │   granted / denied    │  HashMap     │
└─────────────┘                       │  <path,info> │
                                      └──────────────┘
┌─────────────┐     acquire_lock           │
│  Agent B    │ ──────────────────→        │
│  (reviewer) │ ←──────────────────        │
│             │   denied (locked)          │
└─────────────┘                           │
                                          ▼
                                   TTL auto-release
                                   (5 min default)
```

### Lock Store

```rust
pub struct LockStore {
    locks: HashMap<PathBuf, LockInfo>,
}

pub struct LockInfo {
    pub agent_id: String,
    pub acquired_at: Instant,
    pub ttl: Duration,
}

impl LockStore {
    /// Try to acquire a lock. Returns true if granted.
    pub fn try_acquire(&mut self, path: PathBuf, agent_id: &str, ttl: Duration) -> bool {
        self.cleanup_expired();

        if let Some(existing) = self.locks.get(&path) {
            if existing.agent_id != agent_id {
                return false; // locked by another agent
            }
            // Same agent re-acquiring — refresh TTL
            self.locks.insert(path, LockInfo {
                agent_id: agent_id.to_string(),
                acquired_at: Instant::now(),
                ttl,
            });
            return true;
        }

        self.locks.insert(path, LockInfo {
            agent_id: agent_id.to_string(),
            acquired_at: Instant::now(),
            ttl,
        });
        true
    }

    /// Release a lock. Returns true if it was held.
    pub fn release(&mut self, path: &Path, agent_id: &str) -> bool {
        if let Some(existing) = self.locks.get(path) {
            if existing.agent_id == agent_id {
                self.locks.remove(path);
                return true;
            }
        }
        false
    }

    /// Check if a file is locked by another agent.
    pub fn is_locked(&self, path: &Path) -> Option<&LockInfo> {
        self.locks.get(path)
    }

    /// Remove all expired locks.
    fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.locks.retain(|_, info| now.duration_since(info.acquired_at) < info.ttl);
    }
}
```

### MCP Tools

| Tool | Parameters | Returns | Behavior |
|------|-----------|---------|----------|
| `acquire_lock` | `path`, `agent_id`, `ttl_secs?` (default 300) | `{ granted: bool, holder?: string }` | Try to acquire; if locked by another agent, return `granted: false` + current holder |
| `release_lock` | `path`, `agent_id` | `{ released: bool }` | Release lock held by this agent |
| `is_locked` | `path` | `{ locked: bool, holder?, expires_at? }` | Check lock status without acquiring |

### Agent Protocol

1. **Before edit:** `acquire_lock(path, agent_id)`
   - If `granted: true` → proceed with edit
   - If `granted: false` → skip this file, log conflict, move on
2. **After edit:** `release_lock(path, agent_id)`
3. **On crash/timeout:** TTL auto-releases (no manual cleanup needed)

### Contention Strategy: Skip + Report

When a lock is denied:
- Agent does NOT wait (no blocking)
- Agent logs the conflict: `"File X locked by agent Y — skipping edit"`
- Agent continues with other work
- If the file is critical, agent can retry after a short delay (max 3 retries)

This avoids deadlocks and keeps all agents productive.

## Consequences

### Positive
- No disk overhead (in-memory only)
- No deadlocks (TTL auto-release)
- Simple implementation (~50 lines)
- MCP-native — any agent can use it
- Session-scoped — locks vanish when server restarts

### Negative
- Single MCP instance only (locks don't share across processes)
- No cross-session persistence (by design — ephemeral)
- TTL is a blunt instrument (no graceful handoff)
- Agent must check `is_locked` before reading (optional, for read-before-write)

### Mitigations
- Multiple instances: each has own lock store — acceptable because each instance serves one agent pool
- TTL too short: increase default (5 min → 10 min for long edits)
- Read conflicts: optional `is_locked` check before read (not required for most workflows)

## Alternatives Considered

| Alternative | Rejected Because |
|-------------|-----------------|
| Git worktrees | ~178MB per worktree, impractical for Rust repos |
| Filesystem locks (flock) | Platform-specific, doesn't work across processes cleanly |
| SQLite-based locks | Overkill for ephemeral session-scoped locks |
| Git index locks | Too coarse (whole repo), blocks unrelated operations |
| Serial execution | Works but doesn't scale to parallel agents |

## Implementation Plan

1. `LockStore` struct in `kept-core/src/lock.rs` (or `kept-mcp/src/mcp/tools/lock.rs`)
2. MCP tools: `acquire_lock`, `release_lock`, `is_locked`
3. Agent prompt template: include lock protocol
4. No persistent storage — session-scoped only
