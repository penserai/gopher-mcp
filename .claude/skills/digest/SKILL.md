---
name: digest
description: Read an RSS feed from gopher-mcp, summarize the top stories, and save a digest to the vault
argument-hint: [feed-namespace]
---

Create a digest of the feed at: **$ARGUMENTS**

If no namespace is given, default to `feed.hackernews`.

## Workflow

### 1. Browse the feed

Use `gopher_browse` on the namespace root (e.g., `feed.hackernews/`) to get the list of entries.

### 2. Read every entry

Use `gopher_fetch` on each TextFile item to get the full content. Work through all of them.

### 3. Categorize and summarize

Group the entries by theme (e.g., programming, science, business, culture). For each entry write a 1-2 sentence summary capturing the key point.

### 4. Write the digest

Format as a clean markdown document:

```
# Daily Digest: [Feed Title]
Date: [today]

## [Theme 1]
- **[Title]** — [summary]. [link if available]
- ...

## [Theme 2]
- ...

## Quick Stats
- Total entries: N
- Top themes: [theme1], [theme2], [theme3]
```

### 5. Publish to vault

Use `gopher_publish` to save to `vault/digests/[feed-name]-[YYYY-MM-DD].md`.

Show the user the final digest.
