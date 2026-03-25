use std::time::Duration;

use thiserror::Error;
use tracing::{error, info};

use crate::services::types::WebhookPayload;

#[derive(Debug, Error)]
pub enum DeliveryError {
    #[error("http request failed: {0}")]
    HttpError(#[source] reqwest::Error),

    #[error("non-success status code: {0}")]
    StatusError(reqwest::StatusCode),
}

#[async_trait::async_trait]
pub trait WebhookDeliveryService: Send + Sync {
    async fn deliver(&self, url: &str, payload: &WebhookPayload) -> Result<(), DeliveryError>;
}

struct HttpWebhookDeliveryService {
    client: reqwest::Client,
}

pub fn new_webhook_delivery_service() -> impl WebhookDeliveryService + 'static {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build HTTP client");

    HttpWebhookDeliveryService { client }
}

#[async_trait::async_trait]
impl WebhookDeliveryService for HttpWebhookDeliveryService {
    async fn deliver(&self, url: &str, payload: &WebhookPayload) -> Result<(), DeliveryError> {
        let max_attempts = 3;
        let mut last_error = None;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = Duration::from_secs(1 << attempt);
                info!(
                    attempt,
                    delay_secs = delay.as_secs(),
                    url,
                    "Retrying webhook delivery"
                );
                tokio::time::sleep(delay).await;
            }

            match self.try_deliver(url, payload).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    error!(
                        attempt = attempt + 1,
                        max_attempts,
                        url,
                        error = ?e,
                        "Webhook delivery attempt failed"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap())
    }
}

impl HttpWebhookDeliveryService {
    async fn try_deliver(&self, url: &str, payload: &WebhookPayload) -> Result<(), DeliveryError> {
        let response = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(DeliveryError::HttpError)?;

        if !response.status().is_success() {
            return Err(DeliveryError::StatusError(response.status()));
        }

        Ok(())
    }
}
