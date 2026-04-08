# Cipher

> Distributed secret rotation service for AWS Secrets Manager, written in Rust.

Cipher manages cloud secrets and rotates them on a schedule (or on demand) without downtime. It is built as a small workspace of independent services that communicate over gRPC for synchronous calls and RabbitMQ for rotation lifecycle events. State ownership is split per service so each one can be scaled, deployed, and reasoned about in isolation.

## Highlights

- Async Rust (Tokio) end to end across four services
- gRPC via [tonic](https://github.com/hyperium/tonic) for synchronous service-to-service calls
- RabbitMQ via [lapin](https://github.com/amqp-rs/lapin) for asynchronous rotation events
- Two pluggable rotation strategies: **single-path** and **blue/green**
- Cross-account AWS access via STS `AssumeRole`
- Webhook notifications for every step of the rotation lifecycle
- OpenAPI / Swagger UI generated from code with [utoipa](https://github.com/juhaku/utoipa)
- Postgres-backed state with `sqlx` migrations
- Cron scheduling powered by [apalis](https://github.com/geofmureithi/apalis)
- Graceful shutdown and health endpoints per service

## Architecture

Four services, one shared message bus, three Postgres databases.

### Services

| Service         | Role                                               | Owns             |
| --------------- | -------------------------------------------------- | ---------------- |
| **API**         | HTTP (Axum) + gRPC facade. Front door for clients. | secrets metadata |
| **Scheduler**   | Cron orchestrator. Fires rotations on schedule.    | schedules / jobs |
| **Rotator**     | Stateless worker. Talks to AWS Secrets Manager.    | —                |
| **Notificator** | Consumer. Delivers webhook notifications.          | webhooks         |

### Communication

| From        | To          | Protocol | Why                                    |
| ----------- | ----------- | -------- | -------------------------------------- |
| Client      | API         | HTTP     | REST CRUD for secrets and webhooks     |
| API         | Scheduler   | gRPC     | Register rotation schedules            |
| API         | Rotator     | gRPC     | Manual rotation trigger                |
| API         | Notificator | gRPC     | CRUD webhooks                          |
| Scheduler   | RabbitMQ    | AMQP     | Publish `rotation.scheduled`           |
| RabbitMQ    | Rotator     | AMQP     | Consume `rotation.scheduled`           |
| Rotator     | API         | gRPC     | Fetch secret metadata by id            |
| Rotator     | RabbitMQ    | AMQP     | Publish `rotation.started/done/failed` |
| Notificator | RabbitMQ    | AMQP     | Consume `rotation.*` events            |

### Events

| Event                | Publisher | Subscribers          |
| -------------------- | --------- | -------------------- |
| `rotation.scheduled` | Scheduler | Rotator, Notificator |
| `rotation.started`   | Rotator   | Notificator          |
| `rotation.done`      | Rotator   | Notificator          |
| `rotation.failed`    | Rotator   | Notificator          |

## Rotation flow

**Scheduled rotation**

1. Scheduler fires the cron job and publishes `rotation.scheduled`.
2. Rotator consumes the event and calls API gRPC to fetch the secret config.
3. Rotator assumes the configured IAM role and rotates the secret in AWS.
4. Rotator publishes `rotation.started`, then `rotation.done` or `rotation.failed`.
5. Notificator consumes the lifecycle events and delivers them to registered webhooks.

**Manual rotation**

1. Client calls `POST /secrets/{id}/rotate`.
2. API forwards the request to Rotator over gRPC.
3. Steps 3–5 above.

## Rotation strategies

- **Single** — one secret path, rotated in place. Simple, but the secret is briefly inconsistent during the swap.
- **Blue / green** — two paths (`blue_path`, `green_path`) with an `active_slot`. The new value is written to the _inactive_ slot, validated, then the active slot is flipped atomically. Zero-downtime, safe rollback.

## Getting started

**Prerequisites**

- Rust toolchain (edition 2024)
- Docker (for Postgres + RabbitMQ)
- AWS credentials available to the rotator (env vars or `~/.aws/config`)

**Run it locally**

```bash
# 1. start Postgres + RabbitMQ
docker compose up -d

# 2. start each service in its own terminal
cargo run -p api
cargo run -p scheduler
cargo run -p rotator
cargo run -p notificator
```

Each service reads its own `*.config.toml` from the repo root, so commands must be run from the workspace root.

Useful endpoints once the API is up:

- Swagger UI — <http://localhost:3000/swagger-ui>
- OpenAPI JSON — <http://localhost:3000/api-docs/openapi.json>
- RabbitMQ management — <http://localhost:15672> (`cipher` / `cipher`)

## Configuration

Each service has its own TOML config at the repo root. Override host/port, database, RabbitMQ, downstream service addresses, and log level there.

- [api.config.toml](api.config.toml)
- [scheduler.config.toml](scheduler.config.toml)
- [rotator.config.toml](rotator.config.toml)
- [notificator.config.toml](notificator.config.toml)

The Postgres init script in [docker/postgres/init.sql](docker/postgres/init.sql) creates the three databases (`api`, `scheduler`, `notificator`) on first boot.
