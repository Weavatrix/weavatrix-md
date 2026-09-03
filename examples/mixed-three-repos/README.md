# Mixed three-repo example

A payments API, a Kafka worker, and a checkout web app. Each service needs to
be a Git repository (discovery keys off `.git`):

```bash
git init payments-api payment-worker checkout-web
weavatrix-md --folder .
```

That writes `WEAVATRIX.md` in each nested Git repository.

Expected edges:

- `payments-api` shares PostgreSQL / `payments` with `payment-worker`
- `payments-api` produces `payment.completed` to `payment-worker`
- `checkout-web` calls `payments-api`

The committed Markdown files are generated output. Do not edit them by hand.
