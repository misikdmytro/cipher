use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};
use interfaces::secrets::Secret;
use thiserror::Error;

use crate::{
    config::AppConfig,
    repositories::models::{schema::secrets as secrets_schema, secrets::Secret as SecretModel},
};

#[derive(Debug, Clone)]
pub struct AddSecretRequest {
    pub path: String,
}

impl From<AddSecretRequest> for SecretModel {
    fn from(request: AddSecretRequest) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            path: request.path,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: None,
        }
    }
}

#[derive(Debug, Error)]
#[error("failed to open database connection: {0}")]
pub struct DbConnectionError(anyhow::Error);

#[derive(Debug, Error)]
pub enum AddSecretError {
    #[error("infrastructure error: {0}")]
    Infrastructure(anyhow::Error),
}

pub trait SecretsRepository: Send + Sync {
    async fn save_secret(&self, request: AddSecretRequest) -> Result<Secret, AddSecretError>;
}

struct SecretsRepositoryImpl {
    pool: Pool<ConnectionManager<PgConnection>>,
}

pub fn new_secrets_repository(
    config: &AppConfig,
) -> Result<impl SecretsRepository, DbConnectionError> {
    SecretsRepositoryImpl::new(config)
}

impl SecretsRepositoryImpl {
    pub fn new(config: &AppConfig) -> Result<Self, DbConnectionError> {
        let manager = ConnectionManager::<PgConnection>::new(config.database.to_string());
        Pool::builder()
            .build(manager)
            .map_err(|e| DbConnectionError(e.into()))
            .map(|pool| Self { pool })
    }
}

impl SecretsRepository for SecretsRepositoryImpl {
    async fn save_secret(&self, request: AddSecretRequest) -> Result<Secret, AddSecretError> {
        diesel::insert_into(secrets_schema::table)
            .values(SecretModel::from(request))
            .returning(SecretModel::as_returning())
            .get_result(
                &mut self
                    .pool
                    .get()
                    .map_err(|e| AddSecretError::Infrastructure(e.into()))?,
            )
            .map_err(|e| AddSecretError::Infrastructure(e.into()))
            .map(Secret::from)
    }
}
