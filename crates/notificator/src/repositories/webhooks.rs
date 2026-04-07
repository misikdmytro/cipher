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

pub struct ListWebhooksRequest {
    pub secret_id: Uuid,
    pub limit: i64,
    pub offset: i64,
}

pub struct WebhooksPage {
    pub items: Vec<Webhook>,
    pub total: i64,
}

common::repo_error!(AddWebhookError);
common::repo_error!(GetWebhooksError);
common::repo_error!(DeleteWebhookError);

#[async_trait::async_trait]
pub trait WebhooksRepository: Send + Sync {
    async fn save_webhook(&self, request: AddWebhookRequest) -> Result<Webhook, AddWebhookError>;
    async fn get_webhooks_by_secret_id(
        &self,
        secret_id: Uuid,
    ) -> Result<Vec<Webhook>, GetWebhooksError>;
    async fn list_webhooks(
        &self,
        request: ListWebhooksRequest,
    ) -> Result<WebhooksPage, GetWebhooksError>;

    async fn delete_webhook(&self, webhook_id: Uuid) -> Result<bool, DeleteWebhookError>;
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

    async fn list_webhooks(
        &self,
        request: ListWebhooksRequest,
    ) -> Result<WebhooksPage, GetWebhooksError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webhooks WHERE secret_id = $1")
            .bind(request.secret_id)
            .fetch_one(&self.pool)
            .await
            .map_err(GetWebhooksError::from)?;

        let items = sqlx::query_as::<_, Webhook>(
            "SELECT id, secret_id, url, created_at FROM webhooks WHERE secret_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(request.secret_id)
        .bind(request.limit)
        .bind(request.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(GetWebhooksError::from)?;

        Ok(WebhooksPage { items, total })
    }

    async fn delete_webhook(&self, webhook_id: Uuid) -> Result<bool, DeleteWebhookError> {
        let result = sqlx::query("DELETE FROM webhooks WHERE id = $1")
            .bind(webhook_id)
            .execute(&self.pool)
            .await
            .map_err(DeleteWebhookError::from)?;

        Ok(result.rows_affected() > 0)
    }
}
