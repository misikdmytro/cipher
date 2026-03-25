use std::sync::Arc;

pub use common::amqp::PublishError;
use interfaces::events::rotation::RotationScheduled;
use lapin::Channel;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait RotationPublisher: Send + Sync {
    async fn publish(&self, secret_id: Uuid) -> Result<(), PublishError>;
}

struct AmqpRotationPublisher {
    channel: Arc<Channel>,
}

pub fn new_rotation_publisher(channel: Arc<Channel>) -> impl RotationPublisher + 'static {
    AmqpRotationPublisher { channel }
}

#[async_trait::async_trait]
impl RotationPublisher for AmqpRotationPublisher {
    async fn publish(&self, secret_id: Uuid) -> Result<(), PublishError> {
        let event = RotationScheduled { secret_id };
        common::amqp::publish_event(&self.channel, "rotation.scheduled", &event).await
    }
}
