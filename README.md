# weavatrix-md

[![crates.io](https://img.shields.io/crates/v/weavatrix-md.svg)](https://crates.io/crates/weavatrix-md)
[![npm](https://img.shields.io/npm/v/weavatrix-md.svg)](https://www.npmjs.com/package/weavatrix-md)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Tiny deterministic cross-repository integration map for coding agents.

`weavatrix-md` looks at local repositories and writes one `WEAVATRIX.md` next to
`README.md`, `AGENTS.md`, and `CLAUDE.md`. The file answers a single question:

> Which other repositories sit on the other side of this repo’s databases,
> Redis, Vault, Kafka topics, and APIs?

It is not a graph engine, not MCP, and not Weavatrix. It depends only on
[`weavatrix-scan`](https://crates.io/crates/weavatrix-scan) and
[`weavatrix-parse`](https://crates.io/crates/weavatrix-parse).

```bash
cargo install weavatrix-md --locked
# or
npm install -g weavatrix-md

cd payments-api
weavatrix-md
```

Default scope is the current Git repository plus sibling Git repositories in
the parent directory.

```bash
weavatrix-md                  # current repo + siblings
weavatrix-md /path/to/repo    # explicit target + siblings
weavatrix-md --folder ~/work  # every Git repo under the folder
weavatrix-md --check          # CI: fail if WEAVATRIX.md is stale
weavatrix-md --stdout         # print instead of writing
```

## Example

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

A runnable copy of this neighborhood lives in
[`examples/mixed-three-repos`](examples/mixed-three-repos).

`WEAVATRIX.md` is fully generated. Do not edit it by hand.

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

## What it does not do

No MCP, server, watcher, LLM, Git history, blast radius, owners, diagrams, or
edits to `AGENTS.md` / `CLAUDE.md`. RabbitMQ, NATS, SQS, Redis, and runtime
maps are out of scope.

Local mode is offline. It never executes repository code, never opens a
database, and never prints credentials. `.env` secret files stay ignored;
`.env.example` is eligible.

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

## Install

```bash
cargo install weavatrix-md --locked
```

Requires Rust 1.88+. The npm package wraps the same native binary.

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
