# weavatrix-md

[![crates.io](https://img.shields.io/crates/v/weavatrix-md.svg)](https://crates.io/crates/weavatrix-md)
[![npm](https://img.shields.io/npm/v/weavatrix-md.svg)](https://www.npmjs.com/package/weavatrix-md)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Tiny deterministic cross-repository integration map for coding agents.

`weavatrix-md` looks at local repositories and writes one `WEAVATRIX.md` next to
`README.md`, `AGENTS.md`, and `CLAUDE.md`. The file answers a single question:

> Which other repositories sit on the other side of this repo’s databases,
> Redis, Vault, Kafka topics, and APIs?

It is not a graph engine and not Weavatrix Core. Local analysis depends only on
[`weavatrix-scan`](https://crates.io/crates/weavatrix-scan) and
[`weavatrix-parse`](https://crates.io/crates/weavatrix-parse). An optional stdio
MCP host (`weavatrix-md mcp`, tool `weavatrix_md`) wraps the same generator.

## Install

```bash
cargo install weavatrix-md --locked
# or
npm install -g weavatrix-md
```

Requires Rust 1.88+ for the crates.io install. The npm package ships the same
native binary for win32/darwin/linux on x64 and arm64 (Node 18+). After npm
install you also get `weavatrix-md-mcp` (same as `weavatrix-md mcp`).

## Quick start

Default scope is the current Git repository plus sibling Git repositories in
the parent directory:

```bash
cd ~/work/payments-api
weavatrix-md
# -> writes payments-api/WEAVATRIX.md
#    peers: payment-worker, checkout-web (siblings under ~/work)
```

```bash
weavatrix-md                  # current repo + siblings
weavatrix-md /path/to/repo    # explicit target + siblings
weavatrix-md --folder ~/work  # every Git repo under the folder
weavatrix-md --check          # CI: fail if WEAVATRIX.md is stale
weavatrix-md --stdout         # print instead of writing
weavatrix-md mcp              # MCP stdio host (tool weavatrix_md)
```

## Example: three sibling services

A runnable neighborhood lives in
[`examples/mixed-three-repos`](examples/mixed-three-repos):

| Repo | Role | Evidence the tool reads |
|---|---|---|
| `payments-api` | HTTP API + Kafka producer | `.env.example` DB URL, `producer.send({ topic: "payment.completed" })` |
| `payment-worker` | Kafka consumer + same DB | `.env.example` DB URL, `consumer.subscribe({ topic: "payment.completed" })` |
| `checkout-web` | HTTP client | `fetch("http://payments-api:3000/charge")` |

```bash
cd examples/mixed-three-repos
git init payments-api payment-worker checkout-web   # discovery keys off .git
weavatrix-md --folder .
```

Generated `payments-api/WEAVATRIX.md`:

```md
# Weavatrix

Repository: `payments-api`

## Database

- PostgreSQL / `payments`
  - `payment-worker`

## Kafka

- `payment.completed`
  - produces → `payment-worker`

## API

- called by ← `checkout-web`
```

Generated `payment-worker/WEAVATRIX.md`:

```md
# Weavatrix

Repository: `payment-worker`

## Database

- PostgreSQL / `payments`
  - `payments-api`

## Kafka

- `payment.completed`
  - consumes ← `payments-api`
```

Generated `checkout-web/WEAVATRIX.md`:

```md
# Weavatrix

Repository: `checkout-web`

## API

- calls → `payments-api`
```

`WEAVATRIX.md` is fully generated. Do not edit it by hand.

### CI stale check

```bash
weavatrix-md --folder examples/mixed-three-repos --check
# exit 0 if committed files match; exit 1 if stale
```

### Print without writing

```bash
weavatrix-md examples/mixed-three-repos/payments-api --stdout
```

## What it proves

| Section | Identity | Direction |
|---|---|---|
| Database | engine + host + logical database | neighborhood, no arrows |
| Redis | host + logical database | neighborhood, no arrows |
| Vault | host | neighborhood, no arrows |
| Kafka | topic, including Go `flag.String` defaults and kafka-go | `produces →` / `consumes ←` |
| API | host/service alias, or a globally unique non-generic route | `calls →` / `called by ←` |

Precision over recall. A missing edge is acceptable. A false edge in committed
agent context is not.

It will **not** link repositories because they share a `users` table, a
`DATABASE_URL` key, or `GET /health`.

### Redis / Vault example

When two repos share the same Redis or Vault identity (for example
`REDIS_URL=redis://cache.internal:6379/0` or `VAULT_ADDR=https://vault.internal`),
they appear under neighborhood sections — without produce/consume arrows:

```md
## Redis

- Redis / `0`
  - `payment-worker`

## Vault

- `vault.internal`
  - `payment-worker`
```

## What it does not do

No watcher, LLM, Git history, blast radius, owners, diagrams, or edits to
`AGENTS.md` / `CLAUDE.md`. RabbitMQ, NATS, SQS, and runtime maps are out of
scope. Redis and Vault appear only as neighborhood identity sections, not as
message-bus edges.

Local mode is offline. It never executes repository code, never opens a
database, and never prints credentials. `.env` secret files stay ignored;
`.env.example` is eligible. The MCP host uses the same offline path.

## Agent integration

The generator does not patch agent instruction files. Point the agent at the
artifact once:

**Claude Code** — in `CLAUDE.md`:

```md
@WEAVATRIX.md
```

**Codex / Cursor / Copilot CLI** — in `AGENTS.md`:

```md
For cross-repository changes, read WEAVATRIX.md first.
```

**MCP** — stdio host, tool `weavatrix_md`:

```bash
weavatrix-md mcp
# npm also installs: weavatrix-md-mcp
```

Example tool arguments:

```json
{
  "folder": "/Users/you/work",
  "check": false,
  "stdout": false
}
```

```json
{
  "path": "/Users/you/work/payments-api",
  "stdout": true
}
```

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo bench --bench generation
```

Benches live in `benches/generation.rs`. Recorded numbers:
[`benchmark-results/generation.txt`](benchmark-results/generation.txt).
Release binary on this workstation: **1.1 MiB** stripped.

## License

MIT. Copyright (c) 2026 Sergii Ziborov.
