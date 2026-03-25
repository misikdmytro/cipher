use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("persistence error: {0}")]
    PersistenceError(String),

    #[error("not found")]
    NotFound,

    #[error("other error: {0}")]
    Other(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub secret_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
