use std::sync::Arc;

use anyhow::Result;
use interfaces::events::rotation::RotationScheduled;
use lapin::{
    Connection, ConnectionProperties, ExchangeKind,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::FieldTable,
};
use tokio_stream::StreamExt;
use tracing::{error, info};
use uuid::Uuid;

use crate::services::types::ServiceError;
use crate::state::AppState;

pub async fn serve(state: Arc<AppState>) -> Result<()> {
    let conn = Connection::connect(
        &state.config.rabbitmq.connection_string(),
        ConnectionProperties::default(),
    )
    .await?;
    let channel = conn.create_channel().await?;

    channel
        .exchange_declare(
            "rotation".into(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

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

    match state.rotation_service.rotate(secret_id).await {
        Ok(()) => {
            if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                error!(error = ?e, "Failed to ack delivery");
            }
        }
        Err(ServiceError::NotFound) => {
            error!(secret_id = %secret_id, "Secret not found, discarding message");
            if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                error!(error = ?e, "Failed to ack delivery");
            }
        }
        Err(e) => {
            error!(error = ?e, "Rotation failed");
            nack(delivery).await;
        }
    }
}

fn parse_message(delivery: &lapin::message::Delivery) -> Result<Uuid> {
    let event: RotationScheduled = serde_json::from_slice(&delivery.data)?;
    Ok(event.secret_id)
}

async fn nack(delivery: &lapin::message::Delivery) {
    if let Err(e) = delivery.nack(BasicNackOptions::default()).await {
        error!(error = ?e, "Failed to nack delivery");
    }
}
