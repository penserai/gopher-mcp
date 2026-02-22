---
name: research
description: Research a topic across gopher-mcp sources, read relevant documents, and save a structured summary to the vault
argument-hint: [topic]
---

Research the topic: **$ARGUMENTS**

Follow this workflow using the gopher-mcp tools:

## 1. Discover sources

Use `gopher_browse` on each known namespace root to see what's available. Common namespaces include RSS feeds (`feed.*`), RDF graphs (`rdf.*`), local docs (`docs`), and the vault (`vault`). Start by browsing each to understand the landscape.

## 2. Search for relevant content

Use `gopher_search` across every namespace with keywords from the topic. Cast a wide net — try synonyms and related terms.

## 3. Read the most relevant documents

Use `gopher_fetch` to read the top results. Extract key facts, quotes, and insights. Note which source each piece came from.

## 4. Synthesize findings

Produce a structured research note in markdown:

```
# Research: [topic]
Date: [today]

## Summary
[2-3 sentence overview]

## Key Findings
- [finding 1 — with source]
- [finding 2 — with source]
- ...

## Details
[longer analysis organized by theme]

## Sources
- [namespace/path] — [what it contained]
```

## 5. Save to vault

Use `gopher_publish` to save the note to `vault/research/[slugified-topic].md`.

Confirm what you saved and where.
