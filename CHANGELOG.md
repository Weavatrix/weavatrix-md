# Changelog

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
