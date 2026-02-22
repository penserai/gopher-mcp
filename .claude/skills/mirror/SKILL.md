---
name: mirror
description: Mirror a gopher-mcp source namespace into the vault using dump
argument-hint: [source-namespace] [destination-path]
---

Mirror the source namespace into the vault.

**Source:** $0 (required — e.g., `feed.hackernews/`, `docs/`, `rdf.demo/`)
**Destination:** $1 (optional — defaults to `vault/mirrors/[source-name]`)

## Workflow

### 1. Preview the source

Browse the source root to show the user what's there — how many items, what types, whether there are subdirectories:

```bash
gopher-mcp-tui browse [source]/
```

### 2. Dump

Use the dump command to recursively copy documents:

```bash
gopher-mcp-tui dump [source]/ vault/mirrors/[source-name]
```

Use `--max-depth` if the source looks very deep:

```bash
gopher-mcp-tui dump [source]/ vault/mirrors/[source-name] --max-depth 2
```

### 3. Verify the mirror

Browse the destination to confirm the structure was copied correctly:

```bash
gopher-mcp-tui browse vault/mirrors/[source-name]/
```

### 4. Report

Summarize: how many documents were published, how many skipped, and where the mirror lives.
