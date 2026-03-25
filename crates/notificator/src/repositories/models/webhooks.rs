use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow, Debug, Clone)]
pub struct Webhook {
    pub id: Uuid,
    pub secret_id: Uuid,
    pub url: String,
    pub created_at: chrono::NaiveDateTime,
}
