use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("persistence error: {0}")]
    PersistenceError(String),

    #[error("other error: {0}")]
    Other(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;
