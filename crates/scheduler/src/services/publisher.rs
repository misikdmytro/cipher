use std::sync::Arc;

use interfaces::events::rotation::RotationScheduled;
use lapin::{BasicProperties, Channel, options::BasicPublishOptions};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait RotationPublisher: Send + Sync {
    async fn publish(&self, secret_id: Uuid) -> Result<(), PublishError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("amqp error: {0}")]
    Amqp(#[from] lapin::Error),
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
        let payload = serde_json::to_vec(&event)?;

        self.channel
            .basic_publish(
                "rotation".into(),
                "rotation.scheduled".into(),
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }
}
