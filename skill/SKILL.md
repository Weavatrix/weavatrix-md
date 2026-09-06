---
name: weavatrix-md
description: Generate WEAVATRIX.md — a tiny cross-repository integration map (Database, Redis, Vault, Kafka, API) for coding agents. Use when mapping neighbors across repos, Kafka topics, shared databases, Redis, Vault, or HTTP/gRPC callers.
---

# weavatrix-md

CLI and MCP: `weavatrix-md` / `weavatrix-md mcp`.

```bash
weavatrix-md
weavatrix-md --folder DIR
weavatrix-md --check
weavatrix-md mcp
```

MCP tool `weavatrix_md`:

- `path` — target repository
- `folder` — every Git repo under a directory
- `check` — stale-file gate
- `stdout` — return Markdown instead of writing

Do not edit `WEAVATRIX.md` by hand. Missing edges are acceptable; false edges are not.
