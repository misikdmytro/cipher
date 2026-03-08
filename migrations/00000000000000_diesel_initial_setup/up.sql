CREATE TABLE secrets(
    id UUID PRIMARY KEY,
    path VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP
);