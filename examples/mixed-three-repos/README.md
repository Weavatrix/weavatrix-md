# Mixed three-repo example

Minimal neighborhood that `weavatrix-md` can prove without a network or a
running database: a payments API, a Kafka worker, and a checkout web app.

## Layout

```text
mixed-three-repos/
├── payments-api/          # HTTP /charge + Kafka producer + Postgres
│   ├── .env.example
│   ├── src/server.ts
│   └── WEAVATRIX.md       # generated
├── payment-worker/        # Kafka consumer + same Postgres
│   ├── .env.example
│   ├── src/worker.ts
│   └── WEAVATRIX.md       # generated
└── checkout-web/          # HTTP client of payments-api
    ├── src/client.ts
    └── WEAVATRIX.md       # generated
```

Each nested folder must be its own Git repository. Discovery keys off `.git`.

## Source the tool sees

`payments-api/.env.example`:

```env
DATABASE_URL=postgres://payments:example@db.internal:5432/payments
```

`payments-api/src/server.ts` (trimmed):

```ts
app.post("/charge", () => undefined);
await producer.send({ topic: "payment.completed", messages: [] });
```

`payment-worker/.env.example`:

```env
DATABASE_URL=postgres://worker:example@db.internal:5432/payments
```

`payment-worker/src/worker.ts` (trimmed):

```ts
await consumer.subscribe({ topic: "payment.completed" });
```

`checkout-web/src/client.ts`:

```ts
fetch("http://payments-api:3000/charge", { method: "POST" });
```

## Run

From this directory (or any parent that contains the three Git roots):

```bash
# one-time: make each service a Git repo
git init payments-api
git init payment-worker
git init checkout-web

# write WEAVATRIX.md in every nested Git root
weavatrix-md --folder .

# or regenerate a single target against sibling peers
cd payments-api && weavatrix-md
```

Other useful invocations:

```bash
weavatrix-md --folder . --check          # CI: exit 1 if Markdown is stale
weavatrix-md payments-api --stdout       # print one map, do not write
weavatrix-md mcp                         # same generator over stdio MCP
```

## Expected maps

### `payments-api/WEAVATRIX.md`

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

### `payment-worker/WEAVATRIX.md`

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

### `checkout-web/WEAVATRIX.md`

```md
# Weavatrix

Repository: `checkout-web`

## API

- calls → `payments-api`
```

## Proven edges

| Edge | Kind | Why it matches |
|---|---|---|
| `payments-api` ↔ `payment-worker` | Database | same engine + host + logical DB `payments` |
| `payments-api` → `payment-worker` | Kafka | topic `payment.completed` (produce / consume) |
| `checkout-web` → `payments-api` | API | host alias `payments-api` on `/charge` |

The committed `WEAVATRIX.md` files are generated output. Do not edit them by
hand — re-run `weavatrix-md` after changing the example sources.
