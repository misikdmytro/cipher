mod common;

use std::sync::Arc;

use common::mocks::{
    FailingAwsSecretsClientFactory, MockApiService, MockAwsSecretsClient,
    MockAwsSecretsClientFactory, MockRotationEventPublisher, MockSecretGenerator,
    RecordingAwsSecretsClientFactory,
};
use proto::api::{api_service_client::ApiServiceClient, api_service_server::ApiServiceServer};
use rotator::services::rotation::{RotationService, new_rotation_service};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Endpoint, Server};
use uuid::Uuid;

async fn start_mock_api(mock: Arc<MockApiService>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = format!("http://127.0.0.1:{}", addr.port());

    tokio::spawn(async move {
        Server::builder()
            .add_service(ApiServiceServer::from_arc(mock))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    endpoint
}

fn api_client(endpoint: &str) -> ApiServiceClient<Channel> {
    let channel = Endpoint::new(endpoint.to_string()).unwrap().connect_lazy();
    ApiServiceClient::new(channel)
}

#[tokio::test]
async fn rotate_puts_generated_secret_to_aws_path() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok(
        &secret_id.to_string(),
        "prod/db/password",
    ));
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::success();
    let aws_handle = aws.clone();
    let factory = MockAwsSecretsClientFactory::new(aws);
    let generator = Box::new(MockSecretGenerator::new("generated-secret-value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    svc.rotate(secret_id)
        .await
        .expect("rotation should succeed");

    let put_calls = aws_handle.put_calls();
    assert_eq!(put_calls.len(), 1);
    assert_eq!(put_calls[0].0, "prod/db/password");
    assert_eq!(put_calls[0].1, "generated-secret-value");
}

#[tokio::test]
async fn rotate_sends_correct_secret_id_to_api() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok(&secret_id.to_string(), "some/path"));
    let endpoint = start_mock_api(mock_api.clone()).await;

    let aws = MockAwsSecretsClient::success();
    let factory = MockAwsSecretsClientFactory::new(aws);
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    svc.rotate(secret_id).await.unwrap();

    let requests = mock_api.received_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].secret_id, secret_id.to_string());
}

#[tokio::test]
async fn rotate_returns_not_found_when_api_returns_not_found() {
    let mock_api = Arc::new(MockApiService::not_found());
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::success();
    let aws_handle = aws.clone();
    let factory = MockAwsSecretsClientFactory::new(aws);
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    let result = svc.rotate(Uuid::new_v4()).await;

    assert!(matches!(
        result,
        Err(rotator::services::types::ServiceError::NotFound)
    ));
    assert!(aws_handle.put_calls().is_empty());
}

#[tokio::test]
async fn rotate_returns_api_error_on_grpc_failure() {
    let mock_api = Arc::new(MockApiService::internal_error());
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::success();
    let factory = MockAwsSecretsClientFactory::new(aws);
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    let result = svc.rotate(Uuid::new_v4()).await;

    assert!(matches!(
        result,
        Err(rotator::services::types::ServiceError::ApiError(_))
    ));
}

#[tokio::test]
async fn rotate_creates_secret_when_put_returns_not_found() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok(
        &secret_id.to_string(),
        "new/secret/path",
    ));
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::put_not_found_then_create_ok();
    let aws_handle = aws.clone();
    let factory = MockAwsSecretsClientFactory::new(aws);
    let generator = Box::new(MockSecretGenerator::new("new-value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    svc.rotate(secret_id)
        .await
        .expect("rotation should succeed with create fallback");

    let create_calls = aws_handle.create_calls();
    assert_eq!(create_calls.len(), 1);
    assert_eq!(create_calls[0].0, "new/secret/path");
    assert_eq!(create_calls[0].1, "new-value");
}

#[tokio::test]
async fn rotate_returns_aws_error_when_create_fallback_fails() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok(&secret_id.to_string(), "some/path"));
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::put_not_found_then_create_fails();
    let factory = MockAwsSecretsClientFactory::new(aws);
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    let result = svc.rotate(secret_id).await;

    assert!(matches!(
        result,
        Err(rotator::services::types::ServiceError::AwsError(_))
    ));
}

#[tokio::test]
async fn rotate_returns_aws_error_when_put_fails() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok(&secret_id.to_string(), "some/path"));
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::put_fails();
    let factory = MockAwsSecretsClientFactory::new(aws);
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    let result = svc.rotate(secret_id).await;

    assert!(matches!(
        result,
        Err(rotator::services::types::ServiceError::AwsError(_))
    ));
}

#[tokio::test]
async fn rotate_with_role_arn_passes_arn_to_factory() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok_with_aws(
        &secret_id.to_string(),
        "cross-account/secret",
        "arn:aws:iam::123456789012:role/Rotator",
    ));
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::success();
    let recording = RecordingAwsSecretsClientFactory::new(aws);
    let recording_handle = recording.clone();
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(recording),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    svc.rotate(secret_id).await.unwrap();

    let arns = recording_handle.received_role_arns();
    assert_eq!(
        arns,
        vec![Some("arn:aws:iam::123456789012:role/Rotator".to_string())]
    );
}

#[tokio::test]
async fn rotate_without_role_arn_passes_none_to_factory() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok(&secret_id.to_string(), "some/path"));
    let endpoint = start_mock_api(mock_api).await;

    let aws = MockAwsSecretsClient::success();
    let recording = RecordingAwsSecretsClientFactory::new(aws);
    let recording_handle = recording.clone();
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(recording),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    svc.rotate(secret_id).await.unwrap();

    let arns = recording_handle.received_role_arns();
    assert_eq!(arns, vec![None]);
}

#[tokio::test]
async fn rotate_returns_aws_error_when_role_assumption_fails() {
    let secret_id = Uuid::new_v4();
    let mock_api = Arc::new(MockApiService::ok_with_aws(
        &secret_id.to_string(),
        "some/path",
        "arn:aws:iam::999999999999:role/BadRole",
    ));
    let endpoint = start_mock_api(mock_api).await;

    let factory = FailingAwsSecretsClientFactory;
    let generator = Box::new(MockSecretGenerator::new("value"));

    let svc = new_rotation_service(
        api_client(&endpoint),
        Box::new(factory),
        generator,
        Box::new(MockRotationEventPublisher::success()),
    );
    let result = svc.rotate(secret_id).await;

    assert!(matches!(
        result,
        Err(rotator::services::types::ServiceError::AwsError(_))
    ));
}
