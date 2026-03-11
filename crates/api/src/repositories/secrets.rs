use interfaces::secrets::Secret;
use sqlx::PgPool;
use thiserror::Error;

use crate::{config::AppConfig, repositories::models::secrets::Secret as SecretModel};

#[derive(Debug, Clone)]
pub struct AddSecretRequest {
    pub path: String,
}

#[derive(Debug, Error)]
#[error("failed to connect to database: {0}")]
pub struct DbConnectionError(anyhow::Error);

#[derive(Debug, Error)]
pub enum AddSecretError {
    #[error("infrastructure error: {0}")]
    Infrastructure(anyhow::Error),
}

impl From<sqlx::Error> for AddSecretError {
    fn from(e: sqlx::Error) -> Self {
        AddSecretError::Infrastructure(e.into())
    }
}

#[async_trait::async_trait]
pub trait SecretsRepository: Send + Sync {
    async fn save_secret(&self, request: AddSecretRequest) -> Result<Secret, AddSecretError>;
}

struct SecretsRepositoryImpl {
    pool: PgPool,
}

pub async fn new_secrets_repository(
    config: &AppConfig,
) -> Result<impl SecretsRepository + 'static, DbConnectionError> {
    SecretsRepositoryImpl::new(config).await
}

impl SecretsRepositoryImpl {
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
impl SecretsRepository for SecretsRepositoryImpl {
    async fn save_secret(&self, request: AddSecretRequest) -> Result<Secret, AddSecretError> {
        let id = uuid::Uuid::new_v4();
        let created_at = chrono::Utc::now().naive_utc();

        sqlx::query_as::<_, SecretModel>(
            "INSERT INTO secrets (id, path, created_at, updated_at) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(id)
        .bind(request.path)
        .bind(created_at)
        .bind(None::<chrono::NaiveDateTime>)
        .fetch_one(&self.pool)
        .await
        .map_err(AddSecretError::from)
        .map(Secret::from)
    }
}
