---
name: recall
description: Search the gopher-mcp vault and other sources for previously saved notes, research, and digests
argument-hint: [query]
---

Find information matching: **$ARGUMENTS**

## Workflow

### 1. Search the vault first

Use `gopher_search` on `vault/` with the query. This searches across all saved notes, research, digests, and mirrors.

### 2. Browse relevant directories

If the query suggests a category, browse into it directly:
- Research notes → `vault/research/`
- Daily digests → `vault/digests/`
- Mirrors → `vault/mirrors/`
- General notes → `vault/notes/`, `vault/ideas/`, `vault/daily/`

### 3. Expand to other sources

If vault results are thin, also search across other namespaces (feeds, docs, RDF graphs) using `gopher_search`.

### 4. Read and present

Use `gopher_fetch` to read the most relevant documents. Present them to the user with:
- The document path (so they can find it again)
- A brief summary of what's in each match
- The full content of the best match

Keep it concise — the user wants answers, not a wall of text.
