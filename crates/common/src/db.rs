use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to connect to database: {0}")]
pub struct DbConnectionError(pub anyhow::Error);
