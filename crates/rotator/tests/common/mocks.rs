use std::sync::{Arc, Mutex};

use common::amqp::PublishError;
use common::rotation_publisher::RotationEventPublisher;
use rotator::helpers::aws::{AwsSecretError, AwsSecretsClient, AwsSecretsClientFactory};
use rotator::helpers::generator::SecretGenerator;

use interfaces::events::rotation::RotationDoneDetails;
use proto::api::{
    AwsConfig, GetSecretRequest, GetSecretResponse, SingleStrategyConfig,
    api_service_server::ApiService,
    get_secret_response::{Provider, Strategy},
};
use uuid::Uuid;

pub struct MockApiService {
    responses: Mutex<Vec<Result<GetSecretResponse, tonic::Status>>>,
    received: Mutex<Vec<GetSecretRequest>>,
}

impl MockApiService {
    pub fn new(responses: Vec<Result<GetSecretResponse, tonic::Status>>) -> Self {
        Self {
            responses: Mutex::new(responses),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn ok(secret_id: &str, path: &str) -> Self {
        Self::new(vec![Ok(GetSecretResponse {
            secret_id: secret_id.to_string(),
            provider: None,
            strategy: Some(Strategy::Single(SingleStrategyConfig {
                path: path.to_string(),
            })),
        })])
    }

    pub fn ok_with_aws(secret_id: &str, path: &str, role_arn: &str) -> Self {
        Self::new(vec![Ok(GetSecretResponse {
            secret_id: secret_id.to_string(),
            provider: Some(Provider::Aws(AwsConfig {
                role_arn: role_arn.to_string(),
            })),
            strategy: Some(Strategy::Single(SingleStrategyConfig {
                path: path.to_string(),
            })),
        })])
    }

    pub fn not_found() -> Self {
        Self::new(vec![Err(tonic::Status::not_found("secret not found"))])
    }

    pub fn internal_error() -> Self {
        Self::new(vec![Err(tonic::Status::internal("internal error"))])
    }

    pub fn received_requests(&self) -> Vec<GetSecretRequest> {
        self.received.lock().unwrap().clone()
    }
}

#[tonic::async_trait]
impl ApiService for MockApiService {
    async fn get_secret(
        &self,
        request: tonic::Request<GetSecretRequest>,
    ) -> Result<tonic::Response<GetSecretResponse>, tonic::Status> {
        let req = request.into_inner();
        self.received.lock().unwrap().push(req);

        let response = self
            .responses
            .lock()
            .unwrap()
            .pop()
            .expect("MockApiService: no more responses configured");

        response.map(tonic::Response::new)
    }
}

#[derive(Clone)]
pub struct MockAwsSecretsClient {
    inner: Arc<MockAwsSecretsClientInner>,
}

struct MockAwsSecretsClientInner {
    put_result: Mutex<Option<Result<(), AwsSecretError>>>,
    create_result: Mutex<Option<Result<(), AwsSecretError>>>,
    put_calls: Mutex<Vec<(String, String)>>,
    create_calls: Mutex<Vec<(String, String)>>,
}

impl MockAwsSecretsClient {
    fn new(
        put_result: Result<(), AwsSecretError>,
        create_result: Result<(), AwsSecretError>,
    ) -> Self {
        Self {
            inner: Arc::new(MockAwsSecretsClientInner {
                put_result: Mutex::new(Some(put_result)),
                create_result: Mutex::new(Some(create_result)),
                put_calls: Mutex::new(Vec::new()),
                create_calls: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn success() -> Self {
        Self::new(Ok(()), Ok(()))
    }

    pub fn put_not_found_then_create_ok() -> Self {
        Self::new(Err(AwsSecretError::NotFound), Ok(()))
    }

    pub fn put_not_found_then_create_fails() -> Self {
        Self::new(
            Err(AwsSecretError::NotFound),
            Err(AwsSecretError::Other("access denied".into())),
        )
    }

    pub fn put_fails() -> Self {
        Self::new(Err(AwsSecretError::Other("access denied".into())), Ok(()))
    }

    pub fn put_calls(&self) -> Vec<(String, String)> {
        self.inner.put_calls.lock().unwrap().clone()
    }

    pub fn create_calls(&self) -> Vec<(String, String)> {
        self.inner.create_calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl AwsSecretsClient for MockAwsSecretsClient {
    async fn put_secret(&self, path: &str, value: &str) -> Result<(), AwsSecretError> {
        self.inner
            .put_calls
            .lock()
            .unwrap()
            .push((path.to_string(), value.to_string()));

        self.inner
            .put_result
            .lock()
            .unwrap()
            .take()
            .expect("MockAwsSecretsClient: put_result not configured")
    }

    async fn create_secret(&self, path: &str, value: &str) -> Result<(), AwsSecretError> {
        self.inner
            .create_calls
            .lock()
            .unwrap()
            .push((path.to_string(), value.to_string()));

        self.inner
            .create_result
            .lock()
            .unwrap()
            .take()
            .expect("MockAwsSecretsClient: create_result not configured")
    }
}

pub struct MockSecretGenerator {
    value: String,
}

impl MockSecretGenerator {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl SecretGenerator for MockSecretGenerator {
    fn generate(&self) -> String {
        self.value.clone()
    }
}

#[derive(Clone)]
pub struct MockRotationEventPublisher {
    inner: Arc<MockRotationEventPublisherInner>,
}

struct MockRotationEventPublisherInner {
    started_calls: Mutex<Vec<Uuid>>,
    done_calls: Mutex<Vec<Uuid>>,
    failed_calls: Mutex<Vec<(Uuid, String)>>,
    started_should_fail: bool,
}

impl MockRotationEventPublisher {
    pub fn success() -> Self {
        Self {
            inner: Arc::new(MockRotationEventPublisherInner {
                started_calls: Mutex::new(Vec::new()),
                done_calls: Mutex::new(Vec::new()),
                failed_calls: Mutex::new(Vec::new()),
                started_should_fail: false,
            }),
        }
    }

    pub fn started_fails() -> Self {
        Self {
            inner: Arc::new(MockRotationEventPublisherInner {
                started_calls: Mutex::new(Vec::new()),
                done_calls: Mutex::new(Vec::new()),
                failed_calls: Mutex::new(Vec::new()),
                started_should_fail: true,
            }),
        }
    }

    pub fn started_calls(&self) -> Vec<Uuid> {
        self.inner.started_calls.lock().unwrap().clone()
    }

    pub fn done_calls(&self) -> Vec<Uuid> {
        self.inner.done_calls.lock().unwrap().clone()
    }

    pub fn failed_calls(&self) -> Vec<(Uuid, String)> {
        self.inner.failed_calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl RotationEventPublisher for MockRotationEventPublisher {
    async fn publish_scheduled(&self, _secret_id: Uuid) -> Result<(), PublishError> {
        Ok(())
    }

    async fn publish_started(&self, secret_id: Uuid) -> Result<(), PublishError> {
        self.inner.started_calls.lock().unwrap().push(secret_id);

        if self.inner.started_should_fail {
            return Err(PublishError::Serialization(
                serde_json::from_str::<()>("invalid").unwrap_err(),
            ));
        }
        Ok(())
    }

    async fn publish_done(
        &self,
        secret_id: Uuid,
        _details: RotationDoneDetails,
    ) -> Result<(), PublishError> {
        self.inner.done_calls.lock().unwrap().push(secret_id);
        Ok(())
    }

    async fn publish_ready(
        &self,
        _secret_id: Uuid,
        _active_slot: String,
        _active_path: String,
        _ready_slot: String,
        _ready_path: String,
    ) -> Result<(), PublishError> {
        Ok(())
    }

    async fn publish_failed(&self, secret_id: Uuid, error: String) -> Result<(), PublishError> {
        self.inner
            .failed_calls
            .lock()
            .unwrap()
            .push((secret_id, error));
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockAwsSecretsClientFactory {
    client: MockAwsSecretsClient,
}

impl MockAwsSecretsClientFactory {
    pub fn new(client: MockAwsSecretsClient) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl AwsSecretsClientFactory for MockAwsSecretsClientFactory {
    async fn create(
        &self,
        _role_arn: Option<&str>,
    ) -> Result<Box<dyn AwsSecretsClient>, AwsSecretError> {
        Ok(Box::new(self.client.clone()))
    }
}

pub struct FailingAwsSecretsClientFactory;

#[async_trait::async_trait]
impl AwsSecretsClientFactory for FailingAwsSecretsClientFactory {
    async fn create(
        &self,
        _role_arn: Option<&str>,
    ) -> Result<Box<dyn AwsSecretsClient>, AwsSecretError> {
        Err(AwsSecretError::Other("STS AssumeRole failed".into()))
    }
}

#[derive(Clone)]
pub struct RecordingAwsSecretsClientFactory {
    inner: Arc<RecordingAwsSecretsClientFactoryInner>,
}

struct RecordingAwsSecretsClientFactoryInner {
    client: MockAwsSecretsClient,
    received_role_arns: Mutex<Vec<Option<String>>>,
}

impl RecordingAwsSecretsClientFactory {
    pub fn new(client: MockAwsSecretsClient) -> Self {
        Self {
            inner: Arc::new(RecordingAwsSecretsClientFactoryInner {
                client,
                received_role_arns: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn received_role_arns(&self) -> Vec<Option<String>> {
        self.inner.received_role_arns.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl AwsSecretsClientFactory for RecordingAwsSecretsClientFactory {
    async fn create(
        &self,
        role_arn: Option<&str>,
    ) -> Result<Box<dyn AwsSecretsClient>, AwsSecretError> {
        self.inner
            .received_role_arns
            .lock()
            .unwrap()
            .push(role_arn.map(|s| s.to_string()));
        Ok(Box::new(self.inner.client.clone()))
    }
}
