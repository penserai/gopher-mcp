# Examples

Ready-to-run adapter configurations for gopher-mcp. Each example demonstrates a different source adapter projecting external data into navigable menus and documents for AI agents.

## Quick Start

```bash
cargo build -p gopher-mcp-server
cargo run -p gopher-mcp-server -- --no-tls --config examples/rss-demo.toml
```

Then browse the content via MCP tools (`gopher_browse`, `gopher_fetch`, `gopher_search`).

## Configs

### `rss-demo.toml` — RSS/Atom Feed

Fetches the Hacker News front page feed and serves it as a navigable menu. Each entry becomes a fetchable document with title, date, summary, and links.

```bash
cargo run -p gopher-mcp-server -- --no-tls --config examples/rss-demo.toml
```

Navigation:
```
feed.hackernews/              → root menu (feed title + entries)
feed.hackernews/entry/0       → first entry (title, date, content, links)
feed.hackernews/entry/1       → second entry
feed.hackernews/category/...  → category submenus (if the feed uses them)
```

### `rdf-demo.toml` — Local RDF (Turtle)

Parses `sample.ttl` (a small RDF graph of people, projects, and languages) and builds a class-centric navigation hierarchy.

```bash
cargo run -p gopher-mcp-server -- --no-tls --config examples/rdf-demo.toml
```

Navigation:
```
rdf.demo/                                    → root menu listing classes
rdf.demo/class/http_example.org_Person       → menu of Person instances
rdf.demo/class/http_example.org_Project      → menu of Project instances
rdf.demo/resource/http_example.org_alice     → Alice's triples as a document
rdf.demo/resource/http_example.org_gopher-mcp → project properties
```

### `fs-demo.toml` — File System / Wiki

Serves a local directory tree as navigable content. Directories become menus, text files become documents. Supports `.gophermap` files for curated landing pages.

```bash
# Point it at any directory — an Obsidian vault, Jekyll _posts/, a wiki clone, etc.
cargo run -p gopher-mcp-server -- --no-tls --config examples/fs-demo.toml
```

Navigation:
```
docs/                  → root menu (auto-generated or from .gophermap)
docs/subdir            → subdirectory menu
docs/subdir/page.md    → document content
```

### `remote-rdf-demo.toml` — Remote RDF over HTTP

Fetches a Turtle file from DBpedia (Rust programming language, hundreds of triples) and builds a navigable class/resource hierarchy from it.

```bash
cargo run -p gopher-mcp-server -- --no-tls --config examples/remote-rdf-demo.toml
```

### `multi-adapter-demo.toml` — Multiple Adapters

Combines an RSS feed, a remote RDF source, and a local Turtle file in a single config. All three sync at startup and coexist under separate namespaces.

```bash
cargo run -p gopher-mcp-server -- --no-tls --config examples/multi-adapter-demo.toml
```

### `gopher-mcp.toml` — Reference Config

A fully-commented config showing all adapter types and their options. All entries are commented out — uncomment and customize for your setup.

Adapter types:

| Type | Required fields | Optional fields |
|------|----------------|-----------------|
| `rss` | `namespace`, `url` | — |
| `fs` | `namespace`, `root` | `extensions` (e.g. `[".md", ".txt"]`) |
| `rdf` | `namespace`, `format` | `source` (file or URL), `sparql_endpoint` |

## Data Files

### `sample.ttl` — Sample RDF Graph

A small Turtle file used by `rdf-demo.toml`. Contains three classes (Person, Project, Language) with instances and relationships between them:

```
Person:   alice, bob, carol
Project:  gopher-mcp, iron-wolf
Language: rust, haskell
```

Relationships include `worksOn`, `knows`, `language`, and standard `rdf:type`/`rdfs:label` predicates.

## Combining Adapters

A single config file can declare multiple adapters. They all sync at startup and coexist under separate namespaces:

```toml
[[adapter]]
type = "rss"
namespace = "feed.hn"
url = "https://hnrss.org/frontpage"

[[adapter]]
type = "fs"
namespace = "docs"
root = "/path/to/wiki"
extensions = [".md"]

[[adapter]]
type = "rdf"
namespace = "rdf.data"
source = "https://example.org/data.ttl"
format = "turtle"
sparql_endpoint = "https://example.org/sparql"
```

## Remote RDF Sources

The RDF adapter's `source` field accepts URLs — the data is fetched over HTTP at startup:

```toml
[[adapter]]
type = "rdf"
namespace = "rdf.rust"
source = "https://dbpedia.org/data/Rust_(programming_language).ttl"
format = "turtle"
```

For SPARQL-only mode (no local data, just search proxy):

```toml
[[adapter]]
type = "rdf"
namespace = "rdf.dbpedia"
format = "turtle"
sparql_endpoint = "https://dbpedia.org/sparql"
```

Note: SPARQL search uses `CONTAINS` on `rdfs:label` which works well on small/medium endpoints but may time out on large public stores like DBpedia that require vendor-specific full-text indexes.
