# gopher-mcp

MCP server that bridges AI agents to Gopher-style content discovery with mTLS, serving local content and proxying live Gopherspace.

## Motivation

The Gopher protocol's menu-driven, hierarchical content model is a natural fit for agentic content discovery. Unlike the web, where agents must parse arbitrary HTML and guess at navigation structure, Gopher's typed menus provide explicit, machine-readable navigation with clear semantics — every item declares what it is and where it leads.

gopher-mcp wraps this content model into MCP, giving agents structured tools for browsing, fetching, and searching content. It operates in hybrid mode: content may originate locally (never touching a real Gopher server), be ingested from other data formats and protocols (RDF, SPARQL endpoints, Atom/RSS feeds, file systems), or be transparently proxied from live Gopherspace over TCP.

The Gopher content model acts as a universal presentation layer — a simple, navigable hierarchy that agents can traverse regardless of where the underlying data lives. This makes gopher-mcp a bridge not just to Gopherspace, but to any structured data source that can be projected into menus and documents.

mTLS provides mutual authentication so the server knows which agent is connecting and agents can verify the server's identity.

## Goals

- Expose Gopher's hierarchical content model to MCP-connected agents
- Support hybrid content: local-native, ingested from external data sources, and proxied from real Gopher servers
- Enable pluggable data sources — project RDF graphs, SPARQL endpoints, RSS/Atom feeds, file systems, and other structured data into navigable Gopher menus
- Secure all agent communication with mutual TLS
- Keep the `host/selector` path format as the single, clean addressing scheme
- Maintain Gopher's stateless simplicity — each tool call is independent

## Non-Goals

- Implementing a standalone Gopher server (no TCP port 70 listener)
- Replacing or competing with Gemini
- Full-text indexing or crawling of Gopherspace
- Binary file serving through MCP (text content only in v0.1)
- Being a general-purpose data transformation pipeline — sources are projected into Gopher's menu/document model, not arbitrary query interfaces

## Architecture

```
                    ┌──────────────────────────────────────────────────────┐
                    │                    gopher-mcp                        │
                    │                                                      │
Agent ──mTLS──▶    │  MCP Handler ──▶ Router ──▶ Local Store              │
                    │                     │          ▲                     │
                    │                     │          │                     │
                    │                     │      Source Adapters           │
                    │                     │      ├── RDF / SPARQL         │
                    │                     │      ├── RSS / Atom           │
                    │                     │      ├── File System          │
                    │                     │      └── Custom               │
                    │                     │                               │
                    │                     └──▶ Gopher Client ─────────────┼──TCP:70──▶ Real Gopher Server
                    │                                                      │
                    └──────────────────────────────────────────────────────┘
```

### Components

**MCP Handler** — JSON-RPC endpoint at `/mcp`. Handles MCP lifecycle (`initialize`, `tools/list`, `tools/call`, `ping`) and dispatches tool calls to the router.

**Router** — Parses `host/selector` paths, checks whether the host is a registered local namespace, and routes accordingly. The routing decision is transparent to the agent.

**Local Store** — In-memory content store organized as namespaces containing menus and documents. Content follows Gopher's model (typed items in hierarchical menus) but is served purely through MCP. Namespaces are registered at startup or dynamically via source adapters.

**Source Adapters** — Pluggable modules that ingest external data and project it into the local store as Gopher menus and documents. Each adapter maps a foreign data model onto the menu/document hierarchy:

- **RDF / SPARQL** — Navigate an RDF graph as menus. Classes and predicates become menu items, triples become documents. SPARQL endpoints can back search queries (type 7).
- **RSS / Atom** — Feed entries become text documents under a channel menu. Categories map to submenus.
- **File System** — Directories become menus, files become documents. Respects `.gophermap` if present.
- **Custom** — Trait-based interface for implementing new adapters.

**Gopher Client** — Minimal TCP client that connects to real Gopher servers on port 70. Sends `selector\r\n`, reads the response, and parses it. Handles text files (type 0 with `.` terminator), menus (type 1), and search queries (type 7 with `selector\tquery\r\n`).

**mTLS Layer** — rustls-based mutual TLS. The server presents its certificate and validates client certificates against a configured CA. Agent identity is derived from the client certificate's Common Name.

### Routing Logic

Given a path like `floodgap.com/fun/jokes`:

1. Split on first `/` → host=`floodgap.com`, selector=`/fun/jokes`
2. Check if `floodgap.com` is a registered local namespace → no
3. Open TCP connection to `floodgap.com:70`
4. Send `/fun/jokes\r\n`
5. Read response, parse, return structured result

Given a path like `local/docs/rfc1436`:

1. Split on first `/` → host=`local`, selector=`/docs/rfc1436`
2. Check if `local` is a registered local namespace → yes
3. Look up `/docs/rfc1436` in the local store
4. Return the document content

## Tools

### gopher_browse

Navigate a menu. Returns structured items with type, display text, navigable path, and MIME hint.

```json
{
  "name": "gopher_browse",
  "inputSchema": {
    "properties": {
      "path": { "type": "string", "description": "host/selector" }
    },
    "required": ["path"]
  }
}
```

**Response:**
```json
{
  "items": [
    {
      "type": "1",
      "type_name": "Menu",
      "display": "Fun and Games",
      "path": "floodgap.com/fun",
      "mime": "application/x-gopher-menu"
    },
    {
      "type": "0",
      "type_name": "TextFile",
      "display": "About this server",
      "path": "floodgap.com/about",
      "mime": "text/plain"
    }
  ],
  "count": 2
}
```

### gopher_fetch

Retrieve a document's text content.

```json
{
  "name": "gopher_fetch",
  "inputSchema": {
    "properties": {
      "path": { "type": "string", "description": "host/selector" }
    },
    "required": ["path"]
  }
}
```

**Response:**
```json
{
  "content": "This is a Gopher-MCP hybrid server.\nContent here never touched a real Gopher wire.\n"
}
```

### gopher_search

Execute a search query against a Gopher search endpoint (type 7) or filter local menu entries.

```json
{
  "name": "gopher_search",
  "inputSchema": {
    "properties": {
      "path": { "type": "string", "description": "host/selector for search endpoint" },
      "query": { "type": "string" }
    },
    "required": ["path", "query"]
  }
}
```

**Response:** Same structure as `gopher_browse`.

## mTLS Setup

### Certificate Hierarchy

```
CA (self-signed)
├── Server cert (presented by gophergate)
└── Client certs (one per agent)
    ├── agent-01
    ├── agent-02
    └── ...
```

### Agent Identity

The agent's identity is extracted from the client certificate's Common Name (CN) field. This enables:

- Access control per agent
- Audit logging of which agent accessed what content
- Namespace scoping (restrict agents to specific local namespaces)

### Configuration

| Environment Variable | Default | Description |
|---|---|---|
| `GOPHER_MCP_CERT` | `certs/server.crt` | Server certificate chain (PEM) |
| `GOPHER_MCP_KEY` | `certs/server.key` | Server private key (PEM) |
| `GOPHER_MCP_CLIENT_CA` | `certs/ca.crt` | CA cert for client verification (PEM) |

## Content Model

### Gopher Item Types Supported

| Type | Code | Description | MCP Behavior |
|---|---|---|---|
| Text File | `0` | Plain text document | Returned by `gopher_fetch` |
| Menu | `1` | Directory listing | Returned by `gopher_browse` |
| Search | `7` | Full-text search | Used by `gopher_search` |
| Binary | `9` | Binary file | Proxied raw (future) |
| GIF | `g` | GIF image | Listed in menus |
| Image | `I` | Generic image | Listed in menus |
| Info | `i` | Display-only text | Included in menu listings |
| HTML | `h` | HTML document | Listed in menus |

### Local Content Nodes

Local content is either a **Menu** (containing typed entries pointing to other nodes) or a **Document** (leaf node with text content). This mirrors Gopher's two fundamental content types.

### Source Adapter Model

Source adapters implement a trait that projects external data into the local store:

```rust
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Unique namespace this adapter registers (e.g., "rdf.mydata", "feed.hackernews")
    fn namespace(&self) -> &str;

    /// Populate or refresh content in the local store
    async fn sync(&self, store: &LocalStore) -> Result<(), AdapterError>;

    /// Optional: handle search queries natively instead of filtering menu entries
    async fn search(&self, selector: &str, query: &str) -> Option<Vec<MenuItem>>;
}
```

#### RDF / SPARQL Adapter

Maps an RDF graph into navigable menus:

| RDF Concept | Gopher Mapping |
|---|---|
| Named graph / dataset | Namespace root menu |
| `rdf:type` classes | Submenus grouping instances |
| Individual resources | Menu items (linking to their property document) |
| Resource properties | Text document listing predicate-object pairs |
| SPARQL endpoint | Backs `gopher_search` with native queries |

Example navigation:
```
rdf.mydata/                        → root menu listing classes
rdf.mydata/class/Person            → menu of Person instances
rdf.mydata/resource/alice          → document showing Alice's triples
rdf.mydata/sparql                  → search endpoint (type 7)
```

An agent browsing `rdf.mydata/class/Person` sees a menu of all `?s rdf:type :Person` results, each linking to a document that renders that resource's properties as readable text.

#### RSS / Atom Adapter

| Feed Concept | Gopher Mapping |
|---|---|
| Feed channel | Namespace root menu |
| Categories / tags | Submenus |
| Feed entries | Text documents (title + content) |
| Entry links | Info lines with URLs |

#### File System Adapter

| FS Concept | Gopher Mapping |
|---|---|
| Directory | Menu |
| Text file | Text document (type 0) |
| Binary file | Binary item (type 9) |
| `.gophermap` | Explicit menu override |

---

## Implementation Details

### Project Structure

```
gopher-mcp/
├── Cargo.toml
├── README.md
├── PLAN.md
├── scripts/
│   └── gen-certs.sh           # Generate dev CA, server, and client certs
└── src/
    ├── main.rs                # Entry point, CLI args, server bootstrap
    ├── gopher.rs              # Item types, menu parser, TCP client
    ├── store.rs               # Local content store (namespaces, menus, docs)
    ├── router.rs              # Path parsing, local vs remote routing
    ├── mcp.rs                 # MCP JSON-RPC handler, tool definitions
    ├── tls.rs                 # mTLS configuration with rustls
    └── adapters/              # Source adapters (future)
        ├── mod.rs             # SourceAdapter trait definition
        ├── rdf.rs             # RDF / SPARQL adapter
        ├── feed.rs            # RSS / Atom adapter
        └── fs.rs              # File system adapter
```

### Dependencies

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime, TCP streams |
| `axum` | HTTP framework for MCP endpoint |
| `axum-server` | TLS-enabled server (rustls backend) |
| `rustls` | TLS implementation |
| `rustls-pemfile` | PEM file parsing |
| `tokio-rustls` | Async TLS streams |
| `serde` / `serde_json` | JSON serialization for MCP protocol |
| `tracing` | Structured logging |
| `thiserror` | Error type derivation |
| `anyhow` | Top-level error handling |
| `uuid` | Request ID generation |

### Gopher Client Implementation

The embedded Gopher client is intentionally minimal:

1. Open `TcpStream` to `host:70`
2. Write `selector\r\n` (or `selector\tquery\r\n` for search)
3. Shutdown write half
4. Read response into buffer (capped at 2 MiB)
5. For text: strip trailing `.\r\n` terminator
6. For menus: parse tab-delimited lines into `MenuItem` structs

Timeout is 10 seconds. No persistent connections, no keep-alive — true to Gopher's one-request-per-connection model.

### Menu Line Parsing

Gopher menu lines follow the format: `<type_char><display>\t<selector>\t<host>\t<port>\r\n`

The parser handles:
- Standard 4-field tab-delimited lines
- Malformed lines (common with `i` info items that omit fields)
- The `.` end-of-menu terminator
- Both `\r\n` and `\n` line endings

### MCP Protocol Handling

The server implements the streamable HTTP transport with a single POST endpoint at `/mcp`. Supported methods:

| Method | Purpose |
|---|---|
| `initialize` | Return server info and capabilities |
| `notifications/initialized` | Acknowledge (no response) |
| `tools/list` | Return tool definitions with schemas and annotations |
| `tools/call` | Execute a tool and return results |
| `ping` | Health check |

Tool errors are returned as MCP tool results with `isError: true` rather than JSON-RPC protocol errors, following MCP best practices.

### Local Store Design

The store uses a two-level map: `namespace → selector → ContentNode`. The `RwLock` allows concurrent reads with exclusive writes. Content can be registered programmatically at startup or via future management tools.

The `--seed` flag (default: on) populates a `local` namespace with example content for testing.

### CLI Interface

```
gopher-mcp [OPTIONS]

Options:
  --bind, -b <ADDR>    Bind address (default: 127.0.0.1:8443)
  --no-tls             Disable mTLS (development mode)
  --no-seed            Don't seed example content
```

### Future Work

- **Source adapters** — implement the `SourceAdapter` trait and ship RDF/SPARQL, RSS/Atom, and file system adapters as the primary v0.2 feature
- **Adapter configuration** — TOML/YAML config file for declaring adapters, their namespaces, and sync schedules
- **Live sync** — background refresh for adapters with configurable TTL (e.g., poll an RSS feed every 15 minutes)
- **SPARQL-backed search** — route `gopher_search` calls on RDF namespaces to native SPARQL queries instead of menu filtering
- **Agent access control** — enforce per-agent namespace permissions based on client cert CN
- **Content management tools** — MCP tools for agents to publish content to local namespaces
- **Binary content** — support fetching and serving binary files (images, archives)
- **Gopher+ extensions** — handle Gopher+ metadata attributes
- **Caching** — cache proxied Gopher content with TTL
- **Crawling** — optional background crawl of configured Gopher servers for indexing
- **Persistent store** — back the local store with SQLite or filesystem instead of in-memory only
