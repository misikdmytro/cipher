use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("secret not found")]
    NotFound,

    #[error("api error: {0}")]
    ApiError(String),

    #[error("aws error: {0}")]
    AwsError(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;
