---
name: mcp-check
description: Use to verify MCP server status, tools, and connectivity. Invoke when MCP tools fail or before starting MCP-dependent work.
triggers:
  - MCP
  - mcp server
  - tool not found
  - connection failed
  - server status
role: enforcer
scope: workflow
output-format: report
---

# MCP Check Enforcer

Verifies MCP servers are running and tools are available. Catches issues before they block work.

## Tool Selection Rules

| Task | Use | When |
|------|-----|------|
| Check MCP server status | **Bash** | Get-Process or ps aux |
| List available tools | **ListMcpResourcesTool** | See what tools are registered |
| Test tool call | **Any MCP tool** | Verify server responds |
| Debug MCP connection | **MCP Inspector** | Real-time connection monitoring |
| Check VSCode extension | **VSCode** | Open MCP Inspector panel |

## Check List

```
1. VERIFY server process — is the MCP server running?
2. CHECK tool availability — are expected tools registered?
3. TEST tool call — can we call a simple tool?
4. VERIFY permissions — do we have access to required tools?
5. CHECK error logs — any recent failures?
```

## Quick Checks

### Check server process
```powershell
# Windows
Get-Process | Where-Object {$_.ProcessName -like "*mcp*"}

# Or check specific server
Get-Process | Where-Object {$_.ProcessName -like "*bl1nk-kept*"}
```

### Check tool availability
```bash
# List available MCP tools
# Use ListMcpResourcesTool or check Claude Code settings
```

### Test simple tool call
```bash
# Try a simple read operation
# If fails, server may be down
```

## VSCode Integration

Install MCP Inspector extension for real-time debugging:
```
ext install wso2.vscode-mcp-inspector
```

Commands:
- `Open MCP Inspector` — view connections
- `Open MCP Inspector with URL` — connect to specific server

## Common Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Tool not found | Server not running | Restart server |
| Connection timeout | Server hung | Kill and restart |
| Permission denied | Tool not in allowed list | Check settings |
| Tool returns error | Server bug | Check server logs |
| Extension not visible | VSCode not installed | Install VSCode first |

## Output Format

```
🔍 MCP Check Results
- Server: <name> — <running/stopped>
- Tools: <count> registered
- Last test: <pass/fail>
- Errors: <none or list>
```

## Hard Rules

- **ALWAYS** check MCP status before MCP-dependent work
- **ALWAYS** report which server failed when tool call fails
- **NEVER** assume MCP is running without checking
- **NEVER** retry same tool 3+ times without checking server status
