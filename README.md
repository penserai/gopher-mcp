# gopher-mcp

MCP server that bridges AI agents to Gopher-style content discovery with mTLS, serving local content and proxying live Gopherspace.

## Project Structure

This is a Cargo workspace with two crates:

- **`gopher-mcp-core`** — Framework-agnostic library: MCP handler, Gopher client, router, local store, and adapter trait. No web-framework dependencies.
- **`gopher-mcp-server`** — Binary that wires the core library into an axum HTTP server with mTLS and CLI args.

```
gopher-mcp/
├── gopher-mcp-core/        # library crate (publishable)
│   └── src/
│       ├── lib.rs           # public re-exports
│       ├── mcp.rs           # McpHandler, McpRequest, McpResponse
│       ├── router.rs        # Router (local store + Gopher proxy)
│       ├── gopher.rs        # GopherClient, ItemType, MenuItem
│       ├── store.rs         # LocalStore, ContentNode
│       └── adapters/        # SourceAdapter trait
└── gopher-mcp-server/      # binary crate
    └── src/
        ├── main.rs          # CLI, axum wiring, server bootstrap
        └── tls.rs           # mTLS configuration
```

## Quick Start

### 1. Generate Development Certificates
```bash
./scripts/gen-certs.sh
```

### 2. Build and Run the Server
```bash
cargo run -p gopher-mcp-server
```

### 3. Test with mTLS
```bash
./scripts/test-mcp.py
```

### 4. Run without TLS (Development)
```bash
cargo run -p gopher-mcp-server -- --no-tls
./scripts/test-no-tls.py
```

## Using the Core Library

Add `gopher-mcp-core` as a dependency to use the MCP handler in any Rust project:

```rust
use gopher_mcp_core::{McpHandler, McpRequest, Router, LocalStore};
use std::sync::Arc;

let store = LocalStore::new();
store.seed_example();
let handler = Arc::new(McpHandler::new(Arc::new(Router::new(store))));

// In any web framework handler:
let request: McpRequest = /* deserialize from JSON body */;
match handler.handle(request).await {
    Some(response) => /* serialize response as JSON, return 200 */,
    None => /* return 204 No Content (notification) */,
}
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
