use common::db::DbConnectionError;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::repositories::models::webhooks::Webhook;

#[derive(Debug, Clone)]
pub struct AddWebhookRequest {
    pub secret_id: Uuid,
    pub url: String,
}

common::repo_error!(AddWebhookError);
common::repo_error!(GetWebhooksError);

#[async_trait::async_trait]
pub trait WebhooksRepository: Send + Sync {
    async fn save_webhook(&self, request: AddWebhookRequest) -> Result<Webhook, AddWebhookError>;
    async fn get_webhooks_by_secret_id(
        &self,
        secret_id: Uuid,
    ) -> Result<Vec<Webhook>, GetWebhooksError>;
}

struct WebhooksRepositoryImpl {
    pool: PgPool,
}

pub async fn new_webhooks_repository(
    config: &AppConfig,
) -> Result<impl WebhooksRepository + 'static, DbConnectionError> {
    WebhooksRepositoryImpl::new(config).await
}

impl WebhooksRepositoryImpl {
    pub async fn new(config: &AppConfig) -> Result<Self, DbConnectionError> {
        let pool = PgPool::connect(&config.database.connection_string())
            .await
            .map_err(|e| DbConnectionError(e.into()))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| DbConnectionError(e.into()))?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl WebhooksRepository for WebhooksRepositoryImpl {
    async fn save_webhook(&self, request: AddWebhookRequest) -> Result<Webhook, AddWebhookError> {
        let id = Uuid::new_v4();
        let created_at = chrono::Utc::now().naive_utc();

        sqlx::query_as::<_, Webhook>(
            "INSERT INTO webhooks (id, secret_id, url, created_at) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(id)
        .bind(request.secret_id)
        .bind(request.url)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(AddWebhookError::from)
    }

    async fn get_webhooks_by_secret_id(
        &self,
        secret_id: Uuid,
    ) -> Result<Vec<Webhook>, GetWebhooksError> {
        sqlx::query_as::<_, Webhook>(
            "SELECT id, secret_id, url, created_at FROM webhooks WHERE secret_id = $1",
        )
        .bind(secret_id)
        .fetch_all(&self.pool)
        .await
        .map_err(GetWebhooksError::from)
    }
}
