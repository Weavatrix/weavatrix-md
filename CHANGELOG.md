# Changelog

## 0.2.1 — 2026-09-06

- Kafka: bind `process.env.X || 'topic'` defaults; treat `sendMessage` /
  `sendMessages` / `initConsumerForATopic` as produce/consume; prefer topic
  suffix (`_in` / `_out`) and flag name over misleading help text.
- API: unique service-host alias is enough for an edge (no longer requires the
  provider to expose a matching route).

## 0.2.0 — 2026-09-03

- MCP: `weavatrix-md mcp` and tool `weavatrix_md`.
- Kafka: Go `flag.String` / `flag.StringVar` topic defaults and kafka-go Reader/Writer.
- Shared `logs` / similar topics match when a unique producer/consumer pair exists.
- Redis and Vault identities (`redis://`, `REDIS_URL`, `VAULT_ADDR`) as neighborhood sections.

## 0.1.0 — 2026-09-03

- Local CLI: current repo, explicit path, `--folder`, `--check`, `--stdout`.
- Kafka producer/consumer matching with optional cluster hints.
- Database identity for PostgreSQL, MongoDB, MySQL, ClickHouse, SQL Server.
- Strict HTTP resolver (host/service alias, unique non-generic routes) plus gRPC service identity.
- Atomic `WEAVATRIX.md` write. No network in local mode.
