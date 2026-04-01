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

#[derive(Debug, Clone)]
pub enum RotationResult {
    Single {
        path: String,
    },
    BlueGreen {
        active_slot: String,
        active_path: String,
        ready_slot: String,
        ready_path: String,
    },
}
