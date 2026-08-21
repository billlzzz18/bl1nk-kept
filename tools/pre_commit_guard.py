#!/usr/bin/env python3
import json
import os
import subprocess
import sys


def main() -> None:
    # Read stdin payload from Antigravity agent hook
    raw_input = sys.stdin.read().strip()
    if not raw_input:
        print(json.dumps({"decision": "allow"}))
        return

    try:
        data = json.loads(raw_input)
    except Exception:
        print(json.dumps({"decision": "allow"}))
        return

    tool_call = data.get("toolCall", {})
    tool_name = tool_call.get("name", "")
    args = tool_call.get("args", {})
    command_line = args.get("CommandLine", "").strip()

    # Check if this tool execution is attempting a git commit
    if tool_name == "run_command" and ("git commit" in command_line):
        workspace_paths = data.get("workspacePaths", [])
        cwd = workspace_paths[0] if workspace_paths else os.getcwd()

        try:
            # Run the project verification suite using just
            result = subprocess.run(
                ["just", "check"],
                cwd=cwd,
                capture_output=True,
                text=True,
                shell=True if os.name == "nt" else False,
            )
            if result.returncode == 0:
                print(
                    json.dumps(
                        {
                          "decision": "allow",
                          "reason": "Pre-Commit Guard passed: 'just check' succeeded.",
                        }
                    )
                )
            else:
                error_summary = (result.stderr or result.stdout or "Test/Lint suite failed").strip()
                # Truncate if error summary is too long
                if len(error_summary) > 500:
                    error_summary = error_summary[-500:]
                print(
                    json.dumps(
                        {
                          "decision": "deny",
                          "reason": f"Pre-Commit Guard BLOCKED git commit: 'just check' failed.\nDetails:\n{error_summary}",
                        }
                    )
                )
        except Exception as e:
            print(
                json.dumps(
                    {
                      "decision": "deny",
                      "reason": f"Pre-Commit Guard encountered an error running 'just check': {e}",
                    }
                )
            )
        return

    # Default allow for any non-commit commands
    print(json.dumps({"decision": "allow"}))


if __name__ == "__main__":
    main()
