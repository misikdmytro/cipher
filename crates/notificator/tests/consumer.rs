mod common;

use common::mocks::{MockWebhookDeliveryService, MockWebhooksRepository};
use notificator::consumer::process_event;
use notificator::repositories::models::webhooks::Webhook;
use notificator::services::types::WebhookPayload;
use uuid::Uuid;

fn make_webhook(secret_id: Uuid, url: &str) -> Webhook {
    Webhook {
        id: Uuid::new_v4(),
        secret_id,
        url: url.to_string(),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

#[tokio::test]
async fn delivers_to_all_matching_webhooks() {
    let secret_id = Uuid::new_v4();
    let repo = MockWebhooksRepository::with_webhooks(vec![
        make_webhook(secret_id, "https://example.com/hook1"),
        make_webhook(secret_id, "https://example.com/hook2"),
    ]);
    let delivery = MockWebhookDeliveryService::success();

    let payload = WebhookPayload {
        event_type: "rotation.done".to_string(),
        secret_id,
        error: None,
    };

    let result = process_event(&repo, &delivery, &payload).await;

    assert!(result.is_ok());
    let calls = delivery.deliver_calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "https://example.com/hook1");
    assert_eq!(calls[1].0, "https://example.com/hook2");
}

#[tokio::test]
async fn no_delivery_when_no_webhooks_registered() {
    let secret_id = Uuid::new_v4();
    let repo = MockWebhooksRepository::empty();
    let delivery = MockWebhookDeliveryService::success();

    let payload = WebhookPayload {
        event_type: "rotation.started".to_string(),
        secret_id,
        error: None,
    };

    let result = process_event(&repo, &delivery, &payload).await;

    assert!(result.is_ok());
    assert!(delivery.deliver_calls().is_empty());
}

#[tokio::test]
async fn delivery_failure_does_not_block_other_webhooks() {
    let secret_id = Uuid::new_v4();
    let repo = MockWebhooksRepository::with_webhooks(vec![
        make_webhook(secret_id, "https://example.com/hook1"),
        make_webhook(secret_id, "https://example.com/hook2"),
    ]);
    let delivery = MockWebhookDeliveryService::failing();

    let payload = WebhookPayload {
        event_type: "rotation.done".to_string(),
        secret_id,
        error: None,
    };

    let result = process_event(&repo, &delivery, &payload).await;

    assert!(result.is_ok());
    let calls = delivery.deliver_calls();
    assert_eq!(
        calls.len(),
        2,
        "should attempt delivery to all webhooks even if failing"
    );
}

#[tokio::test]
async fn only_delivers_to_webhooks_for_matching_secret() {
    let target_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let repo = MockWebhooksRepository::with_webhooks(vec![
        make_webhook(target_id, "https://example.com/target"),
        make_webhook(other_id, "https://example.com/other"),
    ]);
    let delivery = MockWebhookDeliveryService::success();

    let payload = WebhookPayload {
        event_type: "rotation.done".to_string(),
        secret_id: target_id,
        error: None,
    };

    let result = process_event(&repo, &delivery, &payload).await;

    assert!(result.is_ok());
    let calls = delivery.deliver_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "https://example.com/target");
}

#[tokio::test]
async fn payload_contains_correct_event_type_and_secret_id() {
    let secret_id = Uuid::new_v4();
    let repo = MockWebhooksRepository::with_webhooks(vec![make_webhook(
        secret_id,
        "https://example.com/hook",
    )]);
    let delivery = MockWebhookDeliveryService::success();

    let payload = WebhookPayload {
        event_type: "rotation.started".to_string(),
        secret_id,
        error: None,
    };

    process_event(&repo, &delivery, &payload).await.unwrap();

    let calls = delivery.deliver_calls();
    assert_eq!(calls[0].1.event_type, "rotation.started");
    assert_eq!(calls[0].1.secret_id, secret_id);
    assert!(calls[0].1.error.is_none());
}

#[tokio::test]
async fn failed_event_includes_error_in_payload() {
    let secret_id = Uuid::new_v4();
    let repo = MockWebhooksRepository::with_webhooks(vec![make_webhook(
        secret_id,
        "https://example.com/hook",
    )]);
    let delivery = MockWebhookDeliveryService::success();

    let payload = WebhookPayload {
        event_type: "rotation.failed".to_string(),
        secret_id,
        error: Some("access denied".to_string()),
    };

    process_event(&repo, &delivery, &payload).await.unwrap();

    let calls = delivery.deliver_calls();
    assert_eq!(calls[0].1.event_type, "rotation.failed");
    assert_eq!(calls[0].1.error.as_deref(), Some("access denied"));
}
