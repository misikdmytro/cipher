mod common;

use common::TestApp;
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn create_secret_returns_201_with_uuid() {
    let app = TestApp::spawn().await;

    let response = app
        .post_secret(json!({
            "path": "prod/payments/stripe_api_key",
            "cron_expression": "0 0 0 * * * *",
        }))
        .await;

    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json().await.unwrap();
    let id_str = body["id"]
        .as_str()
        .expect("response should contain an 'id' string");
    Uuid::parse_str(id_str).expect("'id' should be a valid UUID");
}

#[tokio::test]
async fn create_secret_persists_to_database() {
    let app = TestApp::spawn().await;
    let path = format!("test/db-persist/{}", Uuid::new_v4());

    let response = app
        .post_secret(json!({
            "path": path,
            "cron_expression": "0 0 0 * * * *",
        }))
        .await;

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    let id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    assert!(
        app.secret_exists(id).await,
        "secret should be persisted in the database"
    );
}

#[tokio::test]
async fn create_secret_forwards_schedule_request_to_scheduler() {
    let app = TestApp::spawn().await;
    let cron = "0 30 9 * * * *";

    let response = app
        .post_secret(json!({
            "path": format!("test/scheduler-call/{}", Uuid::new_v4()),
            "cron_expression": cron,
        }))
        .await;

    assert_eq!(response.status(), 201);

    let calls = app.scheduler.received_requests();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].cron_expression, cron);
}

#[rstest]
#[case::empty_path(
    json!({"path": "", "cron_expression": "0 0 0 * * * *"}),
    None,
)]
#[case::path_too_long(
    json!({"path": "a".repeat(256), "cron_expression": "0 0 0 * * * *"}),
    None,
)]
#[case::invalid_cron(
    json!({"path": "some/valid/path", "cron_expression": "not-a-cron"}),
    Some("cron"),
)]
#[tokio::test]
async fn create_secret_with_invalid_body_returns_400(
    #[case] body: serde_json::Value,
    #[case] expected_message_fragment: Option<&str>,
) {
    let app = TestApp::spawn().await;

    let response = app.post_secret(body).await;

    assert_eq!(response.status(), 400);

    if let Some(fragment) = expected_message_fragment {
        let body: serde_json::Value = response.json().await.unwrap();
        let message = body["message"].as_str().unwrap_or("");
        assert!(
            message.contains(fragment),
            "error message should contain '{fragment}', got: {message}"
        );
    }
}

#[rstest]
#[case::missing_field(r#"{"path": "some/valid/path"}"#)]
#[case::malformed_json(r#"{ bad json }"#)]
#[tokio::test]
async fn create_secret_with_unparseable_body_returns_422(#[case] body: &str) {
    let app = TestApp::spawn().await;

    let response = app
        .http
        .post(format!("{}/secrets", app.base_url))
        .header("Content-Type", "application/json")
        .body(body.to_owned())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 422);
}

#[tokio::test]
async fn create_secret_rolls_back_on_scheduler_failure() {
    let app = TestApp::spawn_with_failing_scheduler().await;
    let path = format!("test/rollback/{}", Uuid::new_v4());

    let response = app
        .post_secret(json!({
            "path": path,
            "cron_expression": "0 0 0 * * * *",
        }))
        .await;

    assert_eq!(response.status(), 500);
    assert!(
        !app.secret_exists_by_path(&path).await,
        "secret should have been rolled back after scheduler failure"
    );
}
