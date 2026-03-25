use std::sync::Arc;

pub use common::amqp::PublishError;
use interfaces::events::rotation::{RotationDone, RotationFailed, RotationStarted};
use lapin::Channel;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait RotationEventPublisher: Send + Sync {
    async fn publish_started(&self, secret_id: Uuid) -> Result<(), PublishError>;
    async fn publish_done(&self, secret_id: Uuid) -> Result<(), PublishError>;
    async fn publish_failed(&self, secret_id: Uuid, error: String) -> Result<(), PublishError>;
}

struct AmqpRotationEventPublisher {
    channel: Arc<Channel>,
}

pub fn new_rotation_event_publisher(
    channel: Arc<Channel>,
) -> impl RotationEventPublisher + 'static {
    AmqpRotationEventPublisher { channel }
}

#[async_trait::async_trait]
impl RotationEventPublisher for AmqpRotationEventPublisher {
    async fn publish_started(&self, secret_id: Uuid) -> Result<(), PublishError> {
        let event = RotationStarted { secret_id };
        common::amqp::publish_event(&self.channel, "rotation.started", &event).await
    }

    async fn publish_done(&self, secret_id: Uuid) -> Result<(), PublishError> {
        let event = RotationDone { secret_id };
        common::amqp::publish_event(&self.channel, "rotation.done", &event).await
    }

    async fn publish_failed(&self, secret_id: Uuid, error: String) -> Result<(), PublishError> {
        let event = RotationFailed { secret_id, error };
        common::amqp::publish_event(&self.channel, "rotation.failed", &event).await
    }
}
