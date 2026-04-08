CREATE TYPE slot AS ENUM ('blue', 'green');

CREATE TABLE secrets (
    id            UUID PRIMARY KEY,
    cron          VARCHAR(255) NOT NULL,
    strategy_type VARCHAR(32)  NOT NULL,
    provider_type VARCHAR(32)  NOT NULL,
    aws_role_arn  VARCHAR(2048),
    created_at    TIMESTAMP NOT NULL,
    updated_at    TIMESTAMP
);

CREATE TABLE strategy_single (
    secret_id UUID PRIMARY KEY REFERENCES secrets(id) ON DELETE CASCADE,
    path      VARCHAR(255) NOT NULL
);

CREATE TABLE strategy_blue_green (
    secret_id   UUID PRIMARY KEY REFERENCES secrets(id) ON DELETE CASCADE,
    blue_path   VARCHAR(255) NOT NULL,
    green_path  VARCHAR(255) NOT NULL,
    active_slot slot         NOT NULL DEFAULT 'blue',
    CONSTRAINT chk_distinct_paths CHECK (blue_path <> green_path)
);

CREATE INDEX idx_secrets_created_at ON secrets (created_at DESC);
