# Commit, Push, Tag

Complete release preparation. Do not stop at status or summary.

## Main agent flow

1. Inspect `TODO.md`, `git status`, current diff, `CHANGELOG.md`, version files, docs, schema, release workflow, installers, and source-archive inputs.
2. Finish all incomplete implementation work in current scope.
3. Update `TODO.md` for every completed behavior slice.
4. Update `CHANGELOG.md`, README/docs, SPEC, schema, or ADR only when public behavior or contract changed.
5. Remove generated files, temporary files, debug output, stale artifacts, and unrelated untracked files. Preserve user-owned work; report anything ambiguous instead of deleting it.
6. Run repository checks before staging:
   - `just check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
7. Inspect final diff and status. Do not continue until no known release blocker remains.
8. Stage only intended files.
9. Run `git commit`. Native `.git/hooks/pre-commit` runs `just check`; stop if it fails.
10. After commit succeeds, create version tag:
    ```text
    git tag vX.Y.Z
    ```
    Release gate catches tag command after pre-commit has already passed.
11. Push branch and tag:
    ```text
    git push origin HEAD
    git push origin refs/tags/vX.Y.Z
    ```

## Release gate handoff

Release hook waits 40 seconds only after deterministic preflight. Wait means handoff grace window, not test time. Hook sends preflight payload to `release-verifier`.

Verifier receives:

- current repository, HEAD, tag, and command;
- staged, unstaged, untracked, and ignored files;
- required-file and pre-commit status;
- command/check output available from preflight.

Verifier then checks TODO, unfinished work, documentation drift, stale schema/version/lock data, release workflow, installers, source archive, assets, checksums, and test results.

## Verifier correction loop

- Finding belongs in main agent scope: return detailed actionable task to main agent. Main agent fixes it and resumes this command.
- Safe release metadata/doc issue: verifier may fix it, then report exact change.
- Every finding includes category, severity, `path:line`, observed state, blocking reason, and required action.
- Rerun full verification after correction.
- Maximum two correction rounds.
- After round two, unresolved findings become `FAIL_FINAL`; main agent stops and reports all findings to user in one response.
- Only verifier `PASS` allows release continuation.

## Required result

On success, report commit, tag, branch push, tag push, checks, TODO synchronization, and verifier PASS.

On failure, report every finding. Never say only “release blocked.” Include exact path/line, reason, action, round, and whether main agent or verifier owns fix.
