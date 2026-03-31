use std::sync::Arc;

use common::rotation_publisher::RotationEventPublisher;
use lapin::Channel;

use crate::services::rotation::RotationService;

pub struct AppState {
    pub rotation_service: Box<dyn RotationService>,
    pub publisher: Arc<dyn RotationEventPublisher>,
    pub amqp_channel: Arc<Channel>,
}
