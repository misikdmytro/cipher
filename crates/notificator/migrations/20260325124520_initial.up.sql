CREATE TABLE webhooks(
    id UUID PRIMARY KEY,
    secret_id UUID NOT NULL,
    url VARCHAR(2048) NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_webhooks_secret_id ON webhooks(secret_id);
