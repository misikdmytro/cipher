use std::sync::Arc;

use lapin::Channel;

use crate::config::AppConfig;
use crate::services::publisher::RotationEventPublisher;
use crate::services::rotation::RotationService;

pub struct AppState {
    pub config: AppConfig,
    pub rotation_service: Box<dyn RotationService>,
    pub publisher: Arc<dyn RotationEventPublisher>,
    pub amqp_channel: Arc<Channel>,
}
