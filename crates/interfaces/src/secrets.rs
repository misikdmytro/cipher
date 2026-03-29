use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderConfig {
    Aws(AwsProviderConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsProviderConfig {
    pub role_arn: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub id: Uuid,
    pub path: String,
    pub provider: Option<ProviderConfig>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}
