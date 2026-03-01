#!/usr/bin/env python3
"""Test source adapters via MCP endpoints.

Starts a server with a temporary FS adapter config, verifies browse/fetch
work correctly, then cleans up.

Requires: cargo build completed (uses cargo run)
"""

import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import urllib.request
import urllib.error

URL = "http://127.0.0.1:18443/mcp"
PASS = 0
FAIL = 0


def post_json(body: dict) -> dict:
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        URL, data=data, headers={"Content-Type": "application/json"}, method="POST"
    )
    resp = urllib.request.urlopen(req)
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


def wait_for_server(timeout=15):
    """Wait for server to start accepting connections."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            post_json({"jsonrpc": "2.0", "id": 0, "method": "ping", "params": {}})
            return True
        except (urllib.error.URLError, ConnectionRefusedError):
            time.sleep(0.3)
    return False


def main():
    global PASS, FAIL

    # Create temporary test directory with files
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create test file structure
        os.makedirs(os.path.join(tmpdir, "content", "subdir"))

        with open(os.path.join(tmpdir, "content", "hello.txt"), "w") as f:
            f.write("Hello from the FS adapter!")

        with open(os.path.join(tmpdir, "content", "readme.md"), "w") as f:
            f.write("# Test README\n\nThis is a test document.")

        with open(os.path.join(tmpdir, "content", "subdir", "nested.txt"), "w") as f:
            f.write("Nested document content.")

        # Create a gophermap in the subdir
        with open(os.path.join(tmpdir, "content", "subdir", ".gophermap"), "w") as f:
            f.write(
                "iCustom Gophermap\t\t\t0\n"
                "0Nested File\t/subdir/nested.txt\tdocs\t0\n"
            )

        # Create TOML config
        config_path = os.path.join(tmpdir, "config.toml")
        content_path = os.path.join(tmpdir, "content")
        with open(config_path, "w") as f:
            f.write(f'[[adapter]]\ntype = "fs"\nnamespace = "docs"\nroot = "{content_path}"\n')

        # Start server — use pre-built binary for speed, fall back to cargo run
        print("Starting gopher-cli server with FS adapter config...")
        project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        binary = os.path.join(project_root, "target", "debug", "gopher-cli-server")
        if os.path.exists(binary):
            cmd = [binary]
        else:
            cmd = ["cargo", "run", "-p", "gopher-cli-server", "--"]
        cmd += [
            "--no-tls", "--no-seed",
            "--bind", "127.0.0.1:18443",
            "--config", config_path,
        ]
        server = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=project_root,
        )

        try:
            if not wait_for_server():
                print("FATAL: Server did not start within timeout")
                server.terminate()
                sys.exit(1)

            print("\n=== Source Adapter Tests ===\n")

            # Test 1: Browse root menu of FS adapter
            print("--- browse docs/ (FS adapter root) ---")
            r = post_json({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": "gopher_browse", "arguments": {"path": "docs/"}}
            })
            items = json.loads(r["result"]["content"][0]["text"])
            check("root menu has items", lambda _: len(items) >= 2, None)

            # Check we have both files and the subdir
            displays = [i["display"] for i in items]
            check("contains hello.txt", lambda _: any("hello.txt" in d for d in displays), None)
            check("contains subdir", lambda _: any("subdir" in d for d in displays), None)

            # Test 2: Fetch a text file
            print("\n--- fetch docs/hello.txt ---")
            r = post_json({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": {"name": "gopher_fetch", "arguments": {"path": "docs/hello.txt"}}
            })
            check(
                "returns file content",
                "Hello from the FS adapter!",
                r["result"]["content"][0]["text"],
            )

            # Test 3: Browse subdirectory (with gophermap)
            print("\n--- browse docs/subdir (gophermap) ---")
            r = post_json({
                "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": {"name": "gopher_browse", "arguments": {"path": "docs/subdir"}}
            })
            items = json.loads(r["result"]["content"][0]["text"])
            check("subdir has items from gophermap", lambda _: len(items) >= 1, None)
            displays = [i["display"] for i in items]
            check("gophermap has Custom Gophermap info", lambda _: any("Custom" in d or "Nested" in d for d in displays), None)

            # Test 4: Fetch nested file
            print("\n--- fetch docs/subdir/nested.txt ---")
            r = post_json({
                "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                "params": {"name": "gopher_fetch", "arguments": {"path": "docs/subdir/nested.txt"}}
            })
            check(
                "returns nested file content",
                "Nested document content.",
                r["result"]["content"][0]["text"],
            )

            # Test 5: Search within FS namespace
            print("\n--- search docs/ query=hello ---")
            r = post_json({
                "jsonrpc": "2.0", "id": 5, "method": "tools/call",
                "params": {"name": "gopher_search", "arguments": {"path": "docs/", "query": "hello"}}
            })
            items = json.loads(r["result"]["content"][0]["text"])
            check("search finds hello.txt", lambda _: any("hello" in i["display"].lower() for i in items), None)

            # Test 6: Verify non-existent selector returns error
            print("\n--- fetch docs/nonexistent ---")
            r = post_json({
                "jsonrpc": "2.0", "id": 6, "method": "tools/call",
                "params": {"name": "gopher_fetch", "arguments": {"path": "docs/nonexistent"}}
            })
            check("returns error for missing selector", True, r["result"].get("isError"))

            print(f"\n=== Results: {PASS} passed, {FAIL} failed ===")

        finally:
            server.send_signal(signal.SIGTERM)
            server.wait(timeout=5)

    sys.exit(1 if FAIL else 0)


if __name__ == "__main__":
    main()
