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

#[tokio::test]
async fn list_webhooks_returns_200_with_empty_list() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();

    let response = app
        .http
        .get(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items should be an array");
    assert!(items.is_empty());
    assert_eq!(body["total"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn list_webhooks_returns_registered_webhooks() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();
    let url = "https://example.com/hook1";

    // Register a webhook first
    let register_response = app
        .http
        .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .json(&json!({ "url": url }))
        .send()
        .await
        .unwrap();
    assert_eq!(register_response.status(), 201);

    // List webhooks
    let response = app
        .http
        .get(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items should be an array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["url"].as_str().unwrap(), url);
    assert_eq!(body["total"].as_u64().unwrap(), 1);
}

#[tokio::test]
async fn list_webhooks_respects_limit_and_offset() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();

    // Register two webhooks
    for i in 1..=2 {
        let response = app
            .http
            .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
            .json(&json!({ "url": format!("https://example.com/hook{}", i) }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
    }

    // List with limit=1
    let response = app
        .http
        .get(format!(
            "{}/secrets/{}/webhooks?limit=1",
            app.base_url, secret_id
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(body["total"].as_u64().unwrap(), 2);

    // List with offset=1
    let response = app
        .http
        .get(format!(
            "{}/secrets/{}/webhooks?limit=1&offset=1",
            app.base_url, secret_id
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(body["total"].as_u64().unwrap(), 2);
}

#[tokio::test]
async fn delete_webhook_returns_204() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();

    // Register a webhook first
    let register_response = app
        .http
        .post(format!("{}/secrets/{}/webhooks", app.base_url, secret_id))
        .json(&json!({ "url": "https://example.com/hook" }))
        .send()
        .await
        .unwrap();
    assert_eq!(register_response.status(), 201);

    let body: serde_json::Value = register_response.json().await.unwrap();
    let webhook_id = body["id"].as_str().unwrap();

    // Delete the webhook
    let response = app
        .http
        .delete(format!(
            "{}/secrets/{}/webhooks/{}",
            app.base_url, secret_id, webhook_id
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 204);
}

#[tokio::test]
async fn delete_webhook_not_found_returns_404() {
    let app = TestApp::spawn().await;
    let secret_id = Uuid::new_v4();
    let webhook_id = Uuid::new_v4();

    let response = app
        .http
        .delete(format!(
            "{}/secrets/{}/webhooks/{}",
            app.base_url, secret_id, webhook_id
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}
