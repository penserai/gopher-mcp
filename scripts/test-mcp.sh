#!/bin/bash

# Test gopher_browse on local
echo "Testing gopher_browse on 'local' namespace..."
curl -v --cacert certs/ca.crt --cert certs/client.crt --key certs/client.key \
     -X POST -H "Content-Type: application/json" \
     -d '{ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "gopher_browse", "arguments": { "path": "local/" } } }' \
     https://127.0.0.1:8443/mcp

echo -e "\n\nTesting gopher_fetch on local document..."
curl -v --cacert certs/ca.crt --cert certs/client.crt --key certs/client.key \
     -X POST -H "Content-Type: application/json" \
     -d '{ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "gopher_fetch", "arguments": { "path": "local/welcome" } } }' \
     https://127.0.0.1:8443/mcp
