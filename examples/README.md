# Examples

Ready-to-run adapter configurations for gopher-cli. Each example demonstrates a different source adapter projecting external data into navigable menus and documents.

## Using Examples

Every example works two ways — embedded via the CLI, or through the server.

### CLI (embedded, no server)

Copy the example config to `~/.gopher-cli.toml` and use the CLI directly:

```bash
cp examples/rss-demo.toml ~/.gopher-cli.toml
gopher-cli browse feed.hackernews/
gopher-cli fetch feed.hackernews/entry/0
```

Or launch the TUI:

```bash
gopher-cli
```

### Server

Start a server with the config and connect to it:

```bash
cargo run -p gopher-cli-server -- --no-tls --config examples/rss-demo.toml
gopher-cli --url http://127.0.0.1:8443 browse feed.hackernews/
```

## Configs

### `rss-demo.toml` — RSS/Atom Feed

Fetches the Hacker News front page feed and serves it as a navigable menu. Each entry becomes a fetchable document with title, date, summary, and links.

```bash
# CLI
cp examples/rss-demo.toml ~/.gopher-cli.toml
gopher-cli browse feed.hackernews/
gopher-cli fetch feed.hackernews/entry/0
gopher-cli search feed.hackernews/ "rust"

# Server
cargo run -p gopher-cli-server -- --no-tls --config examples/rss-demo.toml
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
# CLI
cp examples/rdf-demo.toml ~/.gopher-cli.toml
gopher-cli browse rdf.demo/
gopher-cli fetch "rdf.demo/resource/http_example.org_alice"

# Server
cargo run -p gopher-cli-server -- --no-tls --config examples/rdf-demo.toml
```

Navigation:
```
rdf.demo/                                    → root menu listing classes
rdf.demo/class/http_example.org_Person       → menu of Person instances
rdf.demo/class/http_example.org_Project      → menu of Project instances
rdf.demo/resource/http_example.org_alice     → Alice's triples as a document
rdf.demo/resource/http_example.org_gopher-cli → project properties
```

### `vault-demo.toml` — Agent Vault (Writable)

A writable namespace backed by the filesystem. Agents can publish, browse, fetch, search, and delete documents. Parent directories are created automatically on publish; menus regenerate from the directory structure after every write or delete.

```bash
# CLI
cp examples/vault-demo.toml ~/.gopher-cli.toml

gopher-cli publish vault/notes/idea.md --content "# My idea
This could work."
gopher-cli browse vault/
gopher-cli browse vault/notes/
gopher-cli fetch vault/notes/idea.md
gopher-cli search vault/ "idea"
gopher-cli delete vault/notes/idea.md

# Pipe content from stdin
echo "Quick note" | gopher-cli publish vault/scratch.md

# Copy an article from a feed into the vault
gopher-cli fetch feed.hackernews/entry/0 \
  | gopher-cli publish vault/saved/hn-top.md

# Bulk-copy an entire feed into the vault
gopher-cli dump feed.hackernews/ vault/mirrors/hn

# Server
cargo run -p gopher-cli-server -- --no-tls --config examples/vault-demo.toml
```

### `fs-demo.toml` — File System / Wiki

Serves a local directory tree as navigable content. Directories become menus, text files become documents. Supports `.gophermap` files for curated landing pages.

```bash
# CLI
cp examples/fs-demo.toml ~/.gopher-cli.toml
gopher-cli browse docs/
gopher-cli fetch docs/subdir/page.md

# Server
cargo run -p gopher-cli-server -- --no-tls --config examples/fs-demo.toml
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
# CLI
cp examples/remote-rdf-demo.toml ~/.gopher-cli.toml
gopher-cli browse rdf.rust/

# Server
cargo run -p gopher-cli-server -- --no-tls --config examples/remote-rdf-demo.toml
```

### `multi-adapter-demo.toml` — Multiple Adapters

Combines an RSS feed, a remote RDF source, and a local Turtle file in a single config. All three sync at startup and coexist under separate namespaces.

```bash
# CLI
cp examples/multi-adapter-demo.toml ~/.gopher-cli.toml
gopher-cli browse                # see all namespaces
gopher-cli browse feed.hn/       # browse the feed
gopher-cli browse rdf.demo/      # browse the RDF graph

# Server
cargo run -p gopher-cli-server -- --no-tls --config examples/multi-adapter-demo.toml
```

### `gopher-cli.toml` — TUI + CLI Config

Config for the standalone binary. Shows embedded mode (no `url`), gopherspace sources for the TUI GoTo popup, and adapter examples.

```bash
cp examples/gopher-cli.toml ~/.gopher-cli.toml
gopher-cli          # TUI
gopher-cli browse   # CLI
```

### `gopher-cli.toml` — Reference Config

A fully-commented config showing all adapter types and their options. All entries are commented out — uncomment and customize for your setup.

Adapter types:

| Type | Required fields | Optional fields |
|------|----------------|-----------------|
| `rss` | `namespace`, `url` | — |
| `fs` | `namespace`, `root` | `extensions` (e.g. `["md", "txt"]`), `writable` (bool) |
| `rdf` | `namespace`, `format` | `source` (file or URL), `sparql_endpoint` |

## Data Files

### `sample.ttl` — Sample RDF Graph

A small Turtle file used by `rdf-demo.toml`. Contains three classes (Person, Project, Language) with instances and relationships between them:

```
Person:   alice, bob, carol
Project:  gopher-cli, iron-wolf
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
extensions = ["md"]

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
