# Architecture

Internal design notes for gopher-cli. For usage documentation, see [README.md](README.md).

## Design Principles

- **Embedded-first** — the CLI binary contains the full content engine. No server process needed. A `--url` flag switches to remote mode when required.
- **Uniform content model** — all sources (files, feeds, graphs, Gopher servers) project into the same menu/document hierarchy. One path format (`namespace/selector`), one set of operations.
- **Agent-friendly** — auto-JSON when stdout is piped, structured errors on stderr, pipe-friendly stdin for publish, composable commands. Agents can use the Bash tool instead of MCP.
- **Pluggable adapters** — new data sources implement the `SourceAdapter` trait. Each adapter maps a foreign data model onto menus and documents.

## Crate Architecture

```
┌─────────────────────────────────────────────────────┐
│                   gopher-cli                        │
│                                                     │
│  ┌─────────────┐        ┌──────────────────┐        │
│  │ CLI commands │───────▶│  ContentClient   │        │
│  │ (cli.rs)     │        │  (trait)         │        │
│  └─────────────┘        └──────┬───────────┘        │
│  ┌─────────────┐               │                    │
│  │ TUI          │──────────────┤                    │
│  │ (app/ui.rs)  │              │                    │
│  └─────────────┘        ┌──────┴───────────┐        │
│                         │  EmbeddedClient  │        │
│                         │  (Arc<Router>)   │        │
│                         └──────┬───────────┘        │
│                                │                    │
│          ┌─────────────────────┘                    │
│          ▼                                          │
│  ┌──────────────────────────────────────────┐       │
│  │            gopher-cli-core               │       │
│  │                                          │       │
│  │  Router ──▶ LocalStore (namespace/sel)   │       │
│  │    │           ▲                         │       │
│  │    │           │                         │       │
│  │    │       Adapters (rss, fs, rdf)       │       │
│  │    │                                     │       │
│  │    └──▶ GopherClient ──TCP:70──▶ :70     │       │
│  └──────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│              gopher-cli-server                      │
│                                                     │
│  axum ──▶ McpHandler ──▶ Router (same core)         │
│  mTLS                                               │
└─────────────────────────────────────────────────────┘
```

### ContentClient Trait

The CLI and TUI interact with content through `ContentClient`, which has two implementations:

| Implementation | When used | How it works |
|---------------|-----------|-------------|
| `EmbeddedClient` | Default (no `--url`) | Wraps `Arc<Router>` directly. In-process calls. |
| `McpClient` | When `--url` is set | HTTP JSON-RPC to a remote `gopher-cli-server`. |

Both implement the same six operations: browse, fetch, search, publish, delete, dump.

### Startup Flow (Embedded Mode)

1. Parse CLI args, load `~/.gopher-cli.toml`
2. Create `LocalStore`, optionally seed example content
3. Create adapters from config, sync each into the store
4. Build `Router`, wrap in `Arc`, create `EmbeddedClient`
5. If subcommand → run CLI handler; else → launch TUI

### Routing Logic

Given a path like `feed.hackernews/entry/0`:

1. Split on first `/` → namespace=`feed.hackernews`, selector=`/entry/0`
2. Check if namespace is registered locally → yes (RSS adapter synced it)
3. Look up `/entry/0` in the `feed.hackernews` namespace
4. Return the document content

Given `gopher.floodgap.com/fun/jokes`:

1. Split → namespace=`gopher.floodgap.com`, selector=`/fun/jokes`
2. Not a local namespace → treat as Gopher server hostname
3. TCP connect to `gopher.floodgap.com:70`, send `/fun/jokes\r\n`
4. Parse response, return structured result

### Local Store

Two-level map: `namespace → selector → ContentNode`. `ContentNode` is either `Menu(Vec<MenuItem>)` or `Document(String)`. Protected by `RwLock` for concurrent reads with exclusive writes.

### Source Adapters

Each adapter implements:

```rust
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn namespace(&self) -> &str;
    async fn sync(&self, store: &LocalStore) -> Result<(), AdapterError>;
    async fn search(&self, selector: &str, query: &str) -> Option<Vec<MenuItem>>;
}
```

**RSS/Atom** — Parses feed XML. Channel becomes root menu, entries become documents with title/date/content/links. Categories map to submenus.

**File System** — Walks a directory tree. Directories → menus, text files → documents. Respects `.gophermap` files for custom menu layouts. Writable mode enables publish/delete with automatic menu regeneration.

**RDF/SPARQL** — Parses Turtle/RDF-XML/N-Triples. `rdf:type` classes become submenus, individual resources become documents listing their triples. SPARQL endpoints back native search queries.

### Gopher Client

Minimal TCP client: connect to `host:70`, send `selector\r\n`, read response (capped at 2 MiB), parse. 10-second timeout. No persistent connections. Handles text files (type 0), menus (type 1), and search queries (type 7 with `selector\tquery\r\n`).

### MCP Server (gopher-cli-server)

JSON-RPC endpoint at `/mcp`. Handles MCP lifecycle (`initialize`, `tools/list`, `tools/call`, `ping`). All six operations exposed as MCP tools. Optional mTLS with rustls for mutual authentication — agent identity from client cert CN.

### Auto-JSON Output

`cli.rs` checks `std::io::IsTerminal` on stdout. When piped (not a terminal), output is JSON. When on a TTY, output is human-formatted text with type indicators (`[T]`, `[+]`, `[?]`, etc.). The `--json` flag forces JSON mode.

Errors are always on stderr. When JSON mode is active, errors are also structured JSON: `{"error": "..."}`.

## Feature Flags

Adapters are feature-gated to allow building without optional dependencies:

```
adapter-rss  → feed-rs, reqwest
adapter-fs   → (no extra deps)
adapter-rdf  → sophia, reqwest
adapter-all  → all of the above (default)
```

All three crates (`gopher-cli-core`, `gopher-cli`, `gopher-cli-server`) mirror this feature gate pattern.

## Future Work

- **Live sync** — background refresh for adapters with configurable TTL
- **Agent access control** — per-agent namespace permissions via client cert CN
- **Binary content** — fetch and serve images, archives
- **Persistent store** — back the local store with SQLite or filesystem
- **Caching** — TTL-based cache for proxied Gopher content
