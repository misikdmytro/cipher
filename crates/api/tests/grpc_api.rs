mod common;

use common::TestApp;
use proto::api::GetSecretRequest;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn get_secret_returns_path_for_existing_secret() {
    let app = TestApp::spawn().await;
    let path = format!("test/grpc/{}", Uuid::new_v4());

    let response = app
        .post_secret(json!({
            "path": path,
            "cron_expression": "0 0 0 * * * *",
            "aws": { "role_arn": "arn:aws:iam::123456789012:role/TestRotator" }
        }))
        .await;

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    let secret_id = body["id"].as_str().unwrap().to_string();

    let mut grpc_client = app.api_grpc_client.clone();
    let grpc_response = grpc_client
        .get_secret(GetSecretRequest {
            secret_id: secret_id.clone(),
        })
        .await
        .expect("gRPC GetSecret should succeed");

    let inner = grpc_response.into_inner();
    assert_eq!(inner.secret_id, secret_id);
    assert_eq!(inner.path, path);
}

#[tokio::test]
async fn get_secret_returns_not_found_for_nonexistent_secret() {
    let app = TestApp::spawn().await;

    let mut grpc_client = app.api_grpc_client.clone();
    let result = grpc_client
        .get_secret(GetSecretRequest {
            secret_id: Uuid::new_v4().to_string(),
        })
        .await;

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn get_secret_returns_invalid_argument_for_bad_uuid() {
    let app = TestApp::spawn().await;

    let mut grpc_client = app.api_grpc_client.clone();
    let result = grpc_client
        .get_secret(GetSecretRequest {
            secret_id: "not-a-uuid".to_string(),
        })
        .await;

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}
