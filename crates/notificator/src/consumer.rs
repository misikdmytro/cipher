use std::sync::Arc;

use anyhow::Result;
use common::consumer::{self, ConsumerConfig};
use futures::future::join_all;
use interfaces::events::rotation::{RotationDone, RotationFailed, RotationStarted};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::repositories::webhooks::WebhooksRepository;
use crate::services::types::WebhookPayload;
use crate::services::webhook_delivery::WebhookDeliveryService;
use crate::state::AppState;

pub async fn serve(state: Arc<AppState>, shutdown: CancellationToken) -> Result<()> {
    let config = ConsumerConfig {
        queue_name: "notificator.rotation.events".to_string(),
        consumer_tag: "notificator".to_string(),
        exchange: "rotation".to_string(),
        routing_keys: vec![
            "rotation.started".to_string(),
            "rotation.done".to_string(),
            "rotation.failed".to_string(),
        ],
        dead_letter_exchange: Some("rotation.dlx".to_string()),
        dead_letter_routing_key: Some("notificator.rotation.events".to_string()),
    };

    let mut amqp_consumer = consumer::setup(&state.amqp_channel, &config).await?;

    info!("Notificator consumer started, waiting for messages");

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
    let routing_key = delivery.routing_key.as_str();

    let event = match parse_event(routing_key, &delivery.data) {
        Ok(event) => event,
        Err(e) => {
            error!(error = ?e, routing_key, "Invalid event message");
            consumer::nack_reject(delivery).await;
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

    consumer::ack(delivery).await;
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

    let futures = webhooks.iter().map(|webhook| async {
        if let Err(e) = delivery_service.deliver(&webhook.url, payload).await {
            error!(
                webhook_id = %webhook.id,
                url = %webhook.url,
                error = ?e,
                "Webhook delivery failed after retries"
            );
        }
    });

    join_all(futures).await;

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
