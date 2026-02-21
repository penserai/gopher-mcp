#!/bin/bash
# Test gopher_browse on local (no TLS)
echo "Testing gopher_browse on 'local' namespace (no TLS)..."
curl -v -X POST -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "gopher_browse", "arguments": { "path": "local/" } } }' http://127.0.0.1:8443/mcp

echo -e "\n\nTesting gopher_fetch on local document (no TLS)..."
curl -v -X POST -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "gopher_fetch", "arguments": { "path": "local/welcome" } } }' http://127.0.0.1:8443/mcp

echo -e "\n\nTesting gopher_search on local menu (filtering)..."
curl -v -X POST -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "gopher_search", "arguments": { "path": "local/", "query": "Welcome" } } }' http://127.0.0.1:8443/mcp
