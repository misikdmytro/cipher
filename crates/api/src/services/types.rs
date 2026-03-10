use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Persistence error: {0}")]
    PersistenceError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type ServiceResult<T> = Result<T, ServiceError>;
