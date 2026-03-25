use crate::{
    config::AppConfig,
    services::{secrets::SecretsService, webhooks::WebhooksService},
};

pub struct AppState {
    pub config: AppConfig,
    pub secrets_service: Box<dyn SecretsService>,
    pub webhooks_service: Box<dyn WebhooksService>,
}
