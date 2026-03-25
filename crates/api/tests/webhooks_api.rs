mod common;

use common::TestApp;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn register_webhook_returns_201_with_uuid() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();

    let response = app
        .http
        .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .json(&json!({
            "url": "https://example.com/webhook"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json().await.unwrap();
    let id_str = body["id"]
        .as_str()
        .expect("response should contain an 'id' string");
    Uuid::parse_str(id_str).expect("'id' should be a valid UUID");
}

#[tokio::test]
async fn register_webhook_forwards_request_to_notificator() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();
    let url = "https://example.com/my-hook";

    let response = app
        .http
        .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .json(&json!({ "url": url }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);

    let calls = app.notificator.received_requests();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].secret_id, secret_id.to_string());
    assert_eq!(calls[0].url, url);
}

#[tokio::test]
async fn register_webhook_with_invalid_url_returns_400() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();

    let response = app
        .http
        .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .json(&json!({ "url": "not-a-url" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn register_webhook_with_empty_url_returns_400() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();

    let response = app
        .http
        .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .json(&json!({ "url": "" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn register_webhook_with_missing_url_returns_422() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();

    let response = app
        .http
        .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 422);
}
