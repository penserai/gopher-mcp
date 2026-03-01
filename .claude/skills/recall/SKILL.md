---
name: recall
description: Search the gopher-cli vault and other sources for previously saved notes, research, and digests
argument-hint: [query]
---

Find information matching: **$ARGUMENTS**

## Workflow

### 1. Search the vault first

Search across all saved notes, research, digests, and mirrors:

```bash
gopher-cli search vault/ "[query]"
```

### 2. Browse relevant directories

If the query suggests a category, browse into it directly:

```bash
gopher-cli browse vault/research/
gopher-cli browse vault/digests/
gopher-cli browse vault/mirrors/
gopher-cli browse vault/notes/
```

### 3. Expand to other sources

If vault results are thin, search across other namespaces:

```bash
gopher-cli search feed.hackernews/ "[query]"
gopher-cli search docs/ "[query]"
```

### 4. Read and present

Fetch the most relevant documents:

```bash
gopher-cli fetch vault/research/matching-note.md
```

Present results to the user with:
- The document path (so they can find it again)
- A brief summary of what's in each match
- The full content of the best match

Keep it concise — the user wants answers, not a wall of text.
