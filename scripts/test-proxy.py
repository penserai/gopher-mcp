#!/usr/bin/env python3
"""Test Gopher proxy against live servers.

Requires: server running with --no-tls (cargo run -- --no-tls)
Note: requires internet access to reach gopher.floodgap.com:70
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
    resp = urllib.request.urlopen(req, timeout=20)
    return json.loads(resp.read())


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

print("=== Gopher Proxy (live servers) ===\n")

print("--- gopher_browse gopher.floodgap.com/ ---")
r = post_json({
    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
    "params": {"name": "gopher_browse", "arguments": {"path": "gopher.floodgap.com/"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("returns Floodgap menu", lambda _: any("Floodgap" in i["display"] for i in items), None)
check("has navigable items", lambda _: any(i["type"] == "1" for i in items), None)
info_items = [i for i in items if i["type"] == "i"]
check("info items have empty path", lambda _: all(i["path"] == "" for i in info_items), None)

print("\n--- gopher_fetch gopher.floodgap.com/gopher/proxy ---")
r = post_json({
    "jsonrpc": "2.0", "id": 2, "method": "tools/call",
    "params": {"name": "gopher_fetch", "arguments": {"path": "gopher.floodgap.com/gopher/proxy"}}
})
check("returns text document", "Gopherspace", r["result"]["content"][0]["text"])

print("\n--- gopher_search Veronica-2 query=weather ---")
r = post_json({
    "jsonrpc": "2.0", "id": 3, "method": "tools/call",
    "params": {"name": "gopher_search", "arguments": {"path": "gopher.floodgap.com/v2/vs", "query": "weather"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("returns search results", lambda _: len(items) > 0, None)
non_info = [i for i in items if i["type"] != "i"]
check("finds weather-related items", lambda _: any("weather" in i["display"].lower() for i in non_info), None)

print("\n--- cross-server: browse sdf.org/ ---")
r = post_json({
    "jsonrpc": "2.0", "id": 4, "method": "tools/call",
    "params": {"name": "gopher_browse", "arguments": {"path": "sdf.org/"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("returns SDF menu", lambda _: any("SDF" in i["display"] for i in items), None)

print(f"\n=== Results: {PASS} passed, {FAIL} failed ===")
sys.exit(1 if FAIL else 0)
