---
name: recall
description: Search the gopher-mcp vault and other sources for previously saved notes, research, and digests
argument-hint: [query]
---

Find information matching: **$ARGUMENTS**

## Workflow

### 1. Search the vault first

Search across all saved notes, research, digests, and mirrors:

```bash
gopher-mcp-tui search vault/ "[query]"
```

### 2. Browse relevant directories

If the query suggests a category, browse into it directly:

```bash
gopher-mcp-tui browse vault/research/
gopher-mcp-tui browse vault/digests/
gopher-mcp-tui browse vault/mirrors/
gopher-mcp-tui browse vault/notes/
```

### 3. Expand to other sources

If vault results are thin, search across other namespaces:

```bash
gopher-mcp-tui search feed.hackernews/ "[query]"
gopher-mcp-tui search docs/ "[query]"
```

### 4. Read and present

Fetch the most relevant documents:

```bash
gopher-mcp-tui fetch vault/research/matching-note.md
```

Present results to the user with:
- The document path (so they can find it again)
- A brief summary of what's in each match
- The full content of the best match

Keep it concise — the user wants answers, not a wall of text.
