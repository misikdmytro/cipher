use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateSecretJob {
    pub secret_id: Uuid,
    pub cron_expression: String,
}
