use std::sync::Arc;

use interfaces::events::rotation::{
    RotationDone, RotationDoneDetails, RotationFailed, RotationReady, RotationScheduled,
    RotationStarted,
};
use lapin::Channel;
use uuid::Uuid;

use crate::amqp::PublishError;

#[async_trait::async_trait]
pub trait RotationEventPublisher: Send + Sync {
    async fn publish_scheduled(&self, secret_id: Uuid) -> Result<(), PublishError>;
    async fn publish_started(&self, secret_id: Uuid) -> Result<(), PublishError>;
    async fn publish_done(
        &self,
        secret_id: Uuid,
        details: RotationDoneDetails,
    ) -> Result<(), PublishError>;
    async fn publish_ready(
        &self,
        secret_id: Uuid,
        active_slot: String,
        active_path: String,
        ready_slot: String,
        ready_path: String,
    ) -> Result<(), PublishError>;
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
    async fn publish_scheduled(&self, secret_id: Uuid) -> Result<(), PublishError> {
        let event = RotationScheduled { secret_id };
        crate::amqp::publish_event(&self.channel, "rotation.scheduled", &event).await
    }

    async fn publish_started(&self, secret_id: Uuid) -> Result<(), PublishError> {
        let event = RotationStarted { secret_id };
        crate::amqp::publish_event(&self.channel, "rotation.started", &event).await
    }

    async fn publish_done(
        &self,
        secret_id: Uuid,
        details: RotationDoneDetails,
    ) -> Result<(), PublishError> {
        let event = RotationDone { secret_id, details };
        crate::amqp::publish_event(&self.channel, "rotation.done", &event).await
    }

    async fn publish_ready(
        &self,
        secret_id: Uuid,
        active_slot: String,
        active_path: String,
        ready_slot: String,
        ready_path: String,
    ) -> Result<(), PublishError> {
        let event = RotationReady {
            secret_id,
            active_slot,
            active_path,
            ready_slot,
            ready_path,
        };
        crate::amqp::publish_event(&self.channel, "rotation.ready", &event).await
    }

    async fn publish_failed(&self, secret_id: Uuid, error: String) -> Result<(), PublishError> {
        let event = RotationFailed { secret_id, error };
        crate::amqp::publish_event(&self.channel, "rotation.failed", &event).await
    }
}
