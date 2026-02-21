# gopher-mcp

MCP server that bridges AI agents to Gopher-style content discovery with mTLS, serving local content and proxying live Gopherspace.

## Quick Start

### 1. Generate Development Certificates
```bash
./scripts/gen-certs.sh
```

### 2. Build and Run the Server
```bash
cargo run -- --seed
```

### 3. Test with Curl (mTLS)
```bash
./scripts/test-mcp.sh
```

### 4. Run without TLS (Development)
```bash
cargo run -- --no-tls --seed
./scripts/test-no-tls.sh
```

## Tools

- `gopher_browse(path)`: List menu items.
- `gopher_fetch(path)`: Retrieve text content.
- `gopher_search(path, query)`: Search/filter content.

Path format: `host/selector` (e.g., `local/welcome`, `gopher.floodgap.com/`)

## Architecture

- **Local Store**: Serves namespaces like `local` from memory.
- **Proxy Client**: Connects to port 70 for external hosts.
- **mTLS**: Uses `rustls` to verify client and server certificates.
