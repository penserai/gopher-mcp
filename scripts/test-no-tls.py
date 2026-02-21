#!/usr/bin/env python3
"""Test MCP endpoints without TLS.

Requires: server running with --no-tls (cargo run -- --no-tls)
"""

import json
import sys
import urllib.request

URL = "http://127.0.0.1:8443/mcp"
PASS = 0
FAIL = 0


def post_json(body: dict) -> dict:
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        URL, data=data, headers={"Content-Type": "application/json"}, method="POST"
    )
    resp = urllib.request.urlopen(req)
    return json.loads(resp.read())


def post_status(body: dict) -> int:
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        URL, data=data, headers={"Content-Type": "application/json"}, method="POST"
    )
    try:
        resp = urllib.request.urlopen(req)
        return resp.status
    except urllib.error.HTTPError as e:
        return e.code


def check(desc: str, expected, actual):
    global PASS, FAIL
    if isinstance(expected, str):
        ok = expected in str(actual)
    elif callable(expected):
        ok = expected(actual)
    else:
        ok = expected == actual
    if ok:
        print(f"  PASS: {desc}")
        PASS += 1
    else:
        print(f"  FAIL: {desc}")
        print(f"        got: {str(actual)[:200]}")
        FAIL += 1


# --- Tests ---

print("=== MCP Protocol (no TLS) ===\n")

print("--- initialize ---")
r = post_json({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
check("returns protocolVersion", "2024-11-05", r["result"]["protocolVersion"])

print("\n--- notifications/initialized ---")
status = post_status({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
check("returns HTTP 204", 204, status)

print("\n--- tools/list ---")
r = post_json({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
tool_names = [t["name"] for t in r["result"]["tools"]]
check("lists 3 tools", 3, len(tool_names))

print("\n--- ping ---")
r = post_json({"jsonrpc": "2.0", "id": 3, "method": "ping", "params": {}})
check("returns result", lambda r: "result" in r, r)

print("\n--- gopher_browse local/ ---")
r = post_json({
    "jsonrpc": "2.0", "id": 4, "method": "tools/call",
    "params": {"name": "gopher_browse", "arguments": {"path": "local/"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("returns menu items", lambda _: len(items) == 3, None)

print("\n--- gopher_fetch local/welcome ---")
r = post_json({
    "jsonrpc": "2.0", "id": 5, "method": "tools/call",
    "params": {"name": "gopher_fetch", "arguments": {"path": "local/welcome"}}
})
check("returns document", "served directly from the local store", r["result"]["content"][0]["text"])

print("\n--- gopher_search local/ query=Welcome ---")
r = post_json({
    "jsonrpc": "2.0", "id": 6, "method": "tools/call",
    "params": {"name": "gopher_search", "arguments": {"path": "local/", "query": "Welcome"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("finds Welcome item", lambda _: any("Welcome" in i["display"] for i in items), None)

print("\n--- error: unknown tool ---")
r = post_json({
    "jsonrpc": "2.0", "id": 7, "method": "tools/call",
    "params": {"name": "bad_tool", "arguments": {}}
})
check("returns isError in content", True, r["result"].get("isError"))

print(f"\n=== Results: {PASS} passed, {FAIL} failed ===")
sys.exit(1 if FAIL else 0)
