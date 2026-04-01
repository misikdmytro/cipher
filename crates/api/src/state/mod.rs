use crate::{
    config::AppConfig,
    services::{
        rotation::RotationTriggerService, secrets::SecretsService, webhooks::WebhooksService,
    },
};

pub struct AppState {
    pub config: AppConfig,
    pub secrets_service: Box<dyn SecretsService>,
    pub webhooks_service: Box<dyn WebhooksService>,
    pub rotation_service: Box<dyn RotationTriggerService>,
}
