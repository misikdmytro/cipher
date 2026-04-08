use common::db::DbConnectionError;
use interfaces::secrets::{ActiveSlot, ProviderConfig, Secret, StrategyConfig};
use sqlx::PgPool;

use crate::{
    config::AppConfig, repositories::models::secrets::SecretRow,
    repositories::models::secrets::Slot,
};

#[derive(Debug, Clone)]
pub struct AddSecretRequest {
    pub cron_expression: String,
    pub strategy: StrategyConfig,
    pub provider: ProviderConfig,
}

#[derive(Debug, Clone)]
pub struct DeleteSecretRequest {
    pub id: uuid::Uuid,
}

common::repo_error!(AddSecretError);
common::repo_error!(DeleteSecretError, NotFound);
common::repo_error!(GetSecretError, NotFound);
common::repo_error!(ListSecretsError);
common::repo_error!(FlipActiveSlotError, NotFound);

#[derive(Debug, Clone)]
pub struct ListSecretsRequest {
    pub limit: i64,
    pub offset: i64,
}

pub struct SecretsPage {
    pub items: Vec<Secret>,
    pub total: i64,
}

#[async_trait::async_trait]
pub trait SecretsRepository: Send + Sync {
    async fn save_secret(&self, request: AddSecretRequest) -> Result<Secret, AddSecretError>;
    async fn delete_secret(&self, request: DeleteSecretRequest) -> Result<(), DeleteSecretError>;
    async fn get_secret_by_id(&self, id: uuid::Uuid) -> Result<Secret, GetSecretError>;
    async fn list_secrets(
        &self,
        request: ListSecretsRequest,
    ) -> Result<SecretsPage, ListSecretsError>;
    async fn flip_active_slot(&self, id: uuid::Uuid) -> Result<ActiveSlot, FlipActiveSlotError>;
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

const SECRET_SELECT: &str = "
    SELECT
        s.id,
        s.cron,
        s.strategy_type,
        s.provider_type,
        s.aws_role_arn,
        s.created_at,
        s.updated_at,
        ss.path        AS single_path,
        bg.blue_path   AS bg_blue_path,
        bg.green_path  AS bg_green_path,
        bg.active_slot AS bg_active_slot
    FROM secrets s
    LEFT JOIN strategy_single     ss ON ss.secret_id = s.id
    LEFT JOIN strategy_blue_green bg ON bg.secret_id = s.id
";

#[async_trait::async_trait]
impl SecretsRepository for SecretsRepositoryImpl {
    async fn save_secret(&self, request: AddSecretRequest) -> Result<Secret, AddSecretError> {
        let id = uuid::Uuid::new_v4();
        let created_at = chrono::Utc::now().naive_utc();

        let provider_type = match &request.provider {
            ProviderConfig::Aws(_) => "aws",
        };
        let aws_role_arn = match &request.provider {
            ProviderConfig::Aws(aws) => aws.role_arn.clone(),
        };
        let strategy_type = match &request.strategy {
            StrategyConfig::Single(_) => "single",
            StrategyConfig::BlueGreen(_) => "blue_green",
        };

        let mut tx = self.pool.begin().await.map_err(AddSecretError::from)?;

        sqlx::query(
            "INSERT INTO secrets (id, cron, strategy_type, provider_type, aws_role_arn, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(&request.cron_expression)
        .bind(strategy_type)
        .bind(provider_type)
        .bind(&aws_role_arn)
        .bind(created_at)
        .bind(None::<chrono::NaiveDateTime>)
        .execute(&mut *tx)
        .await
        .map_err(AddSecretError::from)?;

        match &request.strategy {
            StrategyConfig::Single(s) => {
                sqlx::query("INSERT INTO strategy_single (secret_id, path) VALUES ($1, $2)")
                    .bind(id)
                    .bind(&s.path)
                    .execute(&mut *tx)
                    .await
                    .map_err(AddSecretError::from)?;
            }
            StrategyConfig::BlueGreen(bg) => {
                sqlx::query(
                    "INSERT INTO strategy_blue_green (secret_id, blue_path, green_path) VALUES ($1, $2, $3)",
                )
                .bind(id)
                .bind(&bg.blue_path)
                .bind(&bg.green_path)
                .execute(&mut *tx)
                .await
                .map_err(AddSecretError::from)?;
            }
        }

        tx.commit().await.map_err(AddSecretError::from)?;

        Ok(Secret {
            id,
            strategy: request.strategy,
            provider: request.provider,
            created_at,
            updated_at: None,
        })
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
        let query = format!("{SECRET_SELECT} WHERE s.id = $1");
        sqlx::query_as::<_, SecretRow>(&query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(GetSecretError::from)
            .and_then(|opt| opt.map(Secret::from).ok_or(GetSecretError::NotFound))
    }

    async fn list_secrets(
        &self,
        request: ListSecretsRequest,
    ) -> Result<SecretsPage, ListSecretsError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secrets")
            .fetch_one(&self.pool)
            .await
            .map_err(ListSecretsError::from)?;

        let query = format!("{SECRET_SELECT} ORDER BY s.created_at DESC LIMIT $1 OFFSET $2");
        let items = sqlx::query_as::<_, SecretRow>(&query)
            .bind(request.limit)
            .bind(request.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(ListSecretsError::from)?
            .into_iter()
            .map(Secret::from)
            .collect();

        Ok(SecretsPage { items, total })
    }

    async fn flip_active_slot(&self, id: uuid::Uuid) -> Result<ActiveSlot, FlipActiveSlotError> {
        let result = sqlx::query_scalar::<_, Slot>(
            "UPDATE strategy_blue_green
             SET active_slot = CASE active_slot
                                 WHEN 'blue' THEN 'green'::slot
                                 ELSE 'blue'::slot
                               END
             WHERE secret_id = $1
             RETURNING active_slot",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(FlipActiveSlotError::from)?;

        match result {
            None => Err(FlipActiveSlotError::NotFound),
            Some(slot) => Ok(slot.into()),
        }
    }
}
