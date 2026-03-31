use std::sync::Arc;

use anyhow::Result;
use common::consumer::{self, ConsumerConfig};
use common::rotation_publisher::RotationEventPublisher;
use interfaces::events::rotation::RotationScheduled;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use uuid::Uuid;

use crate::services::rotation::RotationService;
use crate::services::types::ServiceError;
use crate::state::AppState;

pub async fn serve(state: Arc<AppState>, shutdown: CancellationToken) -> Result<()> {
    let config = ConsumerConfig {
        queue_name: "rotator.rotation.scheduled".to_string(),
        consumer_tag: "rotator".to_string(),
        exchange: "rotation".to_string(),
        routing_keys: vec!["rotation.scheduled".to_string()],
        dead_letter_exchange: Some("rotation.dlx".to_string()),
        dead_letter_routing_key: Some("rotator.rotation.scheduled".to_string()),
    };

    let mut amqp_consumer = consumer::setup(&state.amqp_channel, &config).await?;

    info!("Rotator consumer started, waiting for messages");

    loop {
        tokio::select! {
            delivery = amqp_consumer.next() => {
                match delivery {
                    Some(Ok(delivery)) => handle_delivery(&state, &delivery).await,
                    Some(Err(e)) => error!(error = ?e, "Error receiving delivery"),
                    None => break,
                }
            }
            _ = shutdown.cancelled() => {
                info!("Shutdown signal received, stopping consumer");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_delivery(state: &AppState, delivery: &lapin::message::Delivery) {
    let secret_id = match parse_message(delivery) {
        Ok(id) => id,
        Err(e) => {
            error!(error = ?e, "Invalid rotation message");
            consumer::nack_reject(delivery).await;
            return;
        }
    };

    info!(secret_id = %secret_id, "Processing rotation message");

    if let Err(e) = state.publisher.publish_started(secret_id).await {
        error!(error = ?e, "Failed to publish rotation.started");
        consumer::nack_requeue(delivery).await;
        return;
    }

    match state.rotation_service.rotate(secret_id).await {
        Ok(()) => {
            consumer::ack(delivery).await;
        }
        Err(ServiceError::NotFound) => {
            error!(secret_id = %secret_id, "Secret not found, discarding message");
            if let Err(e) = state
                .publisher
                .publish_failed(secret_id, "secret not found".to_string())
                .await
            {
                error!(error = ?e, "Failed to publish rotation.failed");
            }
            consumer::ack(delivery).await;
        }
        Err(e) => {
            error!(error = ?e, "Rotation failed");
            if let Err(e_pub) = state
                .publisher
                .publish_failed(secret_id, e.to_string())
                .await
            {
                error!(error = ?e_pub, "Failed to publish rotation.failed");
            }
            consumer::nack_requeue(delivery).await;
        }
    }
}

pub async fn process_rotation(
    secret_id: Uuid,
    rotation_service: &dyn RotationService,
    publisher: &dyn RotationEventPublisher,
) -> Result<(), ProcessError> {
    publisher
        .publish_started(secret_id)
        .await
        .map_err(ProcessError::Publish)?;

    match rotation_service.rotate(secret_id).await {
        Ok(()) => Ok(()),
        Err(ServiceError::NotFound) => {
            if let Err(e) = publisher
                .publish_failed(secret_id, "secret not found".to_string())
                .await
            {
                error!(error = ?e, "Failed to publish rotation.failed");
            }
            Err(ProcessError::NotFound)
        }
        Err(e) => {
            if let Err(e_pub) = publisher.publish_failed(secret_id, e.to_string()).await {
                error!(error = ?e_pub, "Failed to publish rotation.failed");
            }
            Err(ProcessError::Rotation(e))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("publish error: {0}")]
    Publish(#[source] common::amqp::PublishError),

    #[error("secret not found")]
    NotFound,

    #[error("rotation error: {0}")]
    Rotation(#[source] ServiceError),
}

fn parse_message(delivery: &lapin::message::Delivery) -> Result<Uuid> {
    let event: RotationScheduled = serde_json::from_slice(&delivery.data)?;
    Ok(event.secret_id)
}
