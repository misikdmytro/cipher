use crate::services::secrets::SecretsService;

pub struct AppState {
    pub secrets_service: Box<dyn SecretsService + Send + Sync>,
}
