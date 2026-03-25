use std::sync::Arc;

use lapin::Channel;

use crate::config::AppConfig;
use crate::repositories::webhooks::WebhooksRepository;
use crate::services::webhook_delivery::WebhookDeliveryService;

pub struct AppState {
    pub config: AppConfig,
    pub webhooks_repository: Box<dyn WebhooksRepository>,
    pub delivery_service: Box<dyn WebhookDeliveryService>,
    pub amqp_channel: Arc<Channel>,
}
