# Cipher — Secret Rotation Service

## High-Level Architecture Plan

### Components

- **API** — HTTP REST facade. Owns secrets metadata and webhooks.
- **Scheduler** — Orchestrator. Owns schedules. Triggers rotations via RabbitMQ.
- **Rotator** — Stateless worker. Executes rotation against AWS Secrets Manager.
- **Notificator** — Consumer. Delivers webhook notifications.

---

### State Ownership

| Service     | Owns             |
| ----------- | ---------------- |
| API         | secrets metadata |
| Scheduler   | schedules        |
| Rotator     | —                |
| Notificator | webhooks         |

---

### Communication

| From        | To          | Protocol | Why                                      |
| ----------- | ----------- | -------- | ---------------------------------------- |
| API         | Scheduler   | gRPC     | CRUD schedules, read jobs                |
| API         | Rotator     | gRPC     | Manual rotation trigger                  |
| API         | Notificator | gRPC     | CRUD webhooks                            |
| Scheduler   | RabbitMQ    | AMQP     | Publish rotation.scheduled               |
| RabbitMQ    | Rotator     | AMQP     | Consume rotation.scheduled               |
| Rotator     | API         | gRPC     | Fetch secret metadata by id              |
| Rotator     | RabbitMQ    | AMQP     | Publish rotation.started / done / failed |
| Notificator | RabbitMQ    | AMQP     | Consume rotation.\* events               |

---

### Events

| Event                | Publisher | Subscribers          |
| -------------------- | --------- | -------------------- |
| `rotation.scheduled` | Scheduler | Notificator, Rotator |
| `rotation.started`   | Rotator   | Notificator          |
| `rotation.done`      | Rotator   | Notificator          |
| `rotation.failed`    | Rotator   | Notificator          |

---

### REST API

| Method | Path                           | Description                                       |
| ------ | ------------------------------ | ------------------------------------------------- |
| POST   | /secrets                       | register secret (with role_arn for cross-account) |
| GET    | /secrets                       | list secrets                                      |
| GET    | /secrets/{id}                  | get secret                                        |
| PATCH  | /secrets/{id}                  | update secret                                     |
| DELETE | /secrets/{id}                  | unregister secret                                 |
| POST   | /secrets/{id}/rotate           | manual rotation                                   |
| GET    | /secrets/{id}/jobs             | rotation history                                  |
| GET    | /secrets/{id}/jobs/{job_id}    | job details                                       |
| POST   | /secrets/{id}/webhooks         | register webhook                                  |
| GET    | /secrets/{id}/webhooks         | list webhooks                                     |
| DELETE | /secrets/{id}/webhooks/{wh_id} | remove webhook                                    |

---

### Rotation Flow

**Scheduled:**

1. Scheduler fires → publishes `rotation.scheduled` to RabbitMQ
2. Rotator consumes → calls API gRPC to get secret metadata (incl. role_arn)
3. Rotator assumes IAM Role → executes zero-downtime rotation against AWS SM
4. Rotator publishes `rotation.started`, then `rotation.done` or `rotation.failed`
5. Scheduler consumes → updates job status
6. Notificator consumes → calls API gRPC for webhooks → delivers

**Manual:**

1. Client calls `POST /secrets/{id}/rotate`
2. API calls Rotator gRPC directly
3. Same steps 4–6 as above
