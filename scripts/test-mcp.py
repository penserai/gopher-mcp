#!/usr/bin/env python3
"""Test MCP endpoints over mTLS.

Requires: server running with mTLS (cargo run)
"""

import json
import ssl
import sys
import urllib.request

URL = "https://127.0.0.1:8443/mcp"
PASS = 0
FAIL = 0


def make_ssl_context():
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    ctx.load_cert_chain("certs/client.crt", "certs/client.key")
    ctx.load_verify_locations("certs/ca.crt")
    return ctx


CTX = make_ssl_context()


def post(body: dict) -> urllib.request.Request:
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        URL, data=data, headers={"Content-Type": "application/json"}, method="POST"
    )
    return req


def post_json(body: dict) -> dict:
    resp = urllib.request.urlopen(post(body), context=CTX)
    return json.loads(resp.read())


def post_status(body: dict) -> int:
    try:
        resp = urllib.request.urlopen(post(body), context=CTX)
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

print("=== MCP Protocol (mTLS) ===\n")

print("--- initialize ---")
r = post_json({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
check("returns protocolVersion", "2024-11-05", r["result"]["protocolVersion"])
check("returns server name", "gopher-mcp", r["result"]["serverInfo"]["name"])
check("no error field in success", lambda r: "error" not in r, r)

print("\n--- notifications/initialized ---")
status = post_status({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
check("returns HTTP 204", 204, status)

print("\n--- tools/list ---")
r = post_json({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
tool_names = [t["name"] for t in r["result"]["tools"]]
check("lists gopher_browse", "gopher_browse" in tool_names, True)
check("lists gopher_fetch", "gopher_fetch" in tool_names, True)
check("lists gopher_search", "gopher_search" in tool_names, True)

print("\n--- ping ---")
r = post_json({"jsonrpc": "2.0", "id": 3, "method": "ping", "params": {}})
check("returns result", lambda r: "result" in r, r)

print("\n--- gopher_browse local/ ---")
r = post_json({
    "jsonrpc": "2.0", "id": 4, "method": "tools/call",
    "params": {"name": "gopher_browse", "arguments": {"path": "local/"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("returns Welcome item", lambda _: any(i["display"] == "Welcome to Local Gopher" for i in items), None)
check("returns Submenu item", lambda _: any(i["display"] == "Submenu Example" for i in items), None)
info_items = [i for i in items if i["type"] == "i"]
check("info items have empty path", lambda _: all(i["path"] == "" for i in info_items), None)

print("\n--- gopher_fetch local/welcome ---")
r = post_json({
    "jsonrpc": "2.0", "id": 5, "method": "tools/call",
    "params": {"name": "gopher_fetch", "arguments": {"path": "local/welcome"}}
})
check("returns document content", "never touched a real Gopher wire", r["result"]["content"][0]["text"])

print("\n--- gopher_search local/ query=sub ---")
r = post_json({
    "jsonrpc": "2.0", "id": 6, "method": "tools/call",
    "params": {"name": "gopher_search", "arguments": {"path": "local/", "query": "sub"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("finds Submenu", lambda _: any("Submenu" in i["display"] for i in items), None)

print("\n--- gopher_browse local/sub ---")
r = post_json({
    "jsonrpc": "2.0", "id": 7, "method": "tools/call",
    "params": {"name": "gopher_browse", "arguments": {"path": "local/sub"}}
})
items = json.loads(r["result"]["content"][0]["text"])
check("returns Deep document", lambda _: any("Deep document" in i["display"] for i in items), None)

print("\n--- gopher_fetch local/sub/deep ---")
r = post_json({
    "jsonrpc": "2.0", "id": 8, "method": "tools/call",
    "params": {"name": "gopher_fetch", "arguments": {"path": "local/sub/deep"}}
})
check("returns deep content", "deep in the local hierarchy", r["result"]["content"][0]["text"])

print("\n--- error: unknown tool ---")
r = post_json({
    "jsonrpc": "2.0", "id": 9, "method": "tools/call",
    "params": {"name": "bad_tool", "arguments": {}}
})
check("returns isError:true", True, r["result"].get("isError"))
check("error in content not protocol", lambda r: "error" not in r, r)
check("mentions tool name", "bad_tool", r["result"]["content"][0]["text"])

print("\n--- error: browse on document ---")
r = post_json({
    "jsonrpc": "2.0", "id": 10, "method": "tools/call",
    "params": {"name": "gopher_browse", "arguments": {"path": "local/welcome"}}
})
check("returns isError:true", True, r["result"].get("isError"))

print("\n--- error: unknown method ---")
r = post_json({"jsonrpc": "2.0", "id": 11, "method": "nonexistent", "params": {}})
check("returns JSON-RPC error", -32601, r.get("error", {}).get("code"))

print("\n--- mTLS: reject without client cert ---")
no_client_ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
no_client_ctx.load_verify_locations("certs/ca.crt")
try:
    req = post({"jsonrpc": "2.0", "id": 1, "method": "ping", "params": {}})
    urllib.request.urlopen(req, context=no_client_ctx)
    check("rejects unauthenticated", True, False)
except Exception:
    check("rejects unauthenticated", True, True)

print(f"\n=== Results: {PASS} passed, {FAIL} failed ===")
sys.exit(1 if FAIL else 0)
