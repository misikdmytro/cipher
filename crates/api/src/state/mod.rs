use crate::{config::AppConfig, services::secrets::SecretsService};

pub struct AppState {
    pub config: AppConfig,
    pub secrets_service: Box<dyn SecretsService>,
}
