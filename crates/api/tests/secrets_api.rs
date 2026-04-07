mod common;

use common::TestApp;
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

fn single_body(path: &str) -> serde_json::Value {
    json!({
        "cron_expression": "0 0 0 * * * *",
        "strategy": { "single": { "path": path } },
        "provider": { "aws": { "role_arn": "arn:aws:iam::123456789012:role/TestRotator" } }
    })
}

async fn create_secret(app: &TestApp) -> Uuid {
    let path = format!("test/helper/{}", Uuid::new_v4());
    let response = app.post_secret(single_body(&path)).await;
    assert_eq!(response.status(), 201, "helper: failed to create secret");
    let body: serde_json::Value = response.json().await.unwrap();
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn create_secret_returns_201_with_uuid() {
    let app = TestApp::spawn().await;

    let response = app
        .post_secret(single_body("prod/payments/stripe_api_key"))
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

    let response = app.post_secret(single_body(&path)).await;

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
            "cron_expression": cron,
            "strategy": { "single": { "path": format!("test/scheduler-call/{}", Uuid::new_v4()) } },
            "provider": { "aws": { "role_arn": "arn:aws:iam::123456789012:role/TestRotator" } }
        }))
        .await;

    assert_eq!(response.status(), 201);

    let calls = app.scheduler.received_requests();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].cron_expression, cron);
}

#[tokio::test]
async fn create_secret_with_invalid_cron_returns_400() {
    let app = TestApp::spawn().await;

    let response = app
        .post_secret(json!({
            "cron_expression": "not-a-cron",
            "strategy": { "single": { "path": "some/valid/path" } },
            "provider": { "aws": { "role_arn": "arn:aws:iam::123456789012:role/TestRotator" } }
        }))
        .await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    let message = body["message"].as_str().unwrap_or("");
    assert!(
        message.contains("cron"),
        "error message should contain 'cron', got: {message}"
    );
}

#[rstest]
#[case::missing_field(r#"{"cron_expression": "0 0 0 * * * *"}"#, 422)]
#[case::wrong_field_type(r#"{"cron_expression": 123}"#, 422)]
#[case::malformed_json(r#"{ bad json }"#, 400)]
#[case::empty_provider(
    r#"{"cron_expression":"0 0 0 * * * *","strategy":{"single":{"path":"a"}},"provider":{}}"#,
    400
)]
#[case::both_strategies(r#"{"cron_expression":"0 0 0 * * * *","strategy":{"single":{"path":"a"},"blue_green":{"blue_path":"a","green_path":"b"}},"provider":{"aws":{"role_arn":"arn:aws:iam::123456789012:role/R"}}}"#, 400)]
#[tokio::test]
async fn create_secret_with_malformed_body(#[case] body: &str, #[case] expected_status: u16) {
    let app = TestApp::spawn().await;

    let response = app
        .http
        .post(format!("{}/secrets", app.base_url))
        .header("Content-Type", "application/json")
        .body(body.to_owned())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), expected_status);
}

#[tokio::test]
async fn create_secret_rolls_back_on_scheduler_failure() {
    let app = TestApp::spawn_with_failing_scheduler().await;
    let path = format!("test/rollback/{}", Uuid::new_v4());

    let response = app.post_secret(single_body(&path)).await;

    assert_eq!(response.status(), 500);
    assert!(
        !app.secret_exists_by_single_path(&path).await,
        "secret should have been rolled back after scheduler failure"
    );
}

// --- GET /secrets ---

#[tokio::test]
async fn list_secrets_returns_200_with_correct_shape() {
    let app = TestApp::spawn().await;

    let response = app.get_secrets("").await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["items"].is_array());
    assert!(body["total"].is_number());
}

#[tokio::test]
async fn list_secrets_newly_created_secret_appears_in_first_page() {
    let app = TestApp::spawn().await;
    let id = create_secret(&app).await;

    let response = app.get_secrets("limit=100").await;
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    let found = items
        .iter()
        .any(|item| item["id"].as_str() == Some(&id.to_string()));
    assert!(found, "newly created secret {id} should appear in the list");
}

#[tokio::test]
async fn list_secrets_respects_limit() {
    let app = TestApp::spawn().await;
    create_secret(&app).await;

    let response = app.get_secrets("limit=1").await;
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["items"].as_array().unwrap().len() <= 1);
}

#[tokio::test]
async fn list_secrets_clamps_limit_to_100() {
    let app = TestApp::spawn().await;

    let response = app.get_secrets("limit=999").await;
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["items"].as_array().unwrap().len() <= 100);
}

#[tokio::test]
async fn list_secrets_offset_skips_first_item() {
    let app = TestApp::spawn().await;
    create_secret(&app).await;
    create_secret(&app).await;

    let body0: serde_json::Value = app
        .get_secrets("limit=1&offset=0")
        .await
        .json()
        .await
        .unwrap();
    let body1: serde_json::Value = app
        .get_secrets("limit=1&offset=1")
        .await
        .json()
        .await
        .unwrap();

    assert!(body0["total"].as_i64().unwrap() >= 2);

    let id0 = body0["items"][0]["id"].as_str().expect("item at offset=0");
    let id1 = body1["items"][0]["id"].as_str().expect("item at offset=1");
    assert_ne!(id0, id1, "offset=1 should skip the first item");
}

#[tokio::test]
async fn list_secrets_total_reflects_created_secrets() {
    let app = TestApp::spawn().await;

    let before: serde_json::Value = app.get_secrets("").await.json().await.unwrap();
    let total_before = before["total"].as_i64().unwrap();

    create_secret(&app).await;
    create_secret(&app).await;

    let after: serde_json::Value = app.get_secrets("").await.json().await.unwrap();
    let total_after = after["total"].as_i64().unwrap();

    assert!(
        total_after >= total_before + 2,
        "total should have grown by at least 2"
    );
}

// --- GET /secrets/{id} ---

#[tokio::test]
async fn get_secret_by_id_returns_200_with_expected_fields() {
    let app = TestApp::spawn().await;
    let path = format!("test/get-by-id/{}", Uuid::new_v4());

    let created: serde_json::Value = app
        .post_secret(single_body(&path))
        .await
        .json()
        .await
        .unwrap();
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();

    let response = app.get_secret_by_id(id).await;
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["id"].as_str().unwrap(), id.to_string());
    assert_eq!(body["strategy"]["single"]["path"].as_str().unwrap(), path);
    assert_eq!(
        body["provider"]["aws"]["role_arn"].as_str().unwrap(),
        "arn:aws:iam::123456789012:role/TestRotator"
    );
    assert!(
        body["created_at"].is_string(),
        "created_at should be present"
    );
}

#[tokio::test]
async fn get_secret_by_id_returns_404_for_unknown_id() {
    let app = TestApp::spawn().await;

    let response = app.get_secret_by_id(Uuid::new_v4()).await;

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn get_secret_by_id_with_invalid_uuid_returns_400() {
    let app = TestApp::spawn().await;

    let response = app
        .http
        .get(format!("{}/secrets/not-a-uuid", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}
