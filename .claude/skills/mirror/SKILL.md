---
name: mirror
description: Mirror a gopher-mcp source namespace into the vault using gopher_dump
argument-hint: [source-namespace] [destination-path]
---

Mirror the source namespace into the vault.

**Source:** $0 (required — e.g., `feed.hackernews/`, `docs/`, `rdf.demo/`)
**Destination:** $1 (optional — defaults to `vault/mirrors/[source-name]`)

## Workflow

### 1. Preview the source

Use `gopher_browse` on the source root to show the user what's there — how many items, what types, whether there are subdirectories.

### 2. Confirm and dump

Use `gopher_dump` with:
- `source`: the source namespace path
- `destination`: the provided destination or `vault/mirrors/[source-name]`
- `max_depth`: 3 (default, unless the source looks very deep)

### 3. Verify the mirror

Use `gopher_browse` on the destination path to confirm the structure was copied correctly. Show the user what landed in the vault.

### 4. Report

Summarize: how many documents were published, how many skipped, and where the mirror lives.
