use std::sync::Arc;

use anyhow::Result;
use interfaces::events::rotation::{RotationDone, RotationFailed, RotationStarted};
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
};
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::repositories::webhooks::WebhooksRepository;
use crate::services::types::WebhookPayload;
use crate::services::webhook_delivery::WebhookDeliveryService;
use crate::state::AppState;

pub async fn serve(state: Arc<AppState>) -> Result<()> {
    let channel = &state.amqp_channel;

    channel
        .queue_declare(
            "notificator.rotation.events".into(),
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    for routing_key in &["rotation.started", "rotation.done", "rotation.failed"] {
        channel
            .queue_bind(
                "notificator.rotation.events".into(),
                "rotation".into(),
                (*routing_key).into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
    }

    let mut consumer = channel
        .basic_consume(
            "notificator.rotation.events".into(),
            "notificator".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("Notificator consumer started, waiting for messages");

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => handle_delivery(&state, &delivery).await,
            Err(e) => error!(error = ?e, "Error receiving delivery"),
        }
    }

    Ok(())
}

async fn handle_delivery(state: &AppState, delivery: &lapin::message::Delivery) {
    let routing_key = delivery.routing_key.as_str();

    let event = match parse_event(routing_key, &delivery.data) {
        Ok(event) => event,
        Err(e) => {
            error!(error = ?e, routing_key, "Invalid event message");
            nack(delivery).await;
            return;
        }
    };

    info!(
        secret_id = %event.secret_id,
        event_type = %event.event_type,
        "Processing rotation event"
    );

    if let Err(e) = process_event(
        &*state.webhooks_repository,
        &*state.delivery_service,
        &event,
    )
    .await
    {
        error!(error = ?e, "Failed to process rotation event");
    }

    ack(delivery).await;
}

pub async fn process_event(
    repository: &dyn WebhooksRepository,
    delivery_service: &dyn WebhookDeliveryService,
    payload: &WebhookPayload,
) -> Result<(), ProcessError> {
    let webhooks = repository
        .get_webhooks_by_secret_id(payload.secret_id)
        .await
        .map_err(ProcessError::Repository)?;

    if webhooks.is_empty() {
        info!(secret_id = %payload.secret_id, "No webhooks registered for secret");
        return Ok(());
    }

    for webhook in &webhooks {
        if let Err(e) = delivery_service.deliver(&webhook.url, payload).await {
            error!(
                webhook_id = %webhook.id,
                url = %webhook.url,
                error = ?e,
                "Webhook delivery failed after retries"
            );
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("repository error: {0}")]
    Repository(#[source] crate::repositories::webhooks::GetWebhooksError),
}

fn parse_event(routing_key: &str, data: &[u8]) -> Result<WebhookPayload> {
    match routing_key {
        "rotation.started" => {
            let event: RotationStarted = serde_json::from_slice(data)?;
            Ok(WebhookPayload {
                event_type: "rotation.started".to_string(),
                secret_id: event.secret_id,
                error: None,
            })
        }
        "rotation.done" => {
            let event: RotationDone = serde_json::from_slice(data)?;
            Ok(WebhookPayload {
                event_type: "rotation.done".to_string(),
                secret_id: event.secret_id,
                error: None,
            })
        }
        "rotation.failed" => {
            let event: RotationFailed = serde_json::from_slice(data)?;
            Ok(WebhookPayload {
                event_type: "rotation.failed".to_string(),
                secret_id: event.secret_id,
                error: Some(event.error),
            })
        }
        _ => anyhow::bail!("unknown routing key: {}", routing_key),
    }
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
