#!/usr/bin/env python3
"""
MCP Server Test: Validates kept MCP server starts and responds.

Usage:
    python benchmarks/scripts/mcp_server_test.py [--server <path>]
"""

import json
import subprocess
import sys
import time
from pathlib import Path


def test_server_check(server_path: str) -> dict:
    """Test server with --check flag."""
    import os
    path = os.path.abspath(server_path)
    if not os.path.exists(path):
        return {"status": "SKIP", "reason": f"Not found: {path}"}
    try:
        result = subprocess.run(
            [path, "--check"],
            capture_output=True, text=True, timeout=5
        )
        return {
            "status": "PASS" if result.returncode == 0 else "FAIL",
            "output": result.stdout.strip(),
            "error": result.stderr.strip() if result.returncode != 0 else None,
        }
    except FileNotFoundError:
        return {"status": "SKIP", "reason": "Server binary not found"}
    except subprocess.TimeoutExpired:
        return {"status": "FAIL", "reason": "Server timed out"}


def test_server_stdio(server_path: str) -> dict:
    """Test server stdio communication with MCP protocol."""
    import os
    path = os.path.abspath(server_path)
    if not os.path.exists(path):
        return {"status": "SKIP", "reason": f"Not found: {path}"}

    try:
        proc = subprocess.Popen(
            [path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=False,
        )

        # Send initialize request
        request = json.dumps({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"},
            }
        })

        content = f"Content-Length: {len(request.encode())}\r\n\r\n{request}"
        proc.stdin.write(content.encode())
        proc.stdin.flush()

        # Read response with threading timeout (avoids select on Windows)
        import threading
        result = {"response": None}

        def read_response():
            try:
                result["response"] = proc.stdout.readline()
            except Exception:
                pass

        reader = threading.Thread(target=read_response)
        reader.daemon = True
        reader.start()
        reader.join(timeout=3)

        proc.terminate()
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            proc.kill()

        if result["response"]:
            return {"status": "PASS", "response_len": len(result["response"])}
        else:
            return {"status": "FAIL", "reason": "No response within 3s"}

    except Exception as e:
        return {"status": "ERROR", "error": str(e)}

    return {"status": "FAIL", "reason": "Unknown"}


def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", default="target/debug/bl1nk-kept-mcp.exe")
    args = parser.parse_args()

    print("MCP Server Test")
    print("=" * 60)

    results = {}

    # Test 1: --check mode
    print("\n[1] Server --check mode...")
    results["check"] = test_server_check(args.server)
    print(f"  {'✅' if results['check']['status'] == 'PASS' else '❌'} {results['check']}")

    # Test 2: Stdio communication
    print("\n[2] Stdio communication...")
    results["stdio"] = test_server_stdio(args.server)
    print(f"  {'✅' if results['stdio']['status'] == 'PASS' else '❌'} {results['stdio']}")

    # Summary
    print("\n" + "=" * 60)
    passed = sum(1 for r in results.values() if r.get("status") == "PASS")
    total = len(results)
    print(f"RESULTS: {passed}/{total} passed")

    # Save report
    report_path = Path("benchmarks/data/mcp_server_test.json")
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, "w") as f:
        json.dump(results, f, indent=2, default=str)
    print(f"Report saved to: {report_path}")


if __name__ == "__main__":
    main()
