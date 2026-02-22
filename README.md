# gopher-mcp

MCP server for structured content discovery. Connects AI agents to local files,
RSS/Atom feeds, RDF knowledge graphs, and Gopher servers through uniform
tools: browse, fetch, search, publish, and delete.

```
       ┌───────┐     ┌───────┐     ┌────────┐     ┌───────┐
   ^   │ files │  ^  │ feeds │  ^  │ graphs │  ^  │  :70  │   ^
  /|\  └───┬───┘ /|\ └───┬───┘ /|\ └────┬───┘ /|\ └───┬───┘  /|\
  ~~~~~~~~~│~~~~~~~~~~~~~│~~~~~~~~~~~~~~│~~~~~~~~~~~~~~│~~~~~~~~~
  ░░░░░░░░░│░░░░░░░░░░░░░│░░░░░░░░░░░░░░│░░░░░░░░░░░░░░│░░░░░░░
  ░░░░┌────┘░░░░░░░┌─────┘░░░░░░░░┌─────┘░░░░░░░░┌─────┘░░░░░░░
  ░░░░│░░░░░░░░░░░░│░░░░░░░░░░░░░░│░░░░░░░░░░░░░░│░░░░░░░░░░░░░
  ░░░░└─────┐░░░░░░└──────┐░░░░░░░└──────┐░░░░░░░│░░░░░░░░░░░░░
  ░░░░░░░░░░└─────────────┴──────────────┴───────┘░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░░░░░░░░░│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░░░░░>(•.•)>░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░░░░░░░░│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░┌───────┴───────┐░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░│  ◊  V A U L T │░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░│  publish  ↓↑  │░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░│  delete   ×   │░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░│  browse   ☰   │░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░│  fetch    ◆   │░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░│  search   ⌕   │░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░└───────────────┘░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
```

## What It Does

| Source | Example | How it works |
|--------|---------|-------------|
| **File System** | Agent vault, Jekyll `_posts/`, any directory tree | Directories become menus, text files become documents. Writable namespaces support publish/delete. |
| **RSS / Atom** | Hacker News, blog feeds | Feed entries become documents under a channel menu |
| **RDF / SPARQL** | Knowledge graphs, DBpedia, local Turtle files | Classes become menus, resources become documents, SPARQL backs search |
| **Gopher servers** | `gopher.floodgap.com` | Transparent TCP proxy to live Gopherspace |

All sources are projected into a uniform menu/document hierarchy and accessed through the same three MCP tools.

## Project Structure

This is a Cargo workspace with two crates:

- **`gopher-mcp-core`** — Framework-agnostic library: MCP handler, content router, local store, and adapter trait. No web-framework dependencies.
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

- `gopher_browse(path)`: Navigate a content hierarchy. Returns structured items with type, display text, and navigable path.
- `gopher_fetch(path)`: Retrieve a document's text content.
- `gopher_search(path, query)`: Search a search endpoint or filter local menu entries.
- `gopher_publish(path, content)`: Write or update a document. Creates parent directories as needed. Only works on writable namespaces.
- `gopher_delete(path)`: Delete a document or directory. Only works on writable namespaces.

Path format: `host/selector` (e.g., `docs/readme.md`, `feed.hn/entry/0`, `gopher.floodgap.com/`)

## Architecture

- **Source Adapters**: Project files, feeds, and knowledge graphs into navigable menus and documents.
- **Local Store**: Serves namespaces from memory.
- **Gopher Proxy**: Connects to port 70 for live Gopher servers.
- **mTLS**: Uses `rustls` to verify client and server certificates.
