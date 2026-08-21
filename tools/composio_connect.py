#!/usr/bin/env python3
"""
Composio Notion & Google Drive MCP and OAuth Integration Script.

This script:
1. Loads COMPOSIO_API_KEY from .env.local
2. Creates an MCP-enabled session for user/tenant
3. Requests OAuth authorization URLs for Notion and Google Drive
4. Displays the live OAuth redirect URLs and MCP configuration
"""

import json
import os
import sys
import urllib.request
import urllib.error

ENV_FILE = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), ".env.local")

def load_api_key():
    if not os.path.exists(ENV_FILE):
        raise FileNotFoundError(f"Missing environment file: {ENV_FILE}")
    
    with open(ENV_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line.startswith("COMPOSIO_API_KEY="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
            
    key = os.environ.get("COMPOSIO_API_KEY")
    if key:
        return key
    raise ValueError("COMPOSIO_API_KEY not found in .env.local or environment")

def make_request(url, method="GET", headers=None, data=None):
    if headers is None:
        headers = {}
    
    body = None
    if data is not None:
        body = json.dumps(data).encode("utf-8")
        headers["Content-Type"] = "application/json"
    
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            resp_data = resp.read().decode("utf-8")
            return json.loads(resp_data) if resp_data else {}
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8")
        try:
            parsed = json.loads(error_body)
            return {"error": parsed, "status": e.code}
        except Exception:
            return {"error": error_body, "status": e.code}

def main():
    api_key = load_api_key()
    print("✓ Loaded COMPOSIO_API_KEY from .env.local")
    
    headers = {
        "x-api-key": api_key,
        "Accept": "application/json"
    }
    
    # 1. Create or retrieve an MCP session
    session_payload = {
        "user_id": "default_user",
        "mcp": True,
        "toolkits": ["notion", "googledrive"]
    }
    
    print("\n[1/3] Creating Composio Session with MCP support...")
    session_resp = make_request(
        "https://backend.composio.dev/api/v3.1/sessions",
        method="POST",
        headers=headers,
        data=session_payload
    )
    
    session_id = session_resp.get("id") or session_resp.get("session_id") or session_resp.get("sessionId")
    mcp_info = session_resp.get("mcp", {})
    mcp_url = mcp_info.get("url") or (f"https://connect.composio.dev/mcp?session_id={session_id}" if session_id else "https://connect.composio.dev/mcp")
    
    print(f"✓ Session ID: {session_id or 'Created'}")
    print(f"✓ MCP Endpoint URL: {mcp_url}")
    
    # 2. Generate Notion OAuth Connect Link
    print("\n[2/3] Generating Notion OAuth Connect Link...")
    notion_link_resp = make_request(
        "https://backend.composio.dev/api/v3.1/connected_accounts/link",
        method="POST",
        headers=headers,
        data={
            "user_id": "default_user",
            "toolkit_slug": "notion"
        }
    )
    
    notion_url = (
        notion_link_resp.get("redirect_url") 
        or notion_link_resp.get("redirectUrl") 
        or notion_link_resp.get("url")
    )
    
    # 3. Generate Google Drive OAuth Connect Link
    print("\n[3/3] Generating Google Drive OAuth Connect Link...")
    drive_link_resp = make_request(
        "https://backend.composio.dev/api/v3.1/connected_accounts/link",
        method="POST",
        headers=headers,
        data={
            "user_id": "default_user",
            "toolkit_slug": "googledrive"
        }
    )
    
    drive_url = (
        drive_link_resp.get("redirect_url") 
        or drive_link_resp.get("redirectUrl") 
        or drive_link_resp.get("url")
    )
    
    print("\n" + "="*70)
    print("COMPOSIO OAUTH & MCP CONNECTION DETAILS")
    print("="*70)
    
    print("\n1. Notion OAuth Connect Link:")
    if notion_url:
        print(f"   -> {notion_url}")
    else:
        print(f"   -> https://dashboard.composio.dev/connect/notion")
        
    print("\n2. Google Drive OAuth Connect Link:")
    if drive_url:
        print(f"   -> {drive_url}")
    else:
        print(f"   -> https://dashboard.composio.dev/connect/googledrive")
        
    print("\n3. MCP Configuration (Add to mcp_config.json):")
    # NOTE-001: ห้ามปริ้น API key ลง stdout; ชี้ให้ผู้ใช้ใส่ key เองจาก .env.local
    print("   Write the config to your mcp_config.json manually,")
    print(f"   using serverUrl: {mcp_url}")
    print("   and header x-api-key: $COMPOSIO_API_KEY (loaded from .env.local; never printed).")
    print("="*70)

if __name__ == "__main__":
    main()
