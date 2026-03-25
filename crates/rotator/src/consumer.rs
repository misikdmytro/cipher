use std::sync::Arc;

use anyhow::Result;
use interfaces::events::rotation::RotationScheduled;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
};
use tokio_stream::StreamExt;
use tracing::{error, info};
use uuid::Uuid;

use crate::services::publisher::RotationEventPublisher;
use crate::services::rotation::RotationService;
use crate::services::types::ServiceError;
use crate::state::AppState;

pub async fn serve(state: Arc<AppState>) -> Result<()> {
    let channel = &state.amqp_channel;

    channel
        .queue_declare(
            "rotator.rotation.scheduled".into(),
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            "rotator.rotation.scheduled".into(),
            "rotation".into(),
            "rotation.scheduled".into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            "rotator.rotation.scheduled".into(),
            "rotator".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("Rotator consumer started, waiting for messages");

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => handle_delivery(&state, &delivery).await,
            Err(e) => error!(error = ?e, "Error receiving delivery"),
        }
    }

    Ok(())
}

async fn handle_delivery(state: &AppState, delivery: &lapin::message::Delivery) {
    let secret_id = match parse_message(delivery) {
        Ok(id) => id,
        Err(e) => {
            error!(error = ?e, "Invalid rotation message");
            nack(delivery).await;
            return;
        }
    };

    info!(secret_id = %secret_id, "Processing rotation message");

    if let Err(e) = state.publisher.publish_started(secret_id).await {
        error!(error = ?e, "Failed to publish rotation.started");
        nack(delivery).await;
        return;
    }

    match state.rotation_service.rotate(secret_id).await {
        Ok(()) => {
            if let Err(e) = state.publisher.publish_done(secret_id).await {
                error!(error = ?e, "Failed to publish rotation.done");
            }
            ack(delivery).await;
        }
        Err(ServiceError::NotFound) => {
            error!(secret_id = %secret_id, "Secret not found, discarding message");
            if let Err(e) = state.publisher.publish_done(secret_id).await {
                error!(error = ?e, "Failed to publish rotation.done");
            }
            ack(delivery).await;
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
            nack(delivery).await;
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
        Ok(()) => {
            if let Err(e) = publisher.publish_done(secret_id).await {
                error!(error = ?e, "Failed to publish rotation.done");
            }
            Ok(())
        }
        Err(ServiceError::NotFound) => {
            if let Err(e) = publisher.publish_done(secret_id).await {
                error!(error = ?e, "Failed to publish rotation.done");
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
    Publish(#[source] crate::services::publisher::PublishError),

    #[error("secret not found")]
    NotFound,

    #[error("rotation error: {0}")]
    Rotation(#[source] ServiceError),
}

fn parse_message(delivery: &lapin::message::Delivery) -> Result<Uuid> {
    let event: RotationScheduled = serde_json::from_slice(&delivery.data)?;
    Ok(event.secret_id)
}

async fn ack(delivery: &lapin::message::Delivery) {
    if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
        error!(error = ?e, "Failed to ack delivery");
    }
}

async fn nack(delivery: &lapin::message::Delivery) {
    if let Err(e) = delivery.nack(BasicNackOptions::default()).await {
        error!(error = ?e, "Failed to nack delivery");
    }
}
