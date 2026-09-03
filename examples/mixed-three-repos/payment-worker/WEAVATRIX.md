# Weavatrix

Repository: `payment-worker`

## Database

- PostgreSQL / `payments`
  - `payments-api`

## Kafka

- `payment.completed`
  - consumes ← `payments-api`
