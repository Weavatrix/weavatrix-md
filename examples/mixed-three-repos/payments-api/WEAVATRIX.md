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
