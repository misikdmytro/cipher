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

impl DeliveryError {
    /// Returns true for errors that may succeed on retry.
    pub fn is_retriable(&self) -> bool {
        match self {
            DeliveryError::HttpError(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            DeliveryError::StatusError(status) => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
        }
    }
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
                Err(e) if e.is_retriable() => {
                    error!(
                        attempt = attempt + 1,
                        max_attempts,
                        url,
                        error = ?e,
                        "Retriable webhook delivery attempt failed"
                    );
                    last_error = Some(e);
                }
                Err(e) => {
                    error!(
                        url,
                        error = ?e,
                        "Non-retriable webhook delivery error, not retrying"
                    );
                    return Err(e);
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
