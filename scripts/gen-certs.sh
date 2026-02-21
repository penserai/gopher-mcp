#!/bin/bash
set -e

mkdir -p certs

# 1. Create CA (v3 with basicConstraints for rustls/webpki compatibility)
openssl req -x509 -sha256 -newkey rsa:4096 -nodes -keyout certs/ca.key -out certs/ca.crt -days 3650 -subj "/CN=gopher-mcp-ca" \
  -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign"

# 2. Create Server Certificate (v3 with SAN and proper key usage)
openssl req -new -newkey rsa:2048 -nodes -keyout certs/server.key -out certs/server.csr -subj "/CN=127.0.0.1"
openssl x509 -req -sha256 -in certs/server.csr -CA certs/ca.crt -CAkey certs/ca.key -CAcreateserial -out certs/server.crt -days 365 \
  -extfile <(printf "subjectAltName=DNS:localhost,IP:127.0.0.1\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth")

# 3. Create Client Certificate for an Agent (v3 with proper key usage)
openssl req -new -newkey rsa:2048 -nodes -keyout certs/client.key -out certs/client.csr -subj "/CN=agent-01"
openssl x509 -req -sha256 -in certs/client.csr -CA certs/ca.crt -CAkey certs/ca.key -CAcreateserial -out certs/client.crt -days 365 \
  -extfile <(printf "keyUsage=digitalSignature\nextendedKeyUsage=clientAuth")

rm certs/*.csr
echo "Generated dev certificates in certs/"
