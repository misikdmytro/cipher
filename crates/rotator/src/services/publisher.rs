use std::sync::Arc;

use interfaces::events::rotation::{RotationDone, RotationFailed, RotationStarted};
use lapin::{BasicProperties, Channel, options::BasicPublishOptions};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("amqp error: {0}")]
    Amqp(#[from] lapin::Error),
}

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

impl AmqpRotationEventPublisher {
    async fn publish_event(&self, routing_key: &str, payload: &[u8]) -> Result<(), PublishError> {
        self.channel
            .basic_publish(
                "rotation".into(),
                routing_key.into(),
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl RotationEventPublisher for AmqpRotationEventPublisher {
    async fn publish_started(&self, secret_id: Uuid) -> Result<(), PublishError> {
        let event = RotationStarted { secret_id };
        let payload = serde_json::to_vec(&event)?;
        self.publish_event("rotation.started", &payload).await
    }

    async fn publish_done(&self, secret_id: Uuid) -> Result<(), PublishError> {
        let event = RotationDone { secret_id };
        let payload = serde_json::to_vec(&event)?;
        self.publish_event("rotation.done", &payload).await
    }

    async fn publish_failed(&self, secret_id: Uuid, error: String) -> Result<(), PublishError> {
        let event = RotationFailed { secret_id, error };
        let payload = serde_json::to_vec(&event)?;
        self.publish_event("rotation.failed", &payload).await
    }
}
