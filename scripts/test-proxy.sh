#!/bin/bash
# Test gopher_browse on floodgap (no TLS)
echo "Testing gopher_browse on floodgap.com (no TLS)..."
curl -v -X POST -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "gopher_browse", "arguments": { "path": "gopher.floodgap.com/" } } }' http://127.0.0.1:8443/mcp

echo -e "

Testing gopher_fetch on floodgap.com document (no TLS)..."
curl -v -X POST -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "gopher_fetch", "arguments": { "path": "gopher.floodgap.com/about" } } }' http://127.0.0.1:8443/mcp
