# gopher-mcp

Structured content discovery for humans and agents. Browse local files,
RSS/Atom feeds, RDF knowledge graphs, and Gopher servers through a
single CLI and TUI — no server required.

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
  ░░░░░░░░░░░░░░░░░░░░░░░>(•.•)>░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ░░░░░░░░░░░░░░░░░░░░░░░░░│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
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

All sources are projected into a uniform menu/document hierarchy. Paths use the format `namespace/selector` (e.g., `local/welcome`, `feed.hackernews/`, `vault/notes/idea.md`).

## Quick Start

```bash
cargo build -p gopher-mcp-tui --release

# Browse all namespaces
./target/release/gopher-mcp-tui browse

# Browse a namespace
./target/release/gopher-mcp-tui browse local/

# Read a document
./target/release/gopher-mcp-tui fetch local/welcome

# Launch the interactive TUI
./target/release/gopher-mcp-tui
```

## CLI

The CLI is designed to be the primary interface for both humans and agents. It embeds the full engine — no server process needed.

### Commands

```
gopher-mcp-tui browse [path]                 List items at a path
gopher-mcp-tui fetch <path>                  Retrieve document content
gopher-mcp-tui search <path> <query>         Search within a namespace
gopher-mcp-tui publish <path> [--content ..]  Write a document
gopher-mcp-tui delete <path>                 Delete a document
gopher-mcp-tui dump <source> <dest>          Bulk-copy between namespaces
gopher-mcp-tui [tui]                         Launch interactive browser
```

### Output Format

**Auto-JSON when piped.** When stdout is not a terminal (i.e., captured by a script or agent), output is automatically JSON. On a TTY, output is human-friendly text. Force JSON with `--json`.

```bash
# Human — sees formatted text
gopher-mcp-tui browse local/
# [T] Welcome to gopher-mcp                    local/welcome
#       -----------------------
# [+] Submenu Example                          local/sub

# Agent — stdout piped, gets JSON automatically
result=$(gopher-mcp-tui browse local/)
echo "$result"
# [{"type":"0","type_name":"TextFile","display":"Welcome to gopher-mcp","path":"local/welcome","mime":"text/plain"}, ...]
```

### Browse

List items at a path. With no path, lists all available namespaces.

```bash
# List all namespaces
gopher-mcp-tui browse

# List items in a namespace
gopher-mcp-tui browse local/

# Browse a subdirectory
gopher-mcp-tui browse local/sub

# Browse a live Gopher server
gopher-mcp-tui browse gopher.floodgap.com/
```

### Fetch

Retrieve a document's text content.

```bash
# Read a local document
gopher-mcp-tui fetch local/welcome

# Read an RSS entry
gopher-mcp-tui fetch feed.hackernews/entry/0

# Read from a Gopher server
gopher-mcp-tui fetch gopher.floodgap.com/gopher/tech/

# Save to a file
gopher-mcp-tui fetch feed.hackernews/entry/0 > article.txt
```

### Search

Search within a namespace or path. Filters menu entries by query string; adapters with native search (RDF/SPARQL) use their own engine.

```bash
# Search across a namespace
gopher-mcp-tui search local/ "welcome"

# Search an RSS feed
gopher-mcp-tui search feed.hackernews/ "rust"

# Search a Gopher server's search endpoint
gopher-mcp-tui search gopherpedia.com/ "gopher protocol"
```

### Publish

Write or update a document. Reads content from `--content` or stdin. Only works on writable namespaces (e.g., `vault`).

```bash
# Publish with inline content
gopher-mcp-tui publish vault/notes/idea.md --content "# My Idea
This could work."

# Publish from stdin
echo "Quick note" | gopher-mcp-tui publish vault/scratch.md

# Pipe a file
cat report.md | gopher-mcp-tui publish vault/reports/q1.md

# Pipe from another command
gopher-mcp-tui fetch feed.hackernews/entry/0 | gopher-mcp-tui publish vault/saved/hn-top.md
```

### Delete

Delete a document or directory from a writable namespace.

```bash
gopher-mcp-tui delete vault/notes/idea.md
```

### Dump

Recursively copy documents from a source into a writable namespace. Walks menus up to `--max-depth` levels (default: 3).

```bash
# Mirror an RSS feed into the vault
gopher-mcp-tui dump feed.hackernews/ vault/mirrors/hn

# Mirror with limited depth
gopher-mcp-tui dump rdf.demo/ vault/mirrors/rdf --max-depth 2

# Mirror a Gopher server subtree
gopher-mcp-tui dump gopher.floodgap.com/gopher/tech vault/mirrors/floodgap-tech
```

### Global Options

| Option | Env Var | Description |
|--------|---------|-------------|
| `--url <URL>` | `GOPHER_MCP_URL` | Connect to a remote gopher-mcp server instead of embedded engine |
| `--no-seed` | | Skip seeding example content into the `local` namespace |
| `--json` | | Force JSON output (auto-enabled when stdout is piped) |

### Connection Precedence

The CLI defaults to the embedded engine. To connect to a remote server instead:

1. `--url` flag (highest priority)
2. `GOPHER_MCP_URL` environment variable
3. `url` field in `~/.gopher-mcp.toml`
4. Embedded engine (default — no server needed)

```bash
# Use a remote server for one command
gopher-mcp-tui --url http://localhost:8443 browse

# Set for the whole session
export GOPHER_MCP_URL=http://localhost:8443
gopher-mcp-tui browse
gopher-mcp-tui fetch local/welcome
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error (details on stderr) |

Errors are structured JSON when output is piped:

```bash
gopher-mcp-tui fetch nonexistent/path 2>&1 >/dev/null
# {"error":"Fetch failed: Selector not found: ..."}
```

## Agent Usage

The CLI is built so agents can use Bash tool calls instead of MCP. Auto-JSON output, structured errors, pipe-friendly publish, and composable commands.

### Discovery

```bash
# What namespaces are available?
gopher-mcp-tui browse

# What's in a namespace?
gopher-mcp-tui browse vault/

# Drill into a directory
gopher-mcp-tui browse vault/research/
```

### Read

```bash
# Get a document
gopher-mcp-tui fetch vault/research/topic.md

# Search then read the first result
gopher-mcp-tui search vault/ "machine learning"
# → pick a path from the results
gopher-mcp-tui fetch vault/research/ml-overview.md
```

### Write

```bash
# Save a note
gopher-mcp-tui publish vault/notes/idea.md --content "# Idea
Something interesting."

# Pipe content from another source
gopher-mcp-tui fetch feed.hackernews/entry/3 \
  | gopher-mcp-tui publish vault/saved/article.md
```

### Bulk Operations

```bash
# Copy all articles from a feed into the vault
gopher-mcp-tui dump feed.hackernews/ vault/mirrors/hn

# Verify what was copied
gopher-mcp-tui browse vault/mirrors/hn/
```

### Agent Workflow Example

An agent researching a topic might:

```bash
# 1. Search feeds for relevant content
gopher-mcp-tui search feed.hackernews/ "rust async"

# 2. Read the interesting entries
gopher-mcp-tui fetch feed.hackernews/entry/5

# 3. Search existing research notes
gopher-mcp-tui search vault/ "rust"

# 4. Save new findings
gopher-mcp-tui publish vault/research/rust-async.md --content "# Rust Async Patterns
..."

# 5. Build a mirror for offline access
gopher-mcp-tui dump feed.hackernews/ vault/mirrors/hn-$(date +%Y-%m-%d)
```

### JSON Output Reference

**Browse / Search** returns an array of items:

```json
[
  {
    "type": "1",
    "type_name": "Menu",
    "display": "Submenu Example",
    "path": "local/sub",
    "mime": "application/x-gopher-menu"
  },
  {
    "type": "0",
    "type_name": "TextFile",
    "display": "Welcome to gopher-mcp",
    "path": "local/welcome",
    "mime": "text/plain"
  }
]
```

**Fetch** returns path and content:

```json
{
  "path": "local/welcome",
  "content": "This is a local document served by gopher-mcp.\nContent here is served directly from the local store."
}
```

**Publish / Delete** returns confirmation:

```json
{ "ok": true, "path": "vault/notes/idea.md", "action": "published" }
```

**Dump** returns counts:

```json
{
  "ok": true,
  "source": "feed.hackernews/",
  "destination": "vault/mirrors/hn",
  "published": 15,
  "skipped": 2
}
```

**Errors** (on stderr):

```json
{ "error": "Fetch failed: Selector not found: /missing in local" }
```

Item types in browse/search results:

| `type` | `type_name` | Meaning |
|--------|-------------|---------|
| `1` | Menu | Directory — pass to `browse` |
| `0` | TextFile | Document — pass to `fetch` |
| `7` | Search | Search endpoint — pass to `search` |
| `h` | Html | HTML document — pass to `fetch` |
| `i` | Info | Display-only text (no path) |

## TUI

The interactive terminal browser. Launches when no subcommand is given.

```bash
gopher-mcp-tui
gopher-mcp-tui tui                    # explicit
gopher-mcp-tui tui local/             # start at a specific path
```

### Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate menu |
| `Enter` | Open item |
| `b` or `Backspace` | Go back |
| `Tab` | Switch pane |
| `Space` | Page down (content pane) |
| `/` | Search |
| `:` | GoTo popup |
| `Tab` (in GoTo) | Expand/collapse directory |
| `Home` | Go to root |
| `PgUp`/`PgDn` | Scroll content |
| `q` | Quit |

## Config File

Place at `~/.gopher-mcp.toml`. Used by both CLI commands and the TUI.

```toml
# Remote mode — uncomment to connect to a running gopher-mcp-server.
# Comment out (or remove) to use the embedded engine.
# url = "http://127.0.0.1:8443"

# Gopherspace sources (shown in the TUI GoTo popup)
sources = [
    "gopher.floodgap.com/",
    "gopher.quux.org/",
    "cosmic.voyage/",
]

# Adapters — synced at startup in embedded mode.
# Adapter namespaces are auto-added to the TUI GoTo popup.

# RSS / Atom feeds
[[adapter]]
type = "rss"
namespace = "feed.hackernews"
url = "https://hnrss.org/frontpage"

# File system directory (writable = agent vault)
[[adapter]]
type = "fs"
namespace = "vault"
root = "/path/to/.gopher-mcp-vault"
writable = true

# File system directory (read-only)
[[adapter]]
type = "fs"
namespace = "docs"
root = "/path/to/notes"
extensions = ["md", "txt"]

# RDF knowledge graph
[[adapter]]
type = "rdf"
namespace = "rdf.dbpedia"
sparql_endpoint = "https://dbpedia.org/sparql"
format = "turtle"
```

### Adapter Types

| Type | Required fields | Optional fields |
|------|----------------|-----------------|
| `rss` | `namespace`, `url` | |
| `fs` | `namespace`, `root` | `extensions` (e.g., `["md", "txt"]`), `writable` (bool) |
| `rdf` | `namespace` | `source` (file or URL), `format`, `sparql_endpoint` |

## Project Structure

Cargo workspace with three crates:

- **`gopher-mcp-core`** — Framework-agnostic library: content router, local store, Gopher client, MCP handler, and adapter trait.
- **`gopher-mcp-server`** — axum HTTP server with mTLS. Use when you need a persistent server process.
- **`gopher-mcp-tui`** — CLI + TUI binary. Embeds the core engine. Self-contained.

```
gopher-mcp/
├── gopher-mcp-core/        # library crate
│   └── src/
│       ├── lib.rs           # public re-exports
│       ├── mcp.rs           # McpHandler (JSON-RPC over HTTP)
│       ├── router.rs        # Router (local store + Gopher proxy)
│       ├── gopher.rs        # GopherClient, ItemType, MenuItem
│       ├── store.rs         # LocalStore, ContentNode
│       └── adapters/        # SourceAdapter trait + fs, rss, rdf
├── gopher-mcp-server/      # server binary
│   └── src/
│       ├── main.rs          # CLI, axum wiring, mTLS
│       └── tls.rs           # certificate handling
└── gopher-mcp-tui/         # CLI + TUI binary
    └── src/
        ├── main.rs          # subcommands, embedded/remote startup
        ├── cli.rs           # CLI command handlers, auto-JSON
        ├── app.rs           # TUI state and logic
        ├── client.rs        # ContentClient trait, McpClient, EmbeddedClient
        ├── config.rs        # TuiConfig, AdapterConfig, adapter creation
        └── ui.rs            # ratatui rendering
```

## Server

For use cases that need a persistent HTTP endpoint (e.g., MCP integration with AI tools).

```bash
# Generate dev certificates
./scripts/gen-certs.sh

# Run with mTLS
cargo run -p gopher-mcp-server

# Run without TLS (development)
cargo run -p gopher-mcp-server -- --no-tls

# Run with adapters
cargo run -p gopher-mcp-server -- --no-tls --config examples/rss-demo.toml
```

Point the CLI at the server:

```bash
gopher-mcp-tui --url http://127.0.0.1:8443 browse local/
```

## Cross-Compilation

```bash
make build              # Release build for host (darwin-arm64)
make build-darwin-arm64 # aarch64-apple-darwin
make build-darwin-x86   # x86_64-apple-darwin
make build-linux-arm64  # aarch64-unknown-linux-gnu (requires cross)
make build-linux-x86    # x86_64-unknown-linux-gnu (requires cross)
make build-all          # All architectures
```

Darwin targets use native `cargo build`. Linux targets use [`cross`](https://github.com/cross-rs/cross) (Docker-based). Binaries output to `dist/`.
