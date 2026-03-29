use common::db::DbConnectionError;
use interfaces::secrets::Secret;
use sqlx::PgPool;

use crate::{config::AppConfig, repositories::models::secrets::Secret as SecretModel};

#[derive(Debug, Clone)]
pub struct AddSecretRequest {
    pub path: String,
    pub cron_expression: String,
    pub aws_role_arn: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteSecretRequest {
    pub id: uuid::Uuid,
}

common::repo_error!(AddSecretError);
common::repo_error!(DeleteSecretError, NotFound);
common::repo_error!(GetSecretError, NotFound);

#[async_trait::async_trait]
pub trait SecretsRepository: Send + Sync {
    async fn save_secret(&self, request: AddSecretRequest) -> Result<Secret, AddSecretError>;
    async fn delete_secret(&self, request: DeleteSecretRequest) -> Result<(), DeleteSecretError>;
    async fn get_secret_by_id(&self, id: uuid::Uuid) -> Result<Secret, GetSecretError>;
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
            "INSERT INTO secrets (id, path, cron, aws_role_arn, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(id)
        .bind(request.path)
        .bind(request.cron_expression)
        .bind(request.aws_role_arn)
        .bind(created_at)
        .bind(None::<chrono::NaiveDateTime>)
        .fetch_one(&self.pool)
        .await
        .map_err(AddSecretError::from)
        .map(Secret::from)
    }

    async fn delete_secret(&self, request: DeleteSecretRequest) -> Result<(), DeleteSecretError> {
        sqlx::query("DELETE FROM secrets WHERE id = $1")
            .bind(request.id)
            .execute(&self.pool)
            .await
            .map_err(DeleteSecretError::from)
            .and_then(|result| {
                if result.rows_affected() == 0 {
                    Err(DeleteSecretError::NotFound)
                } else {
                    Ok(())
                }
            })
    }

    async fn get_secret_by_id(&self, id: uuid::Uuid) -> Result<Secret, GetSecretError> {
        sqlx::query_as::<_, SecretModel>(
            "SELECT id, path, aws_role_arn, created_at, updated_at FROM secrets WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(GetSecretError::from)
        .and_then(|opt| opt.map(Secret::from).ok_or(GetSecretError::NotFound))
    }
}
