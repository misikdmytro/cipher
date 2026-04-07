use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request payload for registering a webhook.
#[derive(Deserialize, ToSchema, Validate)]
pub struct RegisterWebhookRequest {
    /// The URL to call when a rotation event occurs for this secret.
    #[schema(example = "https://example.com/webhook")]
    #[validate(url)]
    #[validate(length(min = 1, max = 2048))]
    pub url: String,
}

/// Response returned after successful webhook registration.
#[derive(Serialize, ToSchema)]
pub struct RegisterWebhookResponse {
    /// The ID of the created webhook.
    #[schema(example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")]
    pub id: uuid::Uuid,
}

/// A single webhook entry.
#[derive(Serialize, ToSchema)]
pub struct WebhookResponse {
    #[schema(example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")]
    pub id: uuid::Uuid,
    #[schema(example = "https://example.com/webhook")]
    pub url: String,
    pub created_at: chrono::NaiveDateTime,
}
