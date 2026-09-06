$ErrorActionPreference = 'Stop'
$raw = [Console]::In.ReadToEnd()
$hookInput = $raw | ConvertFrom-Json
$command = [string]$hookInput.tool_input.command
$isRelease = $command -match '(?i)\bgit\s+(tag|push)\b' -and $command -match '(?i)(refs/tags|tag\s+v?\d+\.\d+\.\d+|--tags)'
if (-not $isRelease) {
    @{ continue = $true } | ConvertTo-Json -Compress
    exit 0
}

$repo = (git rev-parse --show-toplevel 2>$null).Trim()
$head = (git rev-parse HEAD 2>$null).Trim()
Start-Sleep -Seconds 40
$head = (git rev-parse HEAD 2>$null).Trim()
$tag = [regex]::Match($command, 'v?\d+\.\d+\.\d+').Value
$status = @(git status --short 2>&1)
$ignored = @(git status --short --ignored 2>&1 | Where-Object { $_ -match '^!!' })
$preCommitHook = Join-Path $repo '.git/hooks/pre-commit'
$preCommitPresent = Test-Path $preCommitHook
$required = @('TODO.md', 'CHANGELOG.md', 'Cargo.toml', 'Cargo.lock', '.github/workflows/release.yml', 'install-mcp.sh', 'install-mcp.ps1')
$missing = @($required | Where-Object { -not (Test-Path (Join-Path $repo $_)) })
$untracked = @($status | Where-Object { $_ -match '^\?\?' })
$staged = @($status | Where-Object { $_ -match '^[MADRC]' })
$payload = @{
    wait_reason = '40-second handoff grace window starts after deterministic preflight'
    preflight_complete = $true
    preflight_findings = @(
        if (-not $preCommitPresent) { 'missing .git/hooks/pre-commit' }
        if ($missing.Count -gt 0) { 'required release files missing' }
        if ($staged.Count -gt 0) { 'staged changes require verifier decision' }
        if ($untracked.Count -gt 0) { 'untracked files require verifier decision' }
    )
    repository = $repo
    command = $command
    tag = $tag
    head = $head
    working_tree = $status
    missing_required_files = $missing
    untracked_files = $untracked
    staged_changes = $staged
    ignored_files = $ignored
    pre_commit_hook = $preCommitHook
    pre_commit_present = $preCommitPresent
    pre_commit_contract = 'must run just check before commit'
    instruction = 'Release verifier must produce detailed PASS or FAIL. Report every incomplete task, unchecked TODO item, stale or missing documentation, staged/untracked junk, failed check, exact path:line, reason, and required action. Return result to main agent.'
} | ConvertTo-Json -Compress -Depth 4
@{
    continue = $true
    hookSpecificOutput = @{
        hookEventName = 'PreToolUse'
        additionalContext = "Release gate completed deterministic preflight: $payload"
    }
} | ConvertTo-Json -Compress
exit 0
