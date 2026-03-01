---
name: note
description: Quickly save a note to the gopher-cli vault
argument-hint: [path] [content or - for interactive]
---

Save a note to the vault.

**Path:** $0 (required — e.g., `ideas/new-feature.md`, `daily/2024-01-15.md`)
**Content:** Everything after the first argument, or interactive if `-` is given.

## Rules

- Prepend `vault/` to the path if the user didn't include it (e.g., `ideas/foo.md` becomes `vault/ideas/foo.md`)
- If content is provided inline, use it directly
- If content is `-` or missing, ask the user what to write
- Always format as markdown with a `# Title` derived from the filename

Publish using the CLI:

```bash
gopher-cli publish vault/[path] --content "[markdown content]"
```

Confirm with the full vault path after publishing.
