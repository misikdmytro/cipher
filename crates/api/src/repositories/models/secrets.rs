use interfaces::secrets::{AwsProviderConfig, ProviderConfig};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow, Debug, Clone)]
pub struct Secret {
    pub id: Uuid,
    pub path: String,
    pub aws_role_arn: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<Secret> for interfaces::secrets::Secret {
    fn from(secret: Secret) -> Self {
        let provider = secret
            .aws_role_arn
            .map(|arn| ProviderConfig::Aws(AwsProviderConfig { role_arn: arn }));

        Self {
            id: secret.id,
            path: secret.path,
            provider,
            created_at: secret.created_at,
            updated_at: secret.updated_at,
        }
    }
}
